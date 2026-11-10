/// <reference types="vite/client" />

/** KERNEXT version read from the workspace Cargo.toml at build time. */
declare const __KERNEXT_VERSION__: string;

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
