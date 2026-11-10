import { fileURLToPath, URL } from 'node:url';
import { readFileSync } from 'node:fs';
import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

// The website and desktop app must report the same version as the Rust
// engine, so the number is read from the workspace Cargo.toml at build time
// instead of being duplicated here.
export function workspaceVersion(): string {
  const manifest = readFileSync(fileURLToPath(new URL('../../Cargo.toml', import.meta.url)), 'utf8');
  const match = manifest.match(/^\s*version\s*=\s*"([^"]+)"/m);
  if (!match) {
    throw new Error('KERNEXT workspace version could not be read from Cargo.toml');
  }
  return match[1]!;
}

export default defineConfig({
  plugins: [react(), tailwindcss()],
  define: {
    __KERNEXT_VERSION__: JSON.stringify(workspaceVersion()),
  },
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  server: {
    host: '127.0.0.1',
    port: 5173,
    strictPort: true,
    proxy: {
      '/api': {
        target: 'http://127.0.0.1:7878',
        changeOrigin: false,
        ws: true,
      },
    },
  },
  preview: {
    host: '127.0.0.1',
    port: 4173,
    strictPort: true,
    proxy: {
      '/api': {
        target: 'http://127.0.0.1:7878',
        changeOrigin: false,
        ws: true,
      },
    },
  },
  build: {
    outDir: 'dist',
    sourcemap: true,
    target: 'es2022',
  },
});
