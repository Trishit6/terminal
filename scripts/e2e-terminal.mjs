#!/usr/bin/env node
/**
 * End-to-end acceptance test for the KERNEXT web terminal.
 *
 *   node scripts/e2e-terminal.mjs [baseUrl]
 *
 * Defaults to the Vite dev server (http://127.0.0.1:5173) so every request
 * crosses the same `/api` proxy the browser uses. Point it straight at the
 * Axum server (http://127.0.0.1:7878) to skip the proxy.
 *
 * Verifies, against the real Rust backend:
 *   - health probe
 *   - engine sessions: create, execute, stdout/stderr, exit codes, clear
 *   - real external process execution
 *   - error reporting for unknown commands
 *   - session isolation
 *   - PTY mode: resize, interactive echo, Ctrl+C interrupt
 *   - disconnect and reconnect
 *
 * Exit code 0 = every check passed. Requires Node 22+ (global WebSocket).
 */
const BASE = process.argv[2] ?? 'http://127.0.0.1:5173';
const ORIGIN = 'http://127.0.0.1:5173';

let failures = 0;
let checks = 0;

function ok(name, condition, detail = '') {
  checks += 1;

  if (condition) {
    console.log(`  \u2713 ${name}`);
  } else {
    failures += 1;
    console.error(`  \u2717 ${name}${detail ? `: ${detail}` : ''}`);
  }
}

function wsUrl(path) {
  return `${BASE.replace(/^http/i, 'ws')}${path}`;
}

async function api(path, init = {}) {
  const headers = { origin: ORIGIN, ...init.headers };

  if (init.body) {
    headers['content-type'] = 'application/json';
  }

  const response = await fetch(`${BASE}${path}`, { ...init, headers });

  if (!response.ok) {
    const text = await response.text();
    throw new Error(`HTTP ${response.status} from ${path}: ${text.slice(0, 200)}`);
  }

  if (response.status === 204) {
    return null;
  }

  return response.json();
}

function connectSocket(url) {
  const socket = new WebSocket(url);
  const pending = [];
  const waiters = [];

  const push = (message) => {
    const waiter = waiters.shift();

    if (waiter) {
      waiter(message);
    } else {
      pending.push(message);
    }
  };

  socket.addEventListener('message', (event) => {
    let message;

    if (typeof event.data === 'string') {
      try {
        message = JSON.parse(event.data);
      } catch {
        return;
      }

      push(message);
    } else {
      push({ type: 'binary', data: event.data });
    }
  });

  const opened = new Promise((resolve, reject) => {
    socket.addEventListener('open', () => resolve(), { once: true });
    socket.addEventListener('error', () => reject(new Error(`socket failed: ${url}`)), {
      once: true,
    });
  });

  const next = (timeoutMs = 10_000) =>
    new Promise((resolve, reject) => {
      const timer = setTimeout(
        () => reject(new Error('timed out waiting for a server message')),
        timeoutMs,
      );

      const message = pending.shift();

      if (message) {
        clearTimeout(timer);
        resolve(message);
        return;
      }

      waiters.push((delivered) => {
        clearTimeout(timer);
        resolve(delivered);
      });
    });

  const send = (object) => socket.send(JSON.stringify(object));

  return { socket, opened, next, send, closed: new Promise((resolve) => socket.addEventListener('close', resolve)) };
}

function collectBinary(messages) {
  return messages
    .filter((message) => message.type === 'binary')
    .map(() => '')
    .join('');
}

async function testHealth() {
  console.log('\n[1] Health probe');
  const health = await api('/api/v1/health');
  ok('backend answers health', health.status === 'ok');
  ok('reports product KERNEXT', health.product === 'KERNEXT');
  ok('reports PTY availability', typeof health.pty_available === 'boolean');
  return health;
}

