#!/usr/bin/env node
/**
 * Browser-level acceptance check for the KERNEXT web terminal.
 *
 *   node scripts/e2e-browser.mjs [url] [--chrome <path>] [--offline]
 *
 * Defaults to the Vite dev server (`http://127.0.0.1:5173/terminal`), which
 * exercises the same `/api` HTTP + WebSocket proxy path a browser uses.
 * `--offline` instead expects no backend: the page must explain the setup
 * state without showing a raw HTTP 500 error.
 *
 * The script launches an installed Chrome/Edge in headless mode with the
 * DevTools Protocol, opens the terminal page, and verifies that:
 *
 *   - no API request fails during normal operation (no HTTP 500)
 *   - the workspace reaches the connected state on its own
 *   - a typed command runs and its real output appears
 *   - the footer reports a live session, not fabricated metadata
 *
 * It uses the browser that is already installed; no npm dependency is added.
 * Exit code 0 means every check passed.
 */
import { spawn } from 'node:child_process';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const args = process.argv.slice(2);
const chromeFlag = args.indexOf('--chrome');
const explicitChrome = chromeFlag === -1 ? null : args[chromeFlag + 1];
const offline = args.includes('--offline');
const target = args.find(
  (value, index) => !value.startsWith('--') && (chromeFlag === -1 || index !== chromeFlag + 1),
) ?? 'http://127.0.0.1:5173/terminal';

const CANDIDATES = [
  explicitChrome,
  process.env.CHROME_PATH,
  'C:/Program Files/Google/Chrome/Application/chrome.exe',
  'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe',
  '/usr/bin/google-chrome',
  '/usr/bin/chromium',
  '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
].filter(Boolean);

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

function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function findChrome() {
  const { access } = await import('node:fs/promises');

  for (const candidate of CANDIDATES) {
    try {
      await access(candidate);
      return candidate;
    } catch {
      // Try the next candidate.
    }
  }

  throw new Error('no Chrome/Chromium instance found; pass --chrome <path>');
}

async function launchChrome() {
  const executable = await findChrome();
  const profile = mkdtempSync(join(tmpdir(), 'kernext-browser-'));
  const port = 9333 + Math.floor(Math.random() * 200);
  const child = spawn(
    executable,
    [
      '--headless=new',
      '--disable-gpu',
      '--no-first-run',
      '--no-default-browser-check',
      `--remote-debugging-port=${port}`,
      `--user-data-dir=${profile}`,
      'about:blank',
    ],
    { stdio: 'ignore' },
  );

  let version = null;

  for (let attempt = 0; attempt < 50; attempt += 1) {
    try {
      const response = await fetch(`http://127.0.0.1:${port}/json/version`);
      version = await response.json();
      break;
    } catch {
      await delay(200);
    }
  }

  if (!version) {
    child.kill();
    throw new Error('Chrome did not expose the DevTools endpoint');
  }

  return {
    port,
    child,
    close() {
      child.kill();
      try {
        rmSync(profile, { recursive: true, force: true });
      } catch {
        // Chrome can still hold a lock file for a moment on Windows.
      }
    },
  };
}

async function openTarget(port, url) {
  const response = await fetch(
    `http://127.0.0.1:${port}/json/new?${encodeURIComponent(url)}`,
    { method: 'PUT' },
  );
  return response.json();
}

function cdp(webSocketUrl) {
  const socket = new WebSocket(webSocketUrl);
  const pending = new Map();
  const events = [];
  let nextId = 1;

  const ready = new Promise((resolve, reject) => {
    socket.addEventListener('open', () => resolve(), { once: true });
    socket.addEventListener('error', () => reject(new Error('DevTools socket failed')), {
      once: true,
    });
  });

  socket.addEventListener('message', (event) => {
    const message = JSON.parse(event.data);

    if (message.id && pending.has(message.id)) {
      const { resolve, reject } = pending.get(message.id);
      pending.delete(message.id);

      if (message.error) {
        reject(new Error(message.error.message));
      } else {
        resolve(message.result);
      }

      return;
    }

    events.push(message);
  });

  return {
    ready,
    events,
    send(method, params = {}) {
      const id = nextId;
      nextId += 1;

      return new Promise((resolve, reject) => {
        pending.set(id, { resolve, reject });
        socket.send(JSON.stringify({ id, method, params }));
      });
    },
    close() {
      socket.close();
    },
  };
}

async function evaluate(client, expression) {
  const result = await client.send('Runtime.evaluate', {
    expression,
    awaitPromise: true,
    returnByValue: true,
  });

  return result.result?.value;
}

