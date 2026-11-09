/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_KERNEXT_SERVER?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}

interface Window {
  __TAURI_INTERNALS__?: unknown;
  __KERNEXT_SERVER__?: string;
}
