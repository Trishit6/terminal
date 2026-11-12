#!/usr/bin/env node
/**
 * Starts the local KERNEXT development stack with one command:
 *
 *   npm run dev
 *
 * - the Axum backend (`cargo run -p shell-server`) on http://127.0.0.1:7878
 * - the Vite web client (`npm run web:dev`) on http://127.0.0.1:5173
 *
 * The web client proxies `/api` and WebSocket upgrades to the backend, so
 * both processes are needed for the web terminal to work. This script
 * preflights both ports and waits for the backend health endpoint before it
 * starts Vite. That ordering matters: Vite used to come up while Cargo was
 * still compiling, so every open browser tab proxied to a dead port and
 * produced ECONNREFUSED noise until the server finished starting.
 *
 * Zero runtime dependencies on purpose: the lockfile stays untouched.
 */
import { spawn } from 'node:child_process';
import net from 'node:net';

const isWindows = process.platform === 'win32';

const CYAN = '\u001b[36m';
const BLUE = '\u001b[34m';
const YELLOW = '\u001b[33m';
const RED = '\u001b[31m';
const RESET = '\u001b[0m';

const API_PORT = 7878;
const WEB_PORT = 5175;
const HEALTH_URL = `http://127.0.0.1:${API_PORT}/api/v1/health`;
const BACKEND_START_TIMEOUT_MS = 10 * 60 * 1000;

const children = [];
let shuttingDown = false;

function shutdown(code = 0) {
  if (shuttingDown) {
    return;
  }

  shuttingDown = true;

  // Stop the web client before the backend. Closing Vite first tears down its
  // proxy and terminal sockets cleanly instead of leaving them to reset when
  // the Axum process disappears underneath them.
  for (const child of [...children].reverse()) {
    if (child.exitCode === null) {
      child.kill();
    }
  }

  process.exit(code);
}

function start(name, color, command, args) {
  // On Windows `npm` is a .cmd shim, so both commands need a shell to
  // resolve. Pass one pre-joined command string instead of an args array to
  // avoid Node's DEP0190 warning about unescaped shell arguments. These
  // commands are fixed literals with no spaces.
  const child = isWindows
    ? spawn([command, ...args].join(' '), {
        stdio: ['ignore', 'pipe', 'pipe'],
        shell: true,
      })
    : spawn(command, args, {
        stdio: ['ignore', 'pipe', 'pipe'],
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

function portInUse(port, timeoutMs = 700) {
  return new Promise((resolve) => {
    const socket = net.connect({ host: '127.0.0.1', port });
    let settled = false;

    const finish = (inUse) => {
      if (settled) {
        return;
      }

      settled = true;
      socket.destroy();
      resolve(inUse);
    };

    socket.once('connect', () => finish(true));
    socket.once('error', () => finish(false));
    socket.setTimeout(timeoutMs, () => finish(false));
  });
}

async function backendHealthy() {
  try {
    const response = await fetch(HEALTH_URL, {
      signal: AbortSignal.timeout(1200),
    });

    if (!response.ok) {
      return false;
    }

    const body = await response.json();
    return body?.product === 'KERNEXT';
  } catch {
    return false;
  }
}

async function waitForBackend(timeoutMs) {
  const startedAt = Date.now();
  let lastNoticeAt = 0;

  while (!shuttingDown && Date.now() - startedAt < timeoutMs) {
    if (await backendHealthy()) {
      return true;
    }

    if (Date.now() - lastNoticeAt >= 15_000) {
      lastNoticeAt = Date.now();
      console.log(
        `${CYAN}[dev]${RESET} waiting for the backend at ${HEALTH_URL} (compiling shell-server)…`,
      );
    }

    await new Promise((resolve) => setTimeout(resolve, 500));
  }

  return false;
}

process.on('SIGINT', () => shutdown(0));
process.on('SIGTERM', () => shutdown(0));

console.log(`${CYAN}[dev]${RESET} KERNEXT development stack`);
console.log(`${CYAN}[dev]${RESET}   backend : http://127.0.0.1:${API_PORT}`);
console.log(`${CYAN}[dev]${RESET}   web     : http://127.0.0.1:${WEB_PORT}`);
console.log(`${CYAN}[dev]${RESET} Ctrl+C stops both processes.\n`);

if (await portInUse(WEB_PORT)) {
  console.error(
    `${RED}[dev]${RESET} Port ${WEB_PORT} is already in use. Stop the existing Vite dev server, then run \`npm run dev\` again.`,
  );
  process.exit(1);
}

const existingBackend = await backendHealthy();

if (!existingBackend && (await portInUse(API_PORT))) {
  console.error(
    `${RED}[dev]${RESET} Port ${API_PORT} is in use, but ${HEALTH_URL} did not answer as a KERNEXT backend. Stop that process, then run \`npm run dev\` again.`,
  );
  process.exit(1);
}

if (existingBackend) {
  console.log(
    `${CYAN}[dev]${RESET} Using the KERNEXT backend already running on port ${API_PORT}.`,
  );
} else {
  start('server', CYAN, 'cargo', ['run', '-p', 'shell-server']);

  if (!(await waitForBackend(BACKEND_START_TIMEOUT_MS))) {
    if (!shuttingDown) {
      console.error(
        `${RED}[dev]${RESET} The backend did not become healthy at ${HEALTH_URL}. Check the [server] output above, then run \`npm run dev\` again.`,
      );
      shutdown(1);
    }
  }
}

start('web', BLUE, 'npm', ['run', 'web:dev']);
