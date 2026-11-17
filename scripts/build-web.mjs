#!/usr/bin/env node
/**
 * Builds the web front end for the Tauri desktop bundler.
 *
 * `tauri.conf.json` calls this from `beforeBuildCommand` instead of a
 * hard-coded package manager. The repository supports both npm
 * (`package-lock.json`) and pnpm (`pnpm-lock.yaml`); a checkout may only have
 * one of the two installed, so the command is resolved at run time:
 *
 *   1. `KERNEXT_PACKAGE_MANAGER` (`npm` or `pnpm`) when set explicitly.
 *   2. pnpm when `pnpm-lock.yaml` exists and the pnpm binary is runnable.
 *   3. npm (which ships with Node, so it is always present).
 *
 * The web package's own `build` script is reused, so this adds no build logic.
 */
import { spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const webDir = join(repoRoot, 'apps', 'web');
const useShell = process.platform === 'win32';

function runnable(command) {
  const probe = spawnSync(command, ['--version'], { stdio: 'ignore', shell: useShell });
  return probe.status === 0;
}

function chooseManager() {
  const requested = (process.env.KERNEXT_PACKAGE_MANAGER ?? '').trim().toLowerCase();
  if (requested === 'pnpm' || requested === 'npm') {
    return requested;
  }
  if (existsSync(join(repoRoot, 'pnpm-lock.yaml')) && runnable('pnpm')) {
    return 'pnpm';
  }
  return 'npm';
}

const manager = chooseManager();
process.stdout.write(`[kernext] building the web front end with ${manager}\n`);

const result = spawnSync(manager, ['run', 'build'], {
  cwd: webDir,
  stdio: 'inherit',
  shell: useShell,
});

if (result.error) {
  process.stderr.write(`[kernext] failed to run ${manager}: ${result.error.message}\n`);
  process.exit(1);
}

process.exit(result.status ?? 1);
