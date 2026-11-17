//! GitHub release metadata for the public downloads page.
//!
//! The website never guesses a download URL. It asks these endpoints for the
//! published releases and renders only the assets that actually exist.
//! Successful lookups are cached in memory, so a burst of page loads produces
//! a single GitHub API call and a GitHub outage serves the last good payload
//! instead of an error.
//!
//! Endpoints:
//!
//! - `GET /api/v1/releases/latest` — the newest stable release plus its
//!   classified assets.
//! - `GET /api/v1/releases` — the release history, newest first, so the page
//!   can offer previous versions.
//!
//! Configuration:
//!
//! - `KERNEXT_REPOSITORY` overrides the repository (`owner/name`); the default
//!   is the project's own repository.
//! - `KERNEXT_GITHUB_TOKEN` raises the API rate limit. It is optional and is
//!   never logged.
//! - `KERNEXT_RELEASES_DISABLED=1` disables network lookups entirely (used by
//!   offline development and tests).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Repository whose releases are published, unless overridden.
const DEFAULT_REPOSITORY: &str = "Trishit6/terminal";

/// How long a fetched payload stays fresh.
const DEFAULT_TTL: Duration = Duration::from_secs(600);

/// Timeout for one GitHub API call.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// GitHub requires a `User-Agent` on every request.
const USER_AGENT: &str = "kernext-server";

/// How many releases to request per page. The history endpoint is served from
/// this single page.
const PAGE_SIZE: u32 = 30;

/// How many previous releases the history payload returns.
const HISTORY_LIMIT: usize = 12;

/// One published asset, classified for the download page.
#[derive(Debug, Clone, Serialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub size: u64,
    pub download_url: String,
    /// Direct link to the matching `.sha256` asset, when the release has one.
    pub checksum_url: Option<String>,
    /// Direct link to the matching `.sig` asset, when the release has one.
    pub signature_url: Option<String>,
    pub platform: Option<String>,
    pub arch: Option<String>,
    pub format: String,
    /// `desktop`, `cli`, or `other`.
    pub kind: String,
}

/// The latest stable release, or an explanation of why none could be read.
#[derive(Debug, Clone, Serialize)]
pub struct LatestRelease {
    pub available: bool,
    pub repository: String,
    pub tag: Option<String>,
    pub version: Option<String>,
    pub name: Option<String>,
    pub published_at: Option<String>,
    pub html_url: Option<String>,
    pub notes: Option<String>,
    pub assets: Vec<ReleaseAsset>,
    /// Whether the payload came from the in-memory cache.
    pub cached: bool,
    pub fetched_at: Option<String>,
    /// Human-readable reason the payload is empty or stale.
    pub error: Option<String>,
}

impl LatestRelease {
    fn unavailable(repository: &str, reason: &str) -> Self {
        Self {
            available: false,
            repository: repository.to_owned(),
            tag: None,
            version: None,
            name: None,
            published_at: None,
            html_url: None,
            notes: None,
            assets: Vec::new(),
            cached: false,
            fetched_at: None,
            error: Some(reason.to_owned()),
        }
    }
}

/// One entry in the release history.
#[derive(Debug, Clone, Serialize)]
pub struct ReleaseSummary {
    pub tag: String,
    pub version: String,
    pub name: Option<String>,
    pub published_at: Option<String>,
    pub html_url: Option<String>,
    pub prerelease: bool,
    pub asset_count: usize,
}

/// The release history, newest first.
#[derive(Debug, Clone, Serialize)]
pub struct ReleaseList {
    pub available: bool,
    pub repository: String,
    pub releases: Vec<ReleaseSummary>,
    pub cached: bool,
    pub error: Option<String>,
}

impl ReleaseList {
    fn unavailable(repository: &str, reason: &str) -> Self {
        Self {
            available: false,
            repository: repository.to_owned(),
            releases: Vec::new(),
            cached: false,
            error: Some(reason.to_owned()),
        }
    }
}

struct Cached {
    at: Instant,
    releases: Vec<GithubRelease>,
}

/// Cached GitHub release lookups shared by the HTTP handlers.
pub struct ReleaseFeed {
    client: reqwest::Client,
    repository: String,
    ttl: Duration,
    enabled: bool,
    token: Option<String>,
    cache: Mutex<Option<Cached>>,
}

