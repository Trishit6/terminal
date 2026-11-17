import { describe, expect, it } from 'vitest';
import {
  archLabel,
  assetsFor,
  cliAssets,
  detectArch,
  detectPlatform,
  formatBytes,
  formatLabel,
  formatReleaseDate,
  platformTargets,
  type LatestRelease,
  type ReleaseAsset,
} from './releases';

function asset(overrides: Partial<ReleaseAsset>): ReleaseAsset {
  return {
    name: 'kernext-desktop-linux-x64.deb',
    size: 1024,
    download_url: 'https://example.test/download',
    checksum_url: null,
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

  it('separates command-line archives from desktop installers', () => {
    const payload = release([
      asset({ name: 'windows.msi', platform: 'windows', arch: 'x64', format: 'msi' }),
      asset({
        name: 'kernext-x86_64-pc-windows-gnu.zip',
        platform: null,
        arch: 'x64',
        format: 'archive',
        kind: 'cli',
      }),
    ]);

    expect(cliAssets(payload).map((entry) => entry.name)).toEqual([
      'kernext-x86_64-pc-windows-gnu.zip',
    ]);
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
});

describe('formatting helpers', () => {
  it('labels known formats and falls back to the raw value', () => {
    expect(formatLabel('appimage')).toBe('AppImage');
    expect(formatLabel('dmg')).toBe('Disk image');
    expect(formatLabel('rpm')).toBe('RPM');
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
  });
});
