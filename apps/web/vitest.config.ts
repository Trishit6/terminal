import { fileURLToPath, URL } from 'node:url';
import { defineConfig } from 'vitest/config';
import { workspaceVersion } from './vite.config';

/**
 * Unit tests for the web terminal logic layer.
 *
 * The tests run in a plain Node environment: the ANSI parser, screen buffer,
 * keyboard mapping, palette, and engine are deliberately free of DOM access
 * so they stay testable without a browser. DOM-bound components
 * (TerminalScreen, TerminalWorkspace) are covered by typecheck, build, and
 * manual verification instead.
 */
export default defineConfig({
  define: {
    __KERNEXT_VERSION__: JSON.stringify(workspaceVersion()),
  },
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  test: {
    environment: 'node',
    include: ['src/**/*.test.ts'],
  },
});