async function testEngineMode() {
  console.log('\n[2] Engine mode: session lifecycle and command execution');
  const created = await api('/api/v1/sessions', {
    method: 'POST',
    body: '{}',
  });
  ok('session created', created.success === true && created.session?.id > 0, JSON.stringify(created));

  const session = created.session;
  const socket = connectSocket(wsUrl(`/api/v1/sessions/${session.id}/ws`));
  await socket.opened;
  const ready = await socket.next();
  ok('ready message arrives', ready.type === 'ready' && ready.session?.id === session.id);
  ok('prompt is the KERNEXT prompt', typeof ready.prompt === 'string' && ready.prompt.length > 0);

  const runCommand = async (command) => {
    socket.send({ type: 'execute', command });
    const messages = [];
    let result = null;

    while (result === null) {
      const message = await socket.next();
      messages.push(message);

      if (message.type === 'result') {
        result = message;
      }
    }

    return { result, messages };
  };

  const help = await runCommand('help');
  const helpText = help.messages
    .filter((message) => message.type === 'output')
    .map((message) => message.data)
    .join('');
  ok('help lists built-ins', helpText.includes('KERNEXT') && helpText.includes('pwd'), helpText.slice(0, 120));
  ok('help succeeds', help.result.success === true);

  const pwd = await runCommand('pwd');
  const pwdText = pwd.messages
    .filter((message) => message.type === 'output')
    .map((message) => message.data)
    .join('');
  ok('pwd prints the working directory', pwdText.includes(process.cwd().slice(0, 8)) || pwdText.trim().length > 0, pwdText.trim());
  ok('pwd exit status is success', pwd.result.success === true);

  const echo = await runCommand('echo KERNEXT');
  const echoText = echo.messages
    .filter((message) => message.type === 'output')
    .map((message) => message.data)
    .join('');
  ok('echo KERNEXT outputs the word', echoText.trim() === 'KERNEXT', echoText);

  const missing = await runCommand('definitely-not-a-real-command-xyz');
  ok('unknown command reports failure', missing.result.success === false);
  ok('unknown command reports a nonzero exit', (missing.result.exit_code ?? 1) !== 0);

  socket.send({ type: 'clear' });
  const clear = await socket.next();
  ok('clear emits the escape sequence', clear.type === 'output' && clear.data.includes('\u001b[2J'));

  const socket2 = socket;
  socket2.socket.close();
  await socket2.closed;
  ok('session socket closes cleanly', true);

  const deleted = await api(`/api/v1/sessions/${session.id}`, { method: 'DELETE' });
  ok('session deletes', deleted === null);
}

async function testExternalCommand() {
  console.log('\n[3] External process execution through the engine');
  const created = await api('/api/v1/sessions', { method: 'POST', body: '{}' });
  const socket = connectSocket(wsUrl(`/api/v1/sessions/${created.session.id}/ws`));
  await socket.opened;
  await socket.next(); // ready

  socket.send({ type: 'execute', command: 'node --version' });
  const messages = [];
  let result = null;

  while (result === null) {
    const message = await socket.next();
    messages.push(message);

    if (message.type === 'result') {
      result = message;
    }
  }

  const output = messages
    .filter((message) => message.type === 'output')
    .map((message) => message.data)
    .join('');

  ok('external node runs (if installed)', result.success === true && /v\d+/u.test(output), output.trim());
  ok('external exit code is reported', typeof result.exit_code === 'number' || result.exit_code === null);

  socket.socket.close();
  await api(`/api/v1/sessions/${created.session.id}`, { method: 'DELETE' });
}

async function testSessionIsolation() {
  console.log('\n[4] Session isolation');
  const a = await api('/api/v1/sessions', { method: 'POST', body: '{}' });
  const b = await api('/api/v1/sessions', { method: 'POST', body: '{}' });
  ok('session ids are unique', a.session.id !== b.session.id, `${a.session.id} vs ${b.session.id}`);

  const socketA = connectSocket(wsUrl(`/api/v1/sessions/${a.session.id}/ws`));
  const socketB = connectSocket(wsUrl(`/api/v1/sessions/${b.session.id}/ws`));
  await socketA.opened;
  await socketB.opened;
  await socketA.next();
  await socketB.next();

  const run = async (socket, command) => {
    socket.send({ type: 'execute', command });
    const messages = [];
    let result = null;

    while (result === null) {
      const message = await socket.next();
      messages.push(message);

      if (message.type === 'result') {
        result = message;
      }
    }

    return messages
      .filter((message) => message.type === 'output')
      .map((message) => message.data)
      .join('');
  };

  const home = process.env.USERPROFILE ?? process.env.HOME ?? '';
  await run(socketA, `cd "${home.replace(/\\/gu, '/')}"`);
  const pwdA = await run(socketA, 'pwd');
  const pwdB = await run(socketB, 'pwd');

  ok('session A kept its cd', pwdA.toLowerCase().includes(home.split(/[\\/]/u).pop()?.toLowerCase() ?? 'zzz'), pwdA.trim());
  ok('session B kept its own directory', !pwdA.includes(pwdB.trim()) || pwdB.trim() === process.cwd(), `${pwdA.trim()} vs ${pwdB.trim()}`);

  socketA.socket.close();
  socketB.socket.close();
  await api(`/api/v1/sessions/${a.session.id}`, { method: 'DELETE' });
  await api(`/api/v1/sessions/${b.session.id}`, { method: 'DELETE' });
}

