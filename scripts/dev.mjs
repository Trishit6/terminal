#!/usr/bin/env node
/**
 * Starts the local KERNEXT development stack with one command:
 *
 *   pnpm dev
 *
 * - the Axum backend (`cargo run -p shell-server`) on http://127.0.0.1:7878
 * - the Vite web client (`pnpm web:dev`) on http://127.0.0.1:5173
 *
 * The web client proxies `/api` and WebSocket upgrades to the backend, so
 * both processes are needed for the web terminal to work. Running only the
 * Vite dev server makes every API request fail — this script exists so
 * that failure mode cannot happen by accident.
 *
 * Zero runtime dependencies on purpose: the lockfile stays untouched.
 */
import { spawn } from 'node:child_process';

const isWindows = process.platform === 'win32';

const CYAN = '\u001b[36m';
const BLUE = '\u001b[34m';
const YELLOW = '\u001b[33m';
const RESET = '\u001b[0m';

const children = [];
let shuttingDown = false;

function shutdown(code = 0) {
  if (shuttingDown) {
    return;
  }

  shuttingDown = true;

  for (const child of children) {
    if (child.exitCode === null) {
      child.kill();
    }
  }

  process.exit(code);
}

function start(name, color, command, args) {
  // On Windows `pnpm` is a .cmd shim, so both commands need a shell to
  // resolve. None of the arguments contain spaces or shell metacharacters.
  const child = spawn(command, args, {
    stdio: ['ignore', 'pipe', 'pipe'],
    shell: isWindows,
  });

  children.push(child);

  const pipe = (stream, target) => {
    stream.setEncoding('utf8');

    let pending = '';

    stream.on('data', (chunk) => {
      pending += chunk;
      const lines = pending.split(/\r?\n/);
      pending = lines.pop() ?? '';

      for (const line of lines) {
        if (line.trim() !== '') {
          target.write(`${color}[${name}]${RESET} ${line}\n`);
        }
      }
    });

    stream.on('end', () => {
      if (pending.trim() !== '') {
        target.write(`${color}[${name}]${RESET} ${pending}\n`);
      }
    });
  };

  pipe(child.stdout, process.stdout);
  pipe(child.stderr, process.stderr);

  child.on('exit', (code) => {
    if (shuttingDown) {
      return;
    }

    process.stderr.write(
      `${YELLOW}[dev] ${name} exited with code ${code ?? 'null'}; stopping the stack.${RESET}\n`,
    );
    shutdown(1);
  });

  return child;
}

process.on('SIGINT', () => shutdown(0));
process.on('SIGTERM', () => shutdown(0));

console.log(`${CYAN}[dev]${RESET} KERNEXT development stack`);
console.log(`${CYAN}[dev]${RESET}   backend : http://127.0.0.1:7878`);
console.log(`${CYAN}[dev]${RESET}   web     : http://127.0.0.1:5173`);
console.log(`${CYAN}[dev]${RESET} Ctrl+C stops both processes.\n`);

start('server', CYAN, 'cargo', ['run', '-p', 'shell-server']);
start('web', BLUE, 'pnpm', ['web:dev']);
