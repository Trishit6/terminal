/**
 * KERNEXT version shared with the Rust workspace.
 *
 * The value is injected at build time from `Cargo.toml` (see
 * `vite.config.ts`), so the website, the desktop app, `version`, and the
 * release artifacts always agree.
 */
export const KERNEXT_VERSION: string = __KERNEXT_VERSION__;

/** Human-readable product version line, e.g. `KERNEXT v0.1.0`. */
export function versionLine(): string {
  return `KERNEXT v${KERNEXT_VERSION}`;
}
