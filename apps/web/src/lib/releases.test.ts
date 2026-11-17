import { describe, expect, it } from 'vitest';
import {
  archLabel,
  assetsFor,
  cliAssets,
  cliAssetsByPlatform,
  detectArch,
  detectPlatform,
  formatBytes,
  formatLabel,
  formatReleaseDate,
  formatShortDate,
  platformTargets,
  platformTitle,
  previousReleases,
  recommendTarget,
  releaseNotesExcerpt,
  releasesUrl,
  type LatestRelease,
  type ReleaseAsset,
  type ReleaseList,
} from './releases';

function asset(overrides: Partial<ReleaseAsset>): ReleaseAsset {
  return {
    name: 'kernext-desktop-linux-x64.deb',
    size: 1024,
    download_url: 'https://example.test/download',
    checksum_url: null,
    signature_url: null,
    platform: 'linux',
    arch: 'x64',
    format: 'deb',
    kind: 'desktop',
    ...overrides,
  };
}

function release(assets: ReleaseAsset[]): LatestRelease {
  return {
    available: true,
    repository: 'owner/repo',
    tag: 'v0.2.0',
    version: '0.2.0',
    name: 'KERNEXT v0.2.0',
    published_at: '2026-01-02T03:04:05Z',
    html_url: 'https://example.test/releases/tag/v0.2.0',
    notes: null,
    assets,
    cached: false,
    fetched_at: null,
    error: null,
  };
}

describe('release asset matching', () => {
  it('returns only the assets for the requested platform and architecture', () => {
    const payload = release([
      asset({ name: 'windows.msi', platform: 'windows', arch: 'x64', format: 'msi' }),
      asset({ name: 'mac-arm.dmg', platform: 'macos', arch: 'arm64', format: 'dmg' }),
      asset({ name: 'mac-intel.dmg', platform: 'macos', arch: 'x64', format: 'dmg' }),
    ]);

    const macArm = assetsFor(payload, 'macos', 'arm64');
    expect(macArm.map((entry) => entry.name)).toEqual(['mac-arm.dmg']);
    expect(assetsFor(payload, 'linux', 'x64')).toEqual([]);
  });

  it('never advertises a platform that has no installer', () => {
    const payload = release([
      asset({ name: 'windows.msi', platform: 'windows', arch: 'x64', format: 'msi' }),
    ]);

    const targets = platformTargets(payload);
    expect(targets).toHaveLength(1);
    expect(targets[0]?.platform).toBe('windows');
    expect(targets.every((target) => target.assets.length > 0)).toBe(true);
  });

  it('orders targets predictably and keeps both macOS architectures', () => {
    const payload = release([
      asset({ name: 'mac-arm.dmg', platform: 'macos', arch: 'arm64', format: 'dmg' }),
      asset({ name: 'mac-intel.dmg', platform: 'macos', arch: 'x64', format: 'dmg' }),
      asset({ name: 'windows.msi', platform: 'windows', arch: 'x64', format: 'msi' }),
      asset({ name: 'linux.AppImage', platform: 'linux', arch: 'x64', format: 'appimage' }),
    ]);

    expect(platformTargets(payload).map((target) => [target.platform, target.arch])).toEqual([
      ['windows', 'x64'],
      ['linux', 'x64'],
      ['macos', 'arm64'],
      ['macos', 'x64'],
    ]);
  });

  it('lists Linux ARM64 right after Linux x64 when both exist', () => {
    const payload = release([
      asset({ name: 'linux-arm.AppImage', platform: 'linux', arch: 'arm64', format: 'appimage' }),
      asset({ name: 'linux-x64.AppImage', platform: 'linux', arch: 'x64', format: 'appimage' }),
    ]);

    expect(platformTargets(payload).map((target) => [target.platform, target.arch])).toEqual([
      ['linux', 'x64'],
      ['linux', 'arm64'],
    ]);
  });

  it('separates command-line archives from desktop installers', () => {
    const payload = release([
      asset({ name: 'windows.msi', platform: 'windows', arch: 'x64', format: 'msi' }),
      asset({
        name: 'kernext-x86_64-pc-windows-gnu.zip',
        platform: 'windows',
        arch: 'x64',
        format: 'archive',
        kind: 'cli',
      }),
    ]);

    expect(cliAssets(payload).map((entry) => entry.name)).toEqual([
      'kernext-x86_64-pc-windows-gnu.zip',
    ]);
  });

  it('groups command-line archives by platform, newest platforms first', () => {
    const payload = release([
      asset({
        name: 'kernext-x86_64-apple-darwin.tar.gz',
        platform: 'macos',
        arch: 'x64',
        format: 'archive',
        kind: 'cli',
      }),
      asset({
        name: 'kernext-x86_64-pc-windows-gnu.zip',
        platform: 'windows',
        arch: 'x64',
        format: 'archive',
        kind: 'cli',
      }),
      asset({
        name: 'kernext-x86_64-unknown-linux-gnu.tar.gz',
        platform: 'linux',
        arch: 'x64',
        format: 'archive',
        kind: 'cli',
      }),
      asset({
        name: 'kernext-aarch64-unknown-linux-gnu.tar.gz',
        platform: 'linux',
        arch: 'arm64',
        format: 'archive',
        kind: 'cli',
      }),
    ]);

    expect(cliAssetsByPlatform(payload).map((group) => group.platform)).toEqual([
      'windows',
      'linux',
      'macos',
    ]);
    expect(cliAssetsByPlatform(payload)[1]?.assets).toHaveLength(2);
  });

  it('keeps unclassified archives in their own group rather than hiding them', () => {
    const payload = release([
      asset({
        name: 'kernext-unknown.tar.gz',
        platform: null,
        arch: null,
        format: 'archive',
        kind: 'cli',
      }),
    ]);

    const groups = cliAssetsByPlatform(payload);
    expect(groups).toHaveLength(1);
    expect(groups[0]?.platform).toBeNull();
  });
});

