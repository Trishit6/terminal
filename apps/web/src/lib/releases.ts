/**
 * GitHub release metadata for the public downloads page.
 *
 * The page never invents a download URL, a version, a file size, or a release
 * date. It reads the KERNEXT server's cached proxy for the GitHub Releases API
 * and renders only the assets that exist in the payload. A GitHub failure
 * arrives as `available: false` with a reason, so the page can explain itself
 * instead of showing a broken button.
 *
 * Endpoints consumed here:
 *
 * - `GET /api/v1/releases/latest` — newest stable release plus its assets.
 * - `GET /api/v1/releases` — release history for the previous-versions list.
 */

import { apiUrl } from './server';

export type PlatformId = 'windows' | 'linux' | 'macos';
export type ArchId = 'x64' | 'arm64';

/** One published asset, already classified by the server. */
export interface ReleaseAsset {
  name: string;
  size: number;
  download_url: string;
  checksum_url: string | null;
  signature_url: string | null;
  platform: PlatformId | null;
  arch: ArchId | null;
  format: string;
  kind: 'desktop' | 'cli' | 'other';
}

/** The latest published release, or the reason there is none. */
export interface LatestRelease {
  available: boolean;
  repository: string;
  tag: string | null;
  version: string | null;
  name: string | null;
  published_at: string | null;
  html_url: string | null;
  notes: string | null;
  assets: ReleaseAsset[];
  cached: boolean;
  fetched_at: string | null;
  error: string | null;
}

/** One entry in the release history. */
export interface ReleaseSummary {
  tag: string;
  version: string;
  name: string | null;
  published_at: string | null;
  html_url: string | null;
  prerelease: boolean;
  asset_count: number;
}

/** The release history, newest first. */
export interface ReleaseList {
  available: boolean;
  repository: string;
  releases: ReleaseSummary[];
  cached: boolean;
  error: string | null;
}

/** Repository used for links when the API payload is unavailable. */
export const DEFAULT_REPOSITORY = 'Trishit6/terminal';

export function repositoryUrl(slug: string): string {
  return `https://github.com/${slug}`;
}

/** Direct link to the releases page for a repository. */
export function releasesUrl(slug: string): string {
  return `${repositoryUrl(slug)}/releases`;
}

const FORMAT_LABELS: Record<string, string> = {
  msi: 'MSI installer',
  nsis: 'Setup installer',
  exe: 'Executable',
  appimage: 'AppImage',
  deb: 'Debian package',
  rpm: 'RPM package',
  dmg: 'Disk image',
  app: 'Application bundle',
  archive: 'Archive',
};

export function formatLabel(format: string): string {
  return FORMAT_LABELS[format] ?? format.toUpperCase();
}

const PLATFORM_LABELS: Record<PlatformId, string> = {
  windows: 'Windows',
  linux: 'Linux',
  macos: 'macOS',
};

export function platformLabel(platform: PlatformId): string {
  return PLATFORM_LABELS[platform];
}

/** Full operating-system name shown on a platform card. */
export function platformTitle(platform: PlatformId, arch: ArchId): string {
  if (platform === 'windows') {
    return 'Windows 10 / 11';
  }
  if (platform === 'macos') {
    return 'macOS';
  }
  return arch === 'arm64' ? 'Linux (ARM64)' : 'Linux';
}

/** Human-readable architecture name for a platform. */
export function archLabel(platform: PlatformId, arch: ArchId): string {
  if (platform === 'macos') {
    return arch === 'arm64' ? 'Apple Silicon' : 'Intel';
  }

  return arch;
}

/**
 * Best-effort operating-system detection for highlighting the visitor's own
 * platform. Never used to hide other platforms: every published installer
 * stays reachable.
 */
export function detectPlatform(userAgent: string, platform: string): PlatformId | null {
  const haystack = `${userAgent} ${platform}`.toLowerCase();

  if (haystack.includes('win')) {
    return 'windows';
  }
  if (haystack.includes('mac') || haystack.includes('darwin')) {
    return 'macos';
  }
  if (haystack.includes('linux') || haystack.includes('x11') || haystack.includes('android')) {
    return 'linux';
  }

  return null;
}

