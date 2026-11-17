//! GitHub release metadata for the public downloads page.
//!
//! The website never guesses a download URL. It asks this endpoint for the
//! latest published release and renders only the assets that actually exist.
//! Successful lookups are cached in memory, so a burst of page loads produces
//! a single GitHub API call and a GitHub outage serves the last good payload
//! instead of an error.
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

/// One published asset, classified for the download page.
#[derive(Debug, Clone, Serialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub size: u64,
    pub download_url: String,
    /// Direct link to the matching `.sha256` asset, when the release has one.
    pub checksum_url: Option<String>,
    pub platform: Option<String>,
    pub arch: Option<String>,
    pub format: String,
    /// `desktop`, `cli`, or `other`.
    pub kind: String,
}

/// The latest release, or an explanation of why none could be read.
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

struct Cached {
    at: Instant,
    release: LatestRelease,
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

    /// Latest release, from cache when fresh.
    pub async fn latest(&self) -> LatestRelease {
        if !self.enabled {
            return LatestRelease::unavailable(
                &self.repository,
                "release lookups are disabled in this environment",
            );
        }

        if let Some(cached) = self.fresh() {
            return cached;
        }

        match self.fetch().await {
            Ok(release) => {
                self.store(&release);
                release
            }
            Err(error) => match self.stale() {
                Some(mut stale) => {
                    stale.cached = true;
                    stale.error = Some(format!("showing the last successful lookup: {error}"));
                    stale
                }
                None => LatestRelease::unavailable(&self.repository, &error),
            },
        }
    }

    fn fresh(&self) -> Option<LatestRelease> {
        let cache = self.cache.lock().ok()?;
        let cached = cache.as_ref()?;
        if cached.at.elapsed() >= self.ttl {
            return None;
        }
        let mut release = cached.release.clone();
        release.cached = true;
        Some(release)
    }

    fn stale(&self) -> Option<LatestRelease> {
        self.cache
            .lock()
            .ok()?
            .as_ref()
            .map(|cached| cached.release.clone())
    }

    fn store(&self, release: &LatestRelease) {
        if let Ok(mut cache) = self.cache.lock() {
            *cache = Some(Cached {
                at: Instant::now(),
                release: release.clone(),
            });
        }
    }

    async fn fetch(&self) -> Result<LatestRelease, String> {
        let url = format!(
            "https://api.github.com/repos/{}/releases/latest",
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
            return Err("the repository has no published release yet".to_owned());
        }
        if !status.is_success() {
            return Err(format!("the GitHub API returned {status}"));
        }

        let release: GithubRelease = response
            .json()
            .await
            .map_err(|error| format!("the GitHub API response could not be read: {error}"))?;

        Ok(map_release(&self.repository, release))
    }
}

#[derive(Debug, Deserialize)]
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
    assets: Vec<GithubAsset>,
}

#[derive(Debug, Deserialize)]
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
    checksum: bool,
}