describe('download recommendation', () => {
  const payload = release([
    asset({ name: 'win.msi', platform: 'windows', arch: 'x64', format: 'msi' }),
    asset({ name: 'linux.deb', platform: 'linux', arch: 'x64', format: 'deb' }),
    asset({ name: 'linux-arm.deb', platform: 'linux', arch: 'arm64', format: 'deb' }),
    asset({ name: 'mac-arm.dmg', platform: 'macos', arch: 'arm64', format: 'dmg' }),
    asset({ name: 'mac-intel.dmg', platform: 'macos', arch: 'x64', format: 'dmg' }),
  ]);

  it('recommends the detected platform and architecture', () => {
    expect(recommendTarget(payload, 'linux', 'arm64')?.arch).toBe('arm64');
    expect(recommendTarget(payload, 'windows', 'x64')?.platform).toBe('windows');
  });

  it('prefers Apple Silicon for macOS because browsers report Intel there', () => {
    expect(recommendTarget(payload, 'macos', 'x64')?.arch).toBe('arm64');
    expect(recommendTarget(payload, 'macos', null)?.arch).toBe('arm64');
  });

  it('falls back to the first target when the platform is unknown', () => {
    expect(recommendTarget(payload, null, null)?.platform).toBe('windows');
  });

  it('falls back within the platform when the architecture has no build', () => {
    const withoutArm = release([
      asset({ name: 'linux.deb', platform: 'linux', arch: 'x64', format: 'deb' }),
    ]);
    expect(recommendTarget(withoutArm, 'linux', 'arm64')?.arch).toBe('x64');
  });

  it('returns null when nothing is published', () => {
    expect(recommendTarget(release([]), 'linux', 'x64')).toBeNull();
  });
});

describe('platform detection', () => {
  it('detects the three supported desktop platforms', () => {
    expect(detectPlatform('Mozilla/5.0 (Windows NT 10.0; Win64; x64)', 'Win32')).toBe('windows');
    expect(detectPlatform('Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)', 'MacIntel')).toBe(
      'macos',
    );
    expect(detectPlatform('Mozilla/5.0 (X11; Linux x86_64)', 'Linux x86_64')).toBe('linux');
  });

  it('returns null for an unrecognised user agent', () => {
    expect(detectPlatform('curl/8.0', 'unknown')).toBeNull();
  });

  it('never guesses macOS architecture from the user agent', () => {
    // Apple Silicon browsers report an Intel user agent for compatibility.
    expect(detectArch('Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)')).toBe('x64');
    expect(detectArch('Mozilla/5.0 (X11; Linux aarch64)')).toBe('arm64');
  });

  it('labels architectures per platform', () => {
    expect(archLabel('macos', 'arm64')).toBe('Apple Silicon');
    expect(archLabel('macos', 'x64')).toBe('Intel');
    expect(archLabel('windows', 'x64')).toBe('x64');
  });

  it('names platforms and architectures for display', () => {
    expect(platformTitle('windows', 'x64')).toBe('Windows 10 / 11');
    expect(platformTitle('linux', 'x64')).toBe('Linux');
    expect(platformTitle('linux', 'arm64')).toBe('Linux (ARM64)');
    expect(platformTitle('macos', 'arm64')).toBe('macOS');
  });
});