/**
 * Best-effort architecture detection. Browsers on Apple Silicon still report
 * an Intel user agent, so macOS architecture is never auto-selected from this
 * alone — the page shows both.
 */
export function detectArch(userAgent: string): ArchId | null {
  const haystack = userAgent.toLowerCase();

  if (haystack.includes('aarch64') || haystack.includes('arm64') || haystack.includes('arm')) {
    return 'arm64';
  }
  if (
    haystack.includes('x86_64') ||
    haystack.includes('x64') ||
    haystack.includes('win64') ||
    haystack.includes('intel')
  ) {
    return 'x64';
  }

  return null;
}

/** The visitor's detected platform and architecture, when it can be read. */
export interface VisitorPlatform {
  platform: PlatformId | null;
  arch: ArchId | null;
}

/**
 * Reads the visitor's platform and architecture from the browser.
 *
 * Returns empty values when there is no `navigator` (for example during a
 * build-time render), so callers never have to guard for it.
 */
export function detectVisitorPlatform(): VisitorPlatform {
  if (typeof navigator === 'undefined') {
    return { platform: null, arch: null };
  }

  const userAgent = navigator.userAgent ?? '';
  const platform = (navigator as { platform?: string }).platform ?? '';

  return {
    platform: detectPlatform(userAgent, platform),
    arch: detectArch(userAgent),
  };
}

/** Desktop assets for one platform and architecture, in payload order. */
export function assetsFor(
  release: LatestRelease,
  platform: PlatformId,
  arch: ArchId,
): ReleaseAsset[] {
  return release.assets.filter(
    (asset) => asset.kind === 'desktop' && asset.platform === platform && asset.arch === arch,
  );
}

export interface PlatformTarget {
  platform: PlatformId;
  arch: ArchId;
  assets: ReleaseAsset[];
}

/**
 * Display order for platform cards. Linux ARM64 sits after Linux x64 so the
 * two architectures read as a pair.
 */
const TARGET_ORDER: Array<[PlatformId, ArchId]> = [
  ['windows', 'x64'],
  ['linux', 'x64'],
  ['linux', 'arm64'],
  ['macos', 'arm64'],
  ['macos', 'x64'],
];

/**
 * Every platform/architecture pair that has at least one real installer, in a
 * stable display order. Pairs with no asset are omitted, so the page can never
 * offer an installer that does not exist.
 */
export function platformTargets(release: LatestRelease): PlatformTarget[] {
  return TARGET_ORDER.map(([platform, arch]) => ({
    platform,
    arch,
    assets: assetsFor(release, platform, arch),
  })).filter((target) => target.assets.length > 0);
}

/**
 * Picks the download to recommend, without preventing manual selection.
 *
 * macOS defaults to Apple Silicon because every browser there reports an Intel
 * user agent, and Apple Silicon runs Intel builds through Rosetta while the
 * reverse is not true. Other platforms trust the detected architecture and
 * fall back to the first available target.
 */
export function recommendTarget(
  release: LatestRelease,
  visitorPlatform: PlatformId | null,
  visitorArch: ArchId | null,
): PlatformTarget | null {
  const targets = platformTargets(release);
  if (targets.length === 0) {
    return null;
  }

  const find = (platform: PlatformId, arch: ArchId) =>
    targets.find((target) => target.platform === platform && target.arch === arch);

  if (visitorPlatform === 'macos') {
    return find('macos', 'arm64') ?? find('macos', 'x64') ?? targets[0]!;
  }

  if (visitorPlatform) {
    const preferred = visitorArch ? find(visitorPlatform, visitorArch) : undefined;
    if (preferred) {
      return preferred;
    }

    const firstForPlatform = targets.find((target) => target.platform === visitorPlatform);
    if (firstForPlatform) {
      return firstForPlatform;
    }
  }

  return targets[0]!;
}

/** Command-line archives published alongside the desktop installers. */
export function cliAssets(release: LatestRelease): ReleaseAsset[] {
  return release.assets.filter((asset) => asset.kind === 'cli');
}

