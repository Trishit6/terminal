#!/usr/bin/env node
/**
 * Runs the web front end's script for the Tauri desktop bundler.
 *
 * `tauri.conf.json` calls this from `beforeBuildCommand` and
 * `beforeDevCommand` instead of hard-coding a package manager. pnpm is the
 * only supported package manager for this repository: there is exactly one
 * lockfile (`pnpm-lock.yaml`), and `package-lock.json` is deliberately absent.
 *
 * Usage:
 *
 *   node scripts/build-web.mjs         # `pnpm run build` — production bundle
 *   node scripts/build-web.mjs --dev   # `pnpm run dev`   — long-running server
 *
 * The web package's own scripts are reused, so this adds no build logic.
 */
import { spawn, spawnSync } from 'node:child_process';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const webDir = join(repoRoot, 'apps', 'web');
const useShell = process.platform === 'win32';

const dev = process.argv.slice(2).includes('--dev');
const script = dev ? 'dev' : 'build';

function pnpmRunnable() {
  const probe = spawnSync('pnpm', ['--version'], { stdio: 'ignore', shell: useShell });
  return probe.status === 0;
}

if (!pnpmRunnable()) {
  process.stderr.write(
    '[kernext] pnpm is required to build the web front end. Install it with ' +
      '`npm install -g pnpm` or `corepack enable`, then retry.\n',
  );
  process.exit(1);
}

process.stdout.write(`[kernext] pnpm run ${script} (apps/web)\n`);

if (dev) {
  // Long-running: Tauri starts this for `tauri dev` and stops it when the
  // window closes, so the child keeps the terminal and stays alive.
  const child = spawn('pnpm', ['run', script], {
    cwd: webDir,
    stdio: 'inherit',
    shell: useShell,
  });

  const stop = () => {
    child.kill();
  };

  process.on('SIGINT', stop);
  process.on('SIGTERM', stop);

  child.on('exit', (code, signal) => {
    process.exit(signal ? 1 : (code ?? 1));
  });
} else {
  const result = spawnSync('pnpm', ['run', script], {
    cwd: webDir,
    stdio: 'inherit',
    shell: useShell,
  });

  if (result.error) {
    process.stderr.write(`[kernext] failed to run pnpm: ${result.error.message}\n`);
    process.exit(1);
  }

  process.exit(result.status ?? 1);
}
