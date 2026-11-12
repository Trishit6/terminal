import { fileURLToPath, URL } from 'node:url';
import { readFileSync } from 'node:fs';
import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import { defineConfig, type Plugin, type ProxyOptions } from 'vite';

/**
 * The website and desktop app must report the same version as the Rust
 * engine, so the number is read from the workspace Cargo.toml at build time
 * instead of being duplicated here.
 */
export function workspaceVersion(): string {
  const manifest = readFileSync(fileURLToPath(new URL('../../Cargo.toml', import.meta.url)), 'utf8');
  const match = manifest.match(/^\s*version\s*=\s*"([^"]+)"/m);
  if (!match) {
    throw new Error('KERNEXT workspace version could not be read from Cargo.toml');
  }
  return match[1]!;
}

const API_TARGET = 'http://127.0.0.1:7878';

/**
 * Dev-only state for the API proxy. Requests legitimately fail while the Rust
 * backend is stopped (for example during a front-end-only edit session), so
 * one outage must produce one warning and a clean 503 instead of a stack trace
 * per request.
 */
export interface BackendProxyGuardState {
  down: boolean;
  logger: null | { warn: (message: string) => void };
}

export function createProxyGuardState(): BackendProxyGuardState {
  return { down: false, logger: null };
}

function reportBackendDown(guard: BackendProxyGuardState, target: string): void {
  if (guard.down) {
    return;
  }

  guard.down = true;
  guard.logger?.warn(
    `[kernext] backend not reachable at ${target}; API requests return 503. Start the full stack with \`npm run dev\`.`,
  );
}

export function createApiProxy(
  target: string = API_TARGET,
  guard: BackendProxyGuardState = createProxyGuardState(),
) {
  return {
    target,
    changeOrigin: false,
    ws: true,
    configure(proxy: Parameters<NonNullable<ProxyOptions['configure']>>[0]) {
      // Registered before Vite's own error handler, so this response wins and
      // Vite's handler only sees an already-finished response.
      proxy.on('error', (error: NodeJS.ErrnoException, _request, response) => {
        if (error.code !== 'ECONNREFUSED' && error.code !== 'ECONNRESET') {
          return;
        }

        reportBackendDown(guard, target);

        if (response && 'req' in response) {
          if (!response.headersSent && !response.writableEnded) {
            response.writeHead(503, {
              'Content-Type': 'application/json; charset=utf-8',
              'Cache-Control': 'no-store',
            });
            response.end(
              JSON.stringify({
                error: 'backend_unavailable',
                message: `The KERNEXT backend is not running at ${target}. Start the full stack with \`npm run dev\`.`,
              }),
            );
          }
          return;
        }

        // A WebSocket upgrade that never completed, or one whose backend
        // disappeared, has no HTTP body to answer: close the client socket the
        // same way Vite does. Writing HTTP bytes onto an upgraded stream would
        // corrupt it.
        response?.destroy();
      });

      proxy.on('proxyRes', () => {
        guard.down = false;
      });
      proxy.on('open', () => {
        guard.down = false;
      });
    },
  };
}

/**
 * Vite logs every proxy failure as an error with a full stack trace. During a
 * backend outage that turns one page load into a wall of ECONNREFUSED noise.
 * Keep genuine proxy bugs visible, but drop the expected connection failures:
 * `apiProxy` has already reported the outage once and answered with 503.
 */
export function backendProxyLogGuard(
  guard: BackendProxyGuardState = createProxyGuardState(),
): Plugin {
  return {
    name: 'kernext-backend-proxy-log-guard',
    apply: 'serve',
    configResolved(config) {
      guard.logger = config.logger;

      const originalError = config.logger.error.bind(config.logger);
      config.logger.error = ((...args: Parameters<typeof originalError>) => {
        const text = args
          .map((argument) => (typeof argument === 'string' ? argument : ''))
          .join(' ');

        if (
          /proxy (socket )?error/.test(text) &&
          /ECONNREFUSED|ECONNRESET/.test(text)
        ) {
          return;
        }

        originalError(...args);
      }) as typeof config.logger.error;
    },
  };
}

const defaultProxyGuard = createProxyGuardState();

export default defineConfig({
  plugins: [react(), tailwindcss(), backendProxyLogGuard(defaultProxyGuard)],
  define: {
    __KERNEXT_VERSION__: JSON.stringify(workspaceVersion()),
  },
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  // `@radix-ui/react-tabs` otherwise fails on-demand optimization in the
  // workspace's linked node_modules and the docs/account routes receive a
  // stale 504 chunk. Pre-bundle it with the other dependencies instead.
  optimizeDeps: {
    include: ['@radix-ui/react-tabs', 'three', '@react-three/fiber', '@react-three/drei'],
    // Vite 8's Windows dependency cache can leave only `deps_temp_*`
    // directories, after which on-demand chunks such as the tabs bundle
    // return 504. Rebuild the in-memory bundle at every dev start.
    force: true,
  // Three.js and R3F deps are pre-bundled to avoid 504 errors on dev start
  // and ensure the hero 3D scene is available immediately.
  },
  // Environment files are shared with the server and live at the workspace
  // root (see `.env.example`): the web client only reads `VITE_*` variables,
  // so server secrets stay server-side.
  envDir: fileURLToPath(new URL('../..', import.meta.url)),
  // Disable Vite's console forwarding to avoid __SERVER_FORWARD_CONSOLE__
  // being injected as `undefined` when the option is not explicitly set.
  server: {
    host: '127.0.0.1',
    port: 5175,
    forwardConsole: false,
    proxy: {
      '/api': createApiProxy(API_TARGET, defaultProxyGuard),
    },
  },
  preview: {
    host: '127.0.0.1',
    port: 4173,
    proxy: {
      '/api': createApiProxy(API_TARGET, defaultProxyGuard),
    },
  },
  build: {
    outDir: 'dist',
    // Production source maps are not published: they would ship readable
    // source to every visitor. Local debugging keeps dev-mode maps.
    sourcemap: false,
    target: 'es2022',
    // Workaround for Vite 8 / rolldown EISDIR bug when using index.html as entry.
    // Using src/main.tsx as the entry point avoids the HTML plugin issue.
    rollupOptions: {
      input: {
        main: './src/main.tsx',
      },
    },
  },
});