/** Command-line archives grouped by platform, in display order. */
export function cliAssetsByPlatform(release: LatestRelease): Array<{
  platform: PlatformId | null;
  assets: ReleaseAsset[];
}> {
  const assets = cliAssets(release);
  const groups: Array<{ platform: PlatformId | null; assets: ReleaseAsset[] }> = [];

  // One group per platform. `TARGET_ORDER` is keyed by architecture so the
  // desktop cards can split x64 from arm64, but a CLI archive's own file name
  // carries its Rust target triple — so x64 and arm64 archives stay together
  // under a single platform heading instead of repeating it.
  const seen = new Set<PlatformId>();
  for (const [platform] of TARGET_ORDER) {
    if (seen.has(platform)) {
      continue;
    }
    seen.add(platform);

    const forPlatform = assets.filter((asset) => asset.platform === platform);
    if (forPlatform.length > 0) {
      groups.push({ platform, assets: forPlatform });
    }
  }

  const unclassified = assets.filter((asset) => asset.platform === null);
  if (unclassified.length > 0) {
    groups.push({ platform: null, assets: unclassified });
  }

  return groups;
}

export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) {
    return '';
  }

  const units = ['B', 'KB', 'MB', 'GB'];
  let value = bytes;
  let unit = 0;

  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }

  return `${value >= 10 || unit === 0 ? Math.round(value) : value.toFixed(1)} ${units[unit]}`;
}

/** Long-form release date, or `null` when it is missing or unparsable. */
export function formatReleaseDate(iso: string | null): string | null {
  if (!iso) {
    return null;
  }

  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) {
    return null;
  }

  return date.toLocaleDateString(undefined, {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  });
}

/** Short release date used by the previous-releases list. */
export function formatShortDate(iso: string | null): string | null {
  if (!iso) {
    return null;
  }

  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) {
    return null;
  }

  return date.toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
}

/**
 * Renders release notes as a short plain-text summary.
 *
 * GitHub release bodies are Markdown. The page shows a bounded excerpt rather
 * than a rendered document, so no Markdown parser or `dangerouslySetInnerHTML`
 * is needed and the notes can never inject markup. Heading markers and
 * emphasis markers are stripped so the excerpt reads as plain sentences.
 */
export function releaseNotesExcerpt(notes: string | null, maxLines = 6): string[] {
  if (!notes) {
    return [];
  }

  return notes
    .split(/\r?\n/)
    .map((line) =>
      line
        .trim()
        .replace(/^#{1,6}\s*/, '')
        .replace(/^[-*+]\s+/, '')
        .replace(/\*\*/g, '')
        .replace(/`/g, ''),
    )
    .filter((line) => line.length > 0)
    .slice(0, maxLines);
}

/** Previous releases, with the release already shown as "latest" removed. */
export function previousReleases(list: ReleaseList, latestTag: string | null): ReleaseSummary[] {
  return list.releases.filter((entry) => entry.tag !== latestTag);
}

async function fetchJson<T>(path: string, signal?: AbortSignal): Promise<T> {
  const response = await fetch(apiUrl(path), {
    credentials: 'include',
    headers: { accept: 'application/json' },
    signal,
  });

  if (!response.ok) {
    throw new Error(`The KERNEXT server answered HTTP ${response.status}.`);
  }

  return (await response.json()) as T;
}

/**
 * Reads the latest release from the KERNEXT server.
 *
 * Throws only when the request never produced a payload (server unreachable,
 * non-2xx status). A reachable server always answers `200`, using
 * `available: false` to describe a missing or unreachable release.
 */
export function fetchLatestRelease(signal?: AbortSignal): Promise<LatestRelease> {
  return fetchJson<LatestRelease>('/api/v1/releases/latest', signal);
}

/** Reads the release history from the KERNEXT server. */
export function fetchReleaseList(signal?: AbortSignal): Promise<ReleaseList> {
  return fetchJson<ReleaseList>('/api/v1/releases', signal);
}
