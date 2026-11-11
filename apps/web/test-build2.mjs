import { build } from 'vite';
import path from 'node:path';
import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';

await build({
  root: path.resolve('.'),
  configFile: path.resolve('./vite.config.ts'),
  mode: 'production',
  build: { 
    outDir: 'dist2', 
    sourcemap: false, 
    emptyOutDir: true,
    rollupOptions: {
      input: {
        main: path.resolve('./src/main.tsx')
      }
    }
  },
  plugins: [tailwindcss(), react()],
});
console.log('Build succeeded!');