impl ReleaseFeed {
    /// Builds the feed from environment variables.
    #[must_use]
    pub fn from_env() -> Self {
        let repository = std::env::var("KERNEXT_REPOSITORY")
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| DEFAULT_REPOSITORY.to_owned());

        let token = std::env::var("KERNEXT_GITHUB_TOKEN")
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());

        let enabled = !matches!(
            std::env::var("KERNEXT_RELEASES_DISABLED").as_deref(),
            Ok("1") | Ok("true") | Ok("TRUE")
        );

        let client = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .unwrap_or_default();

        Self {
            client,
            repository,
            ttl: DEFAULT_TTL,
            enabled,
            token,
            cache: Mutex::new(None),
        }
    }

    /// Configured repository in `owner/name` form.
    #[must_use]
    pub fn repository(&self) -> &str {
        &self.repository
    }

    /// Latest stable release, from cache when fresh.
    pub async fn latest(&self) -> LatestRelease {
        match self.releases().await {
            Ok((releases, cached)) => match releases.iter().find(|release| !release.prerelease) {
                Some(release) => {
                    let mut payload = map_release(&self.repository, release);
                    payload.cached = cached;
                    payload
                }
                None => LatestRelease::unavailable(
                    &self.repository,
                    "the repository has no published stable release yet",
                ),
            },
            Err(error) => LatestRelease::unavailable(&self.repository, &error),
        }
    }

    /// Release history, newest first, from cache when fresh.
    pub async fn list(&self) -> ReleaseList {
        match self.releases().await {
            Ok((releases, cached)) => {
                let summaries: Vec<ReleaseSummary> = releases
                    .iter()
                    .take(HISTORY_LIMIT)
                    .map(summary_of)
                    .collect();

                if summaries.is_empty() {
                    return ReleaseList::unavailable(
                        &self.repository,
                        "the repository has no published release yet",
                    );
                }

                ReleaseList {
                    available: true,
                    repository: self.repository.clone(),
                    releases: summaries,
                    cached,
                    error: None,
                }
            }
            Err(error) => ReleaseList::unavailable(&self.repository, &error),
        }
    }

    /// Fetched releases, served from the in-memory cache when it is fresh and
    /// from the last good payload when GitHub cannot be reached. The flag
    /// reports whether the payload came from the cache rather than a fresh
    /// request.
    async fn releases(&self) -> Result<(Vec<GithubRelease>, bool), String> {
        if !self.enabled {
            return Err("release lookups are disabled in this environment".to_owned());
        }

        if let Some(cached) = self.fresh() {
            return Ok((cached, true));
        }

        match self.fetch().await {
            Ok(releases) => {
                self.store(&releases);
                Ok((releases, false))
            }
            Err(error) => match self.stale() {
                Some(releases) => Ok((releases, true)),
                None => Err(error),
            },
        }
    }

    fn fresh(&self) -> Option<Vec<GithubRelease>> {
        let cache = self.cache.lock().ok()?;
        let cached = cache.as_ref()?;
        if cached.at.elapsed() >= self.ttl {
            return None;
        }
        Some(cached.releases.clone())
    }

    fn stale(&self) -> Option<Vec<GithubRelease>> {
        self.cache
            .lock()
            .ok()?
            .as_ref()
            .map(|cached| cached.releases.clone())
    }

    fn store(&self, releases: &[GithubRelease]) {
        if let Ok(mut cache) = self.cache.lock() {
            *cache = Some(Cached {
                at: Instant::now(),
                releases: releases.to_vec(),
            });
        }
    }

    async fn fetch(&self) -> Result<Vec<GithubRelease>, String> {
        let url = format!(
            "https://api.github.com/repos/{}/releases?per_page={PAGE_SIZE}",
            self.repository
        );

        let mut request = self
            .client
            .get(url)
            .header(reqwest::header::ACCEPT, "application/vnd.github+json");

        if let Some(token) = &self.token {
            request = request.header(reqwest::header::AUTHORIZATION, format!("Bearer {token}"));
        }

        let response = request
            .send()
            .await
            .map_err(|error| format!("the GitHub API could not be reached: {error}"))?;

        let status = response.status();
        if status == reqwest::StatusCode::NOT_FOUND {
            return Err("the repository was not found, or it is private".to_owned());
        }
        if status == reqwest::StatusCode::FORBIDDEN
            || status == reqwest::StatusCode::TOO_MANY_REQUESTS
        {
            return Err(
                "the GitHub API rate limit was reached; set KERNEXT_GITHUB_TOKEN to raise it"
                    .to_owned(),
            );
        }
        if !status.is_success() {
            return Err(format!("the GitHub API returned {status}"));
        }

        let releases: Vec<GithubRelease> = response
            .json()
            .await
            .map_err(|error| format!("the GitHub API response could not be read: {error}"))?;

        Ok(releases
            .into_iter()
            .filter(|release| !release.draft)
            .collect())
    }
}

