import { createHash } from 'node:crypto';
import { createServer as createHttpServer, type Server as HttpServer } from 'node:http';
import net from 'node:net';
import type { Duplex } from 'node:stream';
import { fileURLToPath } from 'node:url';
import { afterEach, describe, expect, it } from 'vitest';
import { createLogger, createServer, type ViteDevServer } from 'vite';
import {
  backendProxyLogGuard,
  createApiProxy,
  createProxyGuardState,
} from '../../vite.config';

const ROOT = fileURLToPath(new URL('../..', import.meta.url));

const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

async function freePort(): Promise<number> {
  const server = net.createServer();
  await new Promise<void>((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });

  const address = server.address();
  const port = typeof address === 'object' && address ? address.port : 0;
  await new Promise<void>((resolve) => server.close(() => resolve()));
  return port;
}

interface ProxyHarness {
  server: ViteDevServer;
  url: string;
  warnings: string[];
  errors: string[];
}

const openServers: ViteDevServer[] = [];

async function startProxy(target: string): Promise<ProxyHarness> {
  const port = await freePort();
  const warnings: string[] = [];
  const errors: string[] = [];
  const logger = createLogger('silent');

  logger.warn = ((message: unknown) => {
    warnings.push(String(message));
  }) as typeof logger.warn;
  logger.error = ((...args: unknown[]) => {
    errors.push(
      args
        .map((argument) =>
          argument instanceof Error ? (argument.stack ?? argument.message) : String(argument),
        )
        .join(' '),
    );
  }) as typeof logger.error;

  const guard = createProxyGuardState();
  const server = await createServer({
    configFile: false,
    root: ROOT,
    customLogger: logger,
    logLevel: 'silent',
    plugins: [backendProxyLogGuard(guard)],
    server: {
      host: '127.0.0.1',
      port,
      strictPort: true,
      proxy: {
        '/api': createApiProxy(target, guard),
      },
    },
  });

  await server.listen();
  openServers.push(server);

  return {
    server,
    url: `http://127.0.0.1:${port}`,
    warnings,
    errors,
  };
}

function proxyErrors(errors: string[]): string[] {
  return errors.filter((entry) => /proxy (socket )?error/.test(entry));
}

function backendWarnings(warnings: string[]): string[] {
  return warnings.filter((entry) => entry.includes('backend not reachable'));
}

interface FakeBackend {
  server: HttpServer;
  port: number;
  sockets: Set<Duplex>;
}

async function startFakeBackend(): Promise<FakeBackend> {
  const sockets = new Set<Duplex>();
  const server = createHttpServer();

  server.on('upgrade', (request, socket) => {
    const key = request.headers['sec-websocket-key'];
    const accept = createHash('sha1')
      .update(`${key}258EAFA5-E914-47DA-95CA-C5AB0DC85B11`)
      .digest('base64');

    socket.write(
      'HTTP/1.1 101 Switching Protocols\r\n' +
        'Upgrade: websocket\r\n' +
        'Connection: Upgrade\r\n' +
        `Sec-WebSocket-Accept: ${accept}\r\n\r\n`,
    );
    sockets.add(socket);
    socket.on('close', () => sockets.delete(socket));
    socket.on('error', () => sockets.delete(socket));
  });

  await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
  const address = server.address();
  const port = typeof address === 'object' && address ? address.port : 0;

  return { server, port, sockets };
}

afterEach(async () => {
  await Promise.all(openServers.splice(0).map((server) => server.close()));
});

describe('Vite API proxy offline handling', () => {
  it('returns 503 JSON and warns once for repeated offline requests', async () => {
    const target = `http://127.0.0.1:${await freePort()}`;
    const harness = await startProxy(target);

    for (let attempt = 0; attempt < 3; attempt += 1) {
      const response = await fetch(`${harness.url}/api/auth/status`);

      expect(response.status).toBe(503);
      await expect(response.json()).resolves.toMatchObject({
        error: 'backend_unavailable',
      });
    }

    expect(backendWarnings(harness.warnings)).toHaveLength(1);
    expect(proxyErrors(harness.errors)).toEqual([]);
  }, 20_000);

  it('closes a reset WebSocket without logging a proxy stack trace', async () => {
    const backend = await startFakeBackend();
    const harness = await startProxy(`http://127.0.0.1:${backend.port}`);

    try {
      const socket = new WebSocket(`ws://127.0.0.1:${new URL(harness.url).port}/api/v1/pty`);
      socket.addEventListener('error', () => {
        // The reset is the behavior under test.
      });

      await new Promise<void>((resolve, reject) => {
        const timer = setTimeout(() => reject(new Error('WebSocket did not open')), 5_000);
        socket.addEventListener(
          'open',
          () => {
            clearTimeout(timer);
            resolve();
          },
          { once: true },
        );
      });

      // Simulate the backend vanishing mid-session, as when the dev server is
      // stopped while a terminal tab is attached.
      for (const connection of backend.sockets) {
        connection.destroy();
      }
      await new Promise<void>((resolve) => backend.server.close(() => resolve()));
      await delay(500);

      expect(proxyErrors(harness.errors)).toEqual([]);
      // http-proxy may surface a reset as a clean close instead of an error, so
      // the guarantee is "no stack trace and at most one warning", never a
      // warning per socket.
      expect(backendWarnings(harness.warnings).length).toBeLessThanOrEqual(1);

      socket.close();
    } finally {
      for (const connection of backend.sockets) {
        connection.destroy();
      }
      if (backend.server.listening) {
        await new Promise<void>((resolve) => backend.server.close(() => resolve()));
      }
    }
  }, 20_000);
});
