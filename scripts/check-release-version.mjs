#!/usr/bin/env node
/**
 * Fails when a release tag disagrees with the versions declared in the
 * workspace manifests.
 *
 * Without this check a mistyped tag publishes installers whose embedded
 * version (baked into the bundle file names and the app's About screen) does
 * not match the release the user is looking at.
 *
 * Usage:
 *   node scripts/check-release-version.mjs v0.2.0
 */
import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const tag = (process.argv[2] ?? '').trim();

if (!tag) {
  process.stderr.write('usage: check-release-version.mjs <tag>\n');
  process.exit(2);
}

const expected = tag.replace(/^v/, '');

function readJson(relativePath) {
  return JSON.parse(readFileSync(join(repoRoot, relativePath), 'utf8'));
}

const cargoManifest = readFileSync(join(repoRoot, 'Cargo.toml'), 'utf8');
const cargoVersion = cargoManifest.match(/^\s*version\s*=\s*"([^"]+)"/m)?.[1] ?? null;
const tauriVersion = readJson('apps/desktop/src-tauri/tauri.conf.json').version ?? null;

const declared = [
  ['Cargo.toml [workspace.package] version', cargoVersion],
  ['apps/desktop/src-tauri/tauri.conf.json version', tauriVersion],
  ['package.json version', readJson('package.json').version],
  ['apps/web/package.json version', readJson('apps/web/package.json').version],
  ['apps/desktop/package.json version', readJson('apps/desktop/package.json').version],
];

const mismatches = declared.filter(([, version]) => version !== expected);

if (mismatches.length > 0) {
  process.stderr.write(`release tag ${tag} expects version ${expected}, but:\n`);
  for (const [where, version] of mismatches) {
    process.stderr.write(`  - ${where} is ${version ?? '<missing>'}\n`);
  }
  process.exit(1);
}

process.stdout.write(`release version ${expected} matches every manifest\n`);