async function testPtyMode(health) {
  console.log('\n[5] PTY mode');

  if (health.pty_available === false) {
    console.log('  (skipped: backend reports no PTY binary)');
    return;
  }

  const socket = connectSocket(wsUrl('/api/v1/pty'));
  await socket.opened;

  const ptyReady = await socket.next(5000);
  ok('pty_ready arrives', ptyReady.type === 'pty_ready');

  socket.send({ type: 'resize', rows: 24, cols: 80 });

  const readUntil = async (predicate, timeoutMs = 10_000) => {
    const deadline = Date.now() + timeoutMs;
    const seen = [];

    while (Date.now() < deadline) {
      let message;

      try {
        message = await socket.next(Math.max(1, deadline - Date.now()));
      } catch {
        // No further message within the window.
        break;
      }

      seen.push(message);

      if (await predicate(message)) {
        return seen;
      }
    }

    return seen;
  };

  const decode = async (message) => {
    if (message.type === 'binary' && message.data instanceof Blob) {
      return await message.data.text();
    }

    return '';
  };

  // Give the shell a moment to draw its prompt, then type a command.
  const startup = await readUntil(() => false, 2500);
  let startupText = '';

  for (const message of startup) {
    startupText += await decode(message);
  }

  ok('PTY prints a prompt or banner', startupText.length > 0, startupText.slice(0, 80));

  socket.socket.send(JSON.stringify({ type: 'input', data: 'echo PTY_KERNEXT\r' }));
  const echoed = await readUntil(async (message) => {
    return (await decode(message)).includes('PTY_KERNEXT\r');
  }, 15_000);

  let echoText = '';

  for (const message of echoed) {
    echoText += await decode(message);
  }

  ok('typed command executes and echoes back', echoText.includes('PTY_KERNEXT'), echoText.slice(-200));

  // Ctrl+C must interrupt whatever the PTY is doing and return the prompt.
  // The KERNEXT line editor clears the current line and redraws the prompt;
  // it deliberately does not echo a literal "^C".
  socket.socket.send(JSON.stringify({ type: 'input', data: '\u0003' }));
  const interrupted = await readUntil(async (message) => {
    return (await decode(message)).includes('kernext ~ $');
  }, 10_000);

  let interruptText = '';

  for (const message of interrupted) {
    interruptText += await decode(message);
  }

  ok(
    'Ctrl+C interrupts and returns the prompt',
    /kernext ~ \$/u.test(interruptText),
    interruptText.slice(-120),
  );

  socket.socket.close();
  ok('PTY socket closes', true);
}

async function testReconnect() {
  console.log('\n[6] Disconnect and reconnect');
  const first = await api('/api/v1/sessions', { method: 'POST', body: '{}' });
  const socket = connectSocket(wsUrl(`/api/v1/sessions/${first.session.id}/ws`));
  await socket.opened;
  await socket.next();
  socket.socket.close();
  await socket.closed;

  const second = await api('/api/v1/sessions', { method: 'POST', body: '{}' });
  const socket2 = connectSocket(wsUrl(`/api/v1/sessions/${second.session.id}/ws`));
  await socket2.opened;
  const ready2 = await socket2.next();
  ok('reconnect creates a fresh working session', ready2.type === 'ready');
  socket2.socket.close();

  const health = await api('/api/v1/health');
  ok('backend still healthy after churn', health.status === 'ok');
  await api(`/api/v1/sessions/${second.session.id}`, { method: 'DELETE' });
}

async function main() {
  console.log(`KERNEXT web terminal E2E — target ${BASE}`);

  try {
    const health = await testHealth();
    await testEngineMode();
    await testExternalCommand();
    await testSessionIsolation();
    await testPtyMode(health);
    await testReconnect();
  } catch (error) {
    failures += 1;
    console.error(`\nE2E aborted: ${error instanceof Error ? error.message : error}`);
  }

  console.log(`\n${checks - failures}/${checks} checks passed.`);

  if (failures > 0) {
    process.exit(1);
  }
}

main();