#[derive(Debug, Clone, Deserialize)]
struct GithubRelease {
    tag_name: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    html_url: Option<String>,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Clone, Deserialize)]
struct GithubAsset {
    name: String,
    #[serde(default)]
    size: u64,
    browser_download_url: String,
}

/// Classification of one asset file name.
struct AssetClass {
    platform: Option<&'static str>,
    arch: Option<&'static str>,
    format: &'static str,
    kind: &'static str,
}

impl AssetClass {
    /// Assets that exist only to accompany another file: they are linked from
    /// their target instead of being listed as downloads.
    fn is_companion(&self) -> bool {
        self.kind == "checksum" || self.kind == "signature"
    }
}

fn desktop(platform: &'static str, arch: Option<&'static str>, format: &'static str) -> AssetClass {
    AssetClass {
        platform: Some(platform),
        arch: Some(arch.unwrap_or("x64")),
        format,
        kind: "desktop",
    }
}

fn detect_arch(lower: &str) -> Option<&'static str> {
    if lower.contains("aarch64") || lower.contains("arm64") {
        Some("arm64")
    } else if lower.contains("x86_64")
        || lower.contains("x64")
        || lower.contains("amd64")
        || lower.contains("intel")
    {
        Some("x64")
    } else {
        None
    }
}

/// Platform of a CLI archive, whose name carries a Rust target triple rather
/// than the `kernext-desktop-<platform>` prefix.
fn detect_platform(lower: &str) -> Option<&'static str> {
    if lower.contains("windows") || lower.contains("pc-windows") {
        Some("windows")
    } else if lower.contains("linux") || lower.contains("musl") {
        Some("linux")
    } else if lower.contains("darwin") || lower.contains("apple") || lower.contains("macos") {
        Some("macos")
    } else {
        None
    }
}

/// Classifies an asset by its file name.
///
/// Only formats the project actually produces are mapped to a platform;
/// anything else stays `other` so the page never offers a wrong download.
fn classify_asset(name: &str) -> AssetClass {
    let lower = name.to_ascii_lowercase();
    let arch = detect_arch(&lower);

    if lower.ends_with(".sha256") || lower.ends_with(".sha512") {
        return AssetClass {
            platform: None,
            arch,
            format: "checksum",
            kind: "checksum",
        };
    }
    if lower.ends_with(".sig") {
        return AssetClass {
            platform: None,
            arch,
            format: "signature",
            kind: "signature",
        };
    }
    if lower.ends_with(".msi") {
        return desktop("windows", arch, "msi");
    }
    if lower.ends_with("-setup.exe") {
        return desktop("windows", arch, "nsis");
    }
    if lower.ends_with(".exe") {
        return desktop("windows", arch, "exe");
    }
    if lower.ends_with(".appimage") {
        return desktop("linux", arch, "appimage");
    }
    if lower.ends_with(".deb") {
        return desktop("linux", arch, "deb");
    }
    if lower.ends_with(".rpm") {
        return desktop("linux", arch, "rpm");
    }
    if lower.ends_with(".dmg") {
        return desktop("macos", arch, "dmg");
    }
    if lower.ends_with(".app.tar.gz") {
        return desktop("macos", arch, "app");
    }
    if lower.ends_with(".tar.gz") || lower.ends_with(".tar.xz") || lower.ends_with(".zip") {
        return AssetClass {
            platform: detect_platform(&lower),
            arch,
            format: "archive",
            kind: "cli",
        };
    }

    AssetClass {
        platform: None,
        arch,
        format: "other",
        kind: "other",
    }
}