describe('release history', () => {
  const list: ReleaseList = {
    available: true,
    repository: 'owner/repo',
    cached: false,
    error: null,
    releases: [
      {
        tag: 'v0.3.0',
        version: '0.3.0',
        name: null,
        published_at: '2026-03-01T00:00:00Z',
        html_url: 'https://example.test/v0.3.0',
        prerelease: false,
        asset_count: 9,
      },
      {
        tag: 'v0.2.0',
        version: '0.2.0',
        name: null,
        published_at: '2026-02-01T00:00:00Z',
        html_url: 'https://example.test/v0.2.0',
        prerelease: false,
        asset_count: 7,
      },
      {
        tag: 'v0.2.0-rc.1',
        version: '0.2.0-rc.1',
        name: null,
        published_at: '2026-01-20T00:00:00Z',
        html_url: 'https://example.test/v0.2.0-rc.1',
        prerelease: true,
        asset_count: 3,
      },
    ],
  };

  it('drops the release already shown as latest', () => {
    expect(previousReleases(list, 'v0.3.0').map((entry) => entry.tag)).toEqual([
      'v0.2.0',
      'v0.2.0-rc.1',
    ]);
  });

  it('keeps every release when there is no latest tag to drop', () => {
    expect(previousReleases(list, null)).toHaveLength(3);
  });
});

describe('formatting helpers', () => {
  it('labels known formats and falls back to the raw value', () => {
    expect(formatLabel('appimage')).toBe('AppImage');
    expect(formatLabel('dmg')).toBe('Disk image');
    expect(formatLabel('deb')).toBe('Debian package');
    expect(formatLabel('rpm')).toBe('RPM package');
    expect(formatLabel('weird')).toBe('WEIRD');
  });

  it('formats byte sizes', () => {
    expect(formatBytes(0)).toBe('');
    expect(formatBytes(512)).toBe('512 B');
    expect(formatBytes(2048)).toBe('2.0 KB');
    expect(formatBytes(15 * 1024 * 1024)).toBe('15 MB');
  });

  it('formats release dates and tolerates bad input', () => {
    expect(formatReleaseDate(null)).toBeNull();
    expect(formatReleaseDate('not-a-date')).toBeNull();
    expect(formatReleaseDate('2026-01-02T03:04:05Z')).toContain('2026');
    expect(formatShortDate(null)).toBeNull();
    expect(formatShortDate('not-a-date')).toBeNull();
    expect(formatShortDate('2026-01-02T03:04:05Z')).toContain('2026');
  });

  it('builds repository and releases links', () => {
    expect(releasesUrl('owner/repo')).toBe('https://github.com/owner/repo/releases');
  });
});

describe('release notes excerpt', () => {
  it('returns an empty list for missing notes', () => {
    expect(releaseNotesExcerpt(null)).toEqual([]);
    expect(releaseNotesExcerpt('   \n  \n')).toEqual([]);
  });

  it('strips Markdown markers so the excerpt reads as plain text', () => {
    const notes = [
      '## Highlights',
      '',
      '- **Native pipelines** are implemented in Rust',
      '* `clear` works on every platform',
      'Plain sentence.',
      'line 5',
      'line 6',
      'line 7',
    ].join('\n');

    const excerpt = releaseNotesExcerpt(notes, 6);
    expect(excerpt).toEqual([
      'Highlights',
      'Native pipelines are implemented in Rust',
      'clear works on every platform',
      'Plain sentence.',
      'line 5',
      'line 6',
    ]);
  });
});
