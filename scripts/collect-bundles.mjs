#!/usr/bin/env node
/**
 * Collects Tauri bundle output into a flat directory with canonical installer
 * names and a SHA-256 checksum file beside each installer.
 *
 * The desktop release workflow runs this once per platform so the published
 * assets have a stable, predictable shape (the runner's default names embed
 * the product name and version, which the website's download matcher should
 * not have to parse).
 *
 * Canonical names:
 *   kernext-desktop-windows-x64.msi
 *   kernext-desktop-windows-x64-setup.exe
 *   kernext-desktop-linux-x64.AppImage
 *   kernext-desktop-linux-x64.deb
 *   kernext-desktop-macos-<x64|arm64>.dmg
 *   kernext-desktop-macos-<x64|arm64>.app.tar.gz   (when produced)
 *
 * Usage:
 *   node scripts/collect-bundles.mjs \
 *     --target <cargo-target-dir> --out <dir> \
 *     --platform <windows|linux|macos> --arch <x64|arm64>
 *
 * Exits non-zero when no installer was found, so a platform whose bundling
 * silently produced nothing fails the job instead of publishing an empty
 * artifact.
 */
import { createHash } from 'node:crypto';
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readdirSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from 'node:fs';
import { basename, join, resolve } from 'node:path';

/** Installer matchers per platform, in priority order. */
const INSTALLERS = {
  windows: [
    { suffix: '.msi', name: () => 'kernext-desktop-windows-x64.msi' },
    { suffix: '-setup.exe', name: () => 'kernext-desktop-windows-x64-setup.exe' },
  ],
  linux: [
    { suffix: '.AppImage', name: () => 'kernext-desktop-linux-x64.AppImage' },
    { suffix: '.deb', name: () => 'kernext-desktop-linux-x64.deb' },
  ],
  macos: [
    { suffix: '.dmg', name: (arch) => `kernext-desktop-macos-${arch}.dmg` },
    { suffix: '.app.tar.gz', name: (arch) => `kernext-desktop-macos-${arch}.app.tar.gz` },
  ],
};

function parseArgs(argv) {
  const args = { platform: '', arch: '', target: '', out: '' };
  for (let index = 0; index < argv.length; index += 1) {
    const key = argv[index];
    if (!key.startsWith('--')) {
      continue;
    }
    args[key.slice(2)] = argv[index + 1] ?? '';
    index += 1;
  }
  return args;
}

function collectFiles(directory, predicate, found = []) {
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const full = join(directory, entry.name);
    if (entry.isDirectory()) {
      collectFiles(full, predicate, found);
    } else if (entry.isFile() && predicate(full)) {
      found.push(full);
    }
  }
  return found;
}

/** Every file under a Tauri `bundle` directory inside the cargo target. */
function bundleFiles(targetDir) {
  return collectFiles(targetDir, (file) => file.split(/[\\/]/).includes('bundle'));
}

function sha256(file) {
  return createHash('sha256').update(readFileSync(file)).digest('hex');
}

function usage() {
  return 'usage: collect-bundles.mjs --target <dir> --out <dir> --platform <windows|linux|macos> [--arch x64|arm64]\n';
}

function main() {
  const argv = process.argv.slice(2);
  if (argv.includes('--help') || argv.includes('-h')) {
    process.stdout.write(usage());
    return;
  }

  const args = parseArgs(argv);
  const platform = args.platform.toLowerCase();
  const arch = (args.arch || 'x64').toLowerCase();

  if (!args.target || !args.out || !INSTALLERS[platform]) {
    process.stderr.write(usage());
    process.exit(2);
  }

  const targetDir = resolve(args.target);
  const outDir = resolve(args.out);
  if (!existsSync(targetDir)) {
    process.stderr.write(`[collect-bundles] cargo target directory not found: ${targetDir}\n`);
    process.exit(1);
  }

  if (existsSync(outDir)) {
    rmSync(outDir, { recursive: true, force: true });
  }
  mkdirSync(outDir, { recursive: true });

  const files = bundleFiles(targetDir);
  const matchers = INSTALLERS[platform];
  const written = [];

  for (const matcher of matchers) {
    const source = files.find((file) => basename(file).endsWith(matcher.suffix));
    if (!source) {
      continue;
    }
    const name = matcher.name(arch);
    const destination = join(outDir, name);
    copyFileSync(source, destination);
    writeFileSync(`${destination}.sha256`, `${sha256(destination)}  ${name}\n`);
    written.push(name);
  }

  if (written.length === 0) {
    process.stderr.write(
      `[collect-bundles] no ${platform} installer found under ${targetDir}\n`,
    );
    process.exit(1);
  }

  process.stdout.write(`[collect-bundles] ${platform}/${arch}: ${written.join(', ')}\n`);
}

main();