fn desktop(platform: &'static str, arch: Option<&'static str>, format: &'static str) -> AssetClass {
    AssetClass {
        platform: Some(platform),
        arch: Some(arch.unwrap_or("x64")),
        format,
        kind: "desktop",
        checksum: false,
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

/// Classifies an asset by its file name.
///
/// Only formats the project actually produces are mapped to a platform;
/// anything else stays `other` so the page never offers a wrong download.
fn classify_asset(name: &str) -> AssetClass {
    let lower = name.to_ascii_lowercase();
    let arch = detect_arch(&lower);

    if lower.ends_with(".sha256") {
        return AssetClass {
            platform: None,
            arch,
            format: "sha256",
            kind: "checksum",
            checksum: true,
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
    if lower.ends_with(".dmg") {
        return desktop("macos", arch, "dmg");
    }
    if lower.ends_with(".app.tar.gz") {
        return desktop("macos", arch, "app");
    }
    if lower.ends_with(".tar.gz") || lower.ends_with(".zip") {
        return AssetClass {
            platform: None,
            arch,
            format: "archive",
            kind: "cli",
            checksum: false,
        };
    }

    AssetClass {
        platform: None,
        arch,
        format: "other",
        kind: "other",
        checksum: false,
    }
}

/// Converts a GitHub payload into the response the website consumes.
fn map_release(repository: &str, release: GithubRelease) -> LatestRelease {
    let urls: HashMap<&str, &str> = release
        .assets
        .iter()
        .map(|asset| (asset.name.as_str(), asset.browser_download_url.as_str()))
        .collect();

    let mut assets = Vec::new();

    for asset in &release.assets {
        let class = classify_asset(&asset.name);
        if class.checksum {
            continue;
        }

        let checksum_name = format!("{}.sha256", asset.name);
        let checksum_url = urls
            .get(checksum_name.as_str())
            .map(|url| (*url).to_owned());

        assets.push(ReleaseAsset {
            name: asset.name.clone(),
            size: asset.size,
            download_url: asset.browser_download_url.clone(),
            checksum_url,
            platform: class.platform.map(ToOwned::to_owned),
            arch: class.arch.map(ToOwned::to_owned),
            format: class.format.to_owned(),
            kind: class.kind.to_owned(),
        });
    }

    let version = release
        .tag_name
        .strip_prefix('v')
        .unwrap_or(&release.tag_name)
        .to_owned();

    LatestRelease {
        available: true,
        repository: repository.to_owned(),
        tag: Some(release.tag_name),
        version: Some(version),
        name: release.name,
        published_at: release.published_at,
        html_url: release.html_url,
        notes: release.body.filter(|body| !body.trim().is_empty()),
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
    fn cli_archives_are_not_desktop_installers() {
        let class = classify_asset("kernext-x86_64-pc-windows-gnu.zip");
        assert_eq!(class.kind, "cli");
        assert_eq!(class.platform, None);
        assert_eq!(class.arch, Some("x64"));

        let class = classify_asset("kernext-aarch64-apple-darwin.tar.gz");
        assert_eq!(class.kind, "cli");
        assert_eq!(class.arch, Some("arm64"));
    }

    #[test]
    fn checksums_are_detected() {
        assert!(classify_asset("kernext-desktop-linux-x64.deb.sha256").checksum);
        assert!(!classify_asset("kernext-desktop-linux-x64.deb").checksum);
    }

    #[test]
    fn unknown_files_stay_unclassified() {
        let class = classify_asset("source.tar");
        assert_eq!(class.kind, "other");
        assert_eq!(class.platform, None);
    }

    #[test]
    fn mapping_links_installers_to_their_checksums() {
        let release = GithubRelease {
            tag_name: "v0.2.0".to_owned(),
            name: Some("KERNEXT v0.2.0".to_owned()),
            published_at: Some("2026-01-01T00:00:00Z".to_owned()),
            html_url: Some("https://example.test/releases/tag/v0.2.0".to_owned()),
            body: Some("  ".to_owned()),
            assets: vec![
                asset("kernext-desktop-linux-x64.deb", "https://example.test/deb"),
                asset(
                    "kernext-desktop-linux-x64.deb.sha256",
                    "https://example.test/deb.sha256",
                ),
                asset(
                    "kernext-desktop-macos-arm64.dmg",
                    "https://example.test/dmg",
                ),
            ],
        };

        let mapped = map_release("owner/repo", release);

        assert!(mapped.available);
        assert_eq!(mapped.version.as_deref(), Some("0.2.0"));
        assert_eq!(mapped.tag.as_deref(), Some("v0.2.0"));
        assert_eq!(mapped.notes, None, "blank release notes are dropped");
        // The `.sha256` asset is folded into its installer, not listed alone.
        assert_eq!(mapped.assets.len(), 2);
        assert_eq!(
            mapped.assets[0].checksum_url.as_deref(),
            Some("https://example.test/deb.sha256")
        );
        assert_eq!(mapped.assets[1].checksum_url, None);
    }

    #[test]
    fn unavailable_payloads_explain_themselves() {
        let payload = LatestRelease::unavailable("owner/repo", "no release yet");
        assert!(!payload.available);
        assert!(payload.assets.is_empty());
        assert_eq!(payload.error.as_deref(), Some("no release yet"));
        assert_eq!(payload.repository, "owner/repo");
    }
}