/// Strips the leading `v` from a tag.
fn version_of(tag: &str) -> String {
    tag.strip_prefix('v').unwrap_or(tag).to_owned()
}

fn summary_of(release: &GithubRelease) -> ReleaseSummary {
    ReleaseSummary {
        tag: release.tag_name.clone(),
        version: version_of(&release.tag_name),
        name: release.name.clone(),
        published_at: release.published_at.clone(),
        html_url: release.html_url.clone(),
        prerelease: release.prerelease,
        asset_count: release
            .assets
            .iter()
            .filter(|asset| !classify_asset(&asset.name).is_companion())
            .count(),
    }
}

/// Converts a GitHub payload into the response the website consumes.
fn map_release(repository: &str, release: &GithubRelease) -> LatestRelease {
    let urls: HashMap<&str, &str> = release
        .assets
        .iter()
        .map(|asset| (asset.name.as_str(), asset.browser_download_url.as_str()))
        .collect();

    let mut assets = Vec::new();

    for asset in &release.assets {
        let class = classify_asset(&asset.name);
        if class.is_companion() {
            continue;
        }

        let checksum_name = format!("{}.sha256", asset.name);
        let signature_name = format!("{}.sig", asset.name);

        assets.push(ReleaseAsset {
            name: asset.name.clone(),
            size: asset.size,
            download_url: asset.browser_download_url.clone(),
            checksum_url: urls
                .get(checksum_name.as_str())
                .map(|url| (*url).to_owned()),
            signature_url: urls
                .get(signature_name.as_str())
                .map(|url| (*url).to_owned()),
            platform: class.platform.map(ToOwned::to_owned),
            arch: class.arch.map(ToOwned::to_owned),
            format: class.format.to_owned(),
            kind: class.kind.to_owned(),
        });
    }

    LatestRelease {
        available: true,
        repository: repository.to_owned(),
        version: Some(version_of(&release.tag_name)),
        tag: Some(release.tag_name.clone()),
        name: release.name.clone(),
        published_at: release.published_at.clone(),
        html_url: release.html_url.clone(),
        notes: release.body.clone().filter(|body| !body.trim().is_empty()),
        assets,
        cached: false,
        fetched_at: Some(chrono::Utc::now().to_rfc3339()),
        error: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asset(name: &str, url: &str) -> GithubAsset {
        GithubAsset {
            name: name.to_owned(),
            size: 1024,
            browser_download_url: url.to_owned(),
        }
    }

    fn release(tag: &str, assets: Vec<GithubAsset>) -> GithubRelease {
        GithubRelease {
            tag_name: tag.to_owned(),
            name: Some(format!("KERNEXT {tag}")),
            published_at: Some("2026-01-01T00:00:00Z".to_owned()),
            html_url: Some(format!("https://example.test/releases/tag/{tag}")),
            body: Some("  ".to_owned()),
            prerelease: false,
            draft: false,
            assets,
        }
    }

    #[test]
    fn installer_formats_are_classified() {
        let cases = [
            ("kernext-desktop-windows-x64.msi", "windows", "x64", "msi"),
            (
                "kernext-desktop-windows-x64-setup.exe",
                "windows",
                "x64",
                "nsis",
            ),
            (
                "kernext-desktop-linux-x64.AppImage",
                "linux",
                "x64",
                "appimage",
            ),
            ("kernext-desktop-linux-x64.deb", "linux", "x64", "deb"),
            ("kernext-desktop-linux-x64.rpm", "linux", "x64", "rpm"),
            (
                "kernext-desktop-linux-arm64.AppImage",
                "linux",
                "arm64",
                "appimage",
            ),
            ("kernext-desktop-linux-arm64.deb", "linux", "arm64", "deb"),
            ("kernext-desktop-macos-arm64.dmg", "macos", "arm64", "dmg"),
            ("kernext-desktop-macos-x64.dmg", "macos", "x64", "dmg"),
        ];

        for (name, platform, arch, format) in cases {
            let class = classify_asset(name);
            assert_eq!(class.kind, "desktop", "{name}");
            assert_eq!(class.platform, Some(platform), "{name}");
            assert_eq!(class.arch, Some(arch), "{name}");
            assert_eq!(class.format, format, "{name}");
        }
    }

    #[test]
    fn cli_archives_carry_their_platform_and_architecture() {
        let cases = [
            ("kernext-x86_64-pc-windows-gnu.zip", "windows", "x64"),
            ("kernext-x86_64-unknown-linux-gnu.tar.gz", "linux", "x64"),
            ("kernext-x86_64-unknown-linux-musl.tar.gz", "linux", "x64"),
            ("kernext-aarch64-unknown-linux-gnu.tar.gz", "linux", "arm64"),
            ("kernext-aarch64-apple-darwin.tar.gz", "macos", "arm64"),
            ("kernext-x86_64-apple-darwin.tar.gz", "macos", "x64"),
        ];

        for (name, platform, arch) in cases {
            let class = classify_asset(name);
            assert_eq!(class.kind, "cli", "{name}");
            assert_eq!(class.platform, Some(platform), "{name}");
            assert_eq!(class.arch, Some(arch), "{name}");
        }
    }

    #[test]
    fn companions_are_detected() {
        assert!(classify_asset("kernext-desktop-linux-x64.deb.sha256").is_companion());
        assert!(classify_asset("kernext-desktop-linux-x64.deb.sig").is_companion());
        assert!(!classify_asset("kernext-desktop-linux-x64.deb").is_companion());
        assert!(!classify_asset("kernext-desktop-linux-x64.app.tar.gz").is_companion());
    }

    #[test]
    fn unknown_files_stay_unclassified() {
        let class = classify_asset("source.tar");
        assert_eq!(class.kind, "other");
        assert_eq!(class.platform, None);
    }

    #[test]
    fn mapping_links_installers_to_their_checksums_and_signatures() {
        let release = release(
            "v0.2.0",
            vec![
                asset("kernext-desktop-linux-x64.deb", "https://example.test/deb"),
                asset(
                    "kernext-desktop-linux-x64.deb.sha256",
                    "https://example.test/deb.sha256",
                ),
                asset(
                    "kernext-desktop-linux-x64.deb.sig",
                    "https://example.test/deb.sig",
                ),
                asset(
                    "kernext-desktop-macos-arm64.dmg",
                    "https://example.test/dmg",
                ),
            ],
        );

        let mapped = map_release("owner/repo", &release);

        assert!(mapped.available);
        assert_eq!(mapped.version.as_deref(), Some("0.2.0"));
        assert_eq!(mapped.tag.as_deref(), Some("v0.2.0"));
        assert_eq!(mapped.notes, None, "blank release notes are dropped");
        // The `.sha256` and `.sig` assets are folded into their installer.
        assert_eq!(mapped.assets.len(), 2);
        assert_eq!(
            mapped.assets[0].checksum_url.as_deref(),
            Some("https://example.test/deb.sha256")
        );
        assert_eq!(
            mapped.assets[0].signature_url.as_deref(),
            Some("https://example.test/deb.sig")
        );
        assert_eq!(mapped.assets[1].checksum_url, None);
        assert_eq!(mapped.assets[1].signature_url, None);
    }

    #[test]
    fn history_summaries_count_only_real_artifacts() {
        let entry = release(
            "v0.1.0",
            vec![
                asset("kernext-desktop-linux-x64.deb", "https://example.test/deb"),
                asset(
                    "kernext-desktop-linux-x64.deb.sha256",
                    "https://example.test/deb.sha256",
                ),
            ],
        );

        let summary = summary_of(&entry);
        assert_eq!(summary.tag, "v0.1.0");
        assert_eq!(summary.version, "0.1.0");
        assert_eq!(summary.asset_count, 1, "the checksum is not counted");
        assert!(!summary.prerelease);
    }

    #[test]
    fn unavailable_payloads_explain_themselves() {
        let payload = LatestRelease::unavailable("owner/repo", "no release yet");
        assert!(!payload.available);
        assert!(payload.assets.is_empty());
        assert_eq!(payload.error.as_deref(), Some("no release yet"));
        assert_eq!(payload.repository, "owner/repo");

        let history = ReleaseList::unavailable("owner/repo", "rate limited");
        assert!(!history.available);
        assert!(history.releases.is_empty());
        assert_eq!(history.error.as_deref(), Some("rate limited"));
    }
}
