/**
 * GitHub release metadata for the public downloads page.
 *
 * The page never invents a download URL. It reads
 * `/api/v1/releases/latest` — a cached server-side proxy for the GitHub
 * Releases API — and renders only the assets that exist in the payload. A
 * GitHub failure arrives as `available: false` with a reason, so the page can
 * explain itself instead of showing a broken button.
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

/** Repository used for links when the API payload is unavailable. */
export const DEFAULT_REPOSITORY = 'Trishit6/terminal';

export function repositoryUrl(slug: string): string {
  return `https://github.com/${slug}`;
}

const FORMAT_LABELS: Record<string, string> = {
  msi: 'MSI installer',
  nsis: 'Setup installer',
  exe: 'Executable',
  appimage: 'AppImage',
  deb: 'Debian package',
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
export function detectPlatform(
  userAgent: string,
  platform: string,
): PlatformId | null {
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

/** Desktop assets for one platform and architecture, in payload order. */
export function assetsFor(
  release: LatestRelease,
  platform: PlatformId,
  arch: ArchId,
): ReleaseAsset[] {
  return release.assets.filter(
    (asset) =>
      asset.kind === 'desktop' &&
      asset.platform === platform &&
      asset.arch === arch,
  );
}

export interface PlatformTarget {
  platform: PlatformId;
  arch: ArchId;
  assets: ReleaseAsset[];
}

/**
 * Every platform/architecture pair that has at least one real installer, in a
 * stable display order. Pairs with no asset are omitted, so the page can never
 * offer an installer that does not exist.
 */
export function platformTargets(release: LatestRelease): PlatformTarget[] {
  const order: Array<[PlatformId, ArchId]> = [
    ['windows', 'x64'],
    ['linux', 'x64'],
    ['macos', 'arm64'],
    ['macos', 'x64'],
  ];

  return order
    .map(([platform, arch]) => ({
      platform,
      arch,
      assets: assetsFor(release, platform, arch),
    }))
    .filter((target) => target.assets.length > 0);
}

/** Command-line archives published alongside the desktop installers. */
export function cliAssets(release: LatestRelease): ReleaseAsset[] {
  return release.assets.filter((asset) => asset.kind === 'cli');
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

/**
 * Reads the latest release from the KERNEXT server.
 *
 * Throws only when the request never produced a payload (server unreachable,
 * non-2xx status). A reachable server always answers `200`, using
 * `available: false` to describe a missing or unreachable release.
 */
export async function fetchLatestRelease(
  signal?: AbortSignal,
): Promise<LatestRelease> {
  const response = await fetch(apiUrl('/api/v1/releases/latest'), {
    credentials: 'include',
    headers: { accept: 'application/json' },
    signal,
  });

  if (!response.ok) {
    throw new Error(`The KERNEXT server answered HTTP ${response.status}.`);
  }

  return (await response.json()) as LatestRelease;
}