async function main() {
  console.log(`KERNEXT browser E2E — ${target}`);

  const chrome = await launchChrome();
  let client = null;

  try {
    const page = await openTarget(chrome.port, 'about:blank');
    client = cdp(page.webSocketDebuggerUrl);
    await client.ready;

    await client.send('Runtime.enable');
    await client.send('Network.enable');
    await client.send('Log.enable');
    await client.send('Page.enable');
    await client.send('Page.navigate', { url: target });
    await delay(6000);

    const failuresSeen = client.events
      .filter(
        (event) =>
          event.method === 'Network.responseReceived' &&
          event.params.response.status >= 400,
      )
      .map(
        (event) =>
          `${event.params.response.status} ${event.params.response.url}`,
      );

    const consoleErrors = client.events
      .filter((event) => {
        if (event.method === 'Runtime.exceptionThrown') {
          return true;
        }

        // With no backend, the proxy's failed health request is expected.
        // Only an application-level console error is a regression there.
        return (
          !offline &&
          event.method === 'Log.entryAdded' &&
          event.params.entry.level === 'error'
        );
      })
      .map((event) => JSON.stringify(event.params).slice(0, 300));

    if (!offline) {
      ok(
        'no failed HTTP requests after load',
        failuresSeen.length === 0,
        failuresSeen.join('; '),
      );
    }
    ok('no uncaught browser errors', consoleErrors.length === 0, consoleErrors.join('; '));

    const state = await evaluate(
      client,
      `(() => {
        const text = document.body.innerText;
        return {
          text,
          hasTerminal: !!document.querySelector('[aria-label="KERNEXT terminal"]'),
          hasBackendNotice: text.includes('server isn') || text.includes('not reachable'),
          hasErrorBanner: text.includes('HTTP 500') || text.includes('request_failed'),
          connected: text.includes('Connected'),
        };
      })()`,
    );

    ok('terminal surface rendered', state?.hasTerminal === true);
    ok('no HTTP 500 banner', state?.hasErrorBanner === false, state?.text?.slice(0, 200));

    if (offline) {
      ok(
        'offline setup notice is shown',
        state?.hasBackendNotice === true,
        state?.text?.slice(0, 300),
      );
      ok('the UI does not claim to be connected', state?.connected === false);
      console.log(`\n${checks - failures}/${checks} checks passed.`);

      if (failures > 0) {
        process.exitCode = 1;
      }

      return;
    }

    ok(
      'workspace reached the connected state',
      state?.connected === true,
      state?.text?.slice(0, 300),
    );

    await evaluate(
      client,
      `document.querySelector('[aria-label="Terminal keyboard input"]')?.focus(),
       document.querySelector('[aria-label="KERNEXT terminal"]')?.click(), true`,
    );
    await delay(300);
    await client.send('Input.insertText', { text: 'echo BROWSER_KERNEXT' });
    await client.send('Input.dispatchKeyEvent', {
      type: 'keyDown',
      key: 'Enter',
      code: 'Enter',
      windowsVirtualKeyCode: 13,
    });
    await client.send('Input.dispatchKeyEvent', {
      type: 'keyUp',
      key: 'Enter',
      code: 'Enter',
      windowsVirtualKeyCode: 13,
    });
    await delay(2500);

    const command = await evaluate(
      client,
      `document.body.innerText.includes('BROWSER_KERNEXT')`,
    );

    ok('typed command output appears in the terminal', command === true);

    const ptyFooter = await evaluate(
      client,
      `document.body.innerText.includes('pty process') &&
       /\\d+ × \\d+/.test(document.body.innerText)`,
    );

    ok('footer reports PTY mode and dimensions', ptyFooter === true);

    // Switch to the engine transport and prove the HTTP session path works in
    // the same page, including the footer's live session state.
    const switched = await evaluate(
      client,
      `(() => {
        const button = [...document.querySelectorAll('button')]
          .find((element) => element.textContent.trim() === 'Engine');
        button?.click();
        return !!button;
      })()`,
    );

    ok('engine mode control exists', switched === true);
    await delay(3500);

    const engineConnected = await evaluate(
      client,
      `document.body.innerText.includes('Connected') &&
       document.body.innerText.includes('engine session')`,
    );

    ok('engine mode connects', engineConnected === true);

    await evaluate(
      client,
      `document.querySelector('[aria-label="Terminal keyboard input"]')?.focus(), true`,
    );
    await delay(200);
    await client.send('Input.insertText', { text: 'echo ENGINE_KERNEXT' });
    await client.send('Input.dispatchKeyEvent', {
      type: 'keyDown',
      key: 'Enter',
      code: 'Enter',
      windowsVirtualKeyCode: 13,
    });
    await client.send('Input.dispatchKeyEvent', {
      type: 'keyUp',
      key: 'Enter',
      code: 'Enter',
      windowsVirtualKeyCode: 13,
    });
    await delay(2000);

    const engineResult = await evaluate(
      client,
      `(() => {
        const text = document.body.innerText;
        return {
          output: text.includes('ENGINE_KERNEXT'),
          session: /session\\s+\\d+\\s*·\\s*running/.test(text),
          cwd: text.includes('terminal') || text.includes('Projects'),
        };
      })()`,
    );

    ok('engine command output appears', engineResult?.output === true);
    ok('footer reports a real running session', engineResult?.session === true);
    ok('footer reports the session directory', engineResult?.cwd === true);
  } finally {
    client?.close();
    chrome.close();
  }

  console.log(`\n${checks - failures}/${checks} checks passed.`);

  if (failures > 0) {
    process.exit(1);
  }
}

main().catch((error) => {
  console.error(`\nBrowser E2E failed: ${error instanceof Error ? error.message : error}`);
  process.exit(1);
});
