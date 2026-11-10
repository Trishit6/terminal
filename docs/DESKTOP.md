# KERNEXT Desktop

The native desktop application. It is a Tauri 2 app (`apps/desktop`, crate
`shell-desktop`, binary `kernext-desktop`) that reuses the `apps/web` React
front end for its user interface while executing everything through a local
Rust runtime. It never routes commands through the public web server and never
simulates the shell in JavaScript.

## Prerequisites

- Rust 1.85+ (Rust 2024 edition) with a working platform linker.
- Node.js and pnpm 11 (`corepack` can supply the pinned `pnpm@11.25.0`).
- On Windows: the WebView2 runtime (preinstalled on Windows 10/11).
- On Linux: webkit2gtk and related system libraries; on macOS: Xcode command
  line tools.

## Build and run

From the repository root:

```sh
pnpm install --frozen-lockfile
pnpm desktop:dev      # tauri dev  (vite dev server + debug window)
pnpm desktop:build    # tauri build (production bundle)
```

From `apps/desktop` directly:

```sh
pnpm exec tauri build --debug --no-bundle   # skip installer bundling
```

Tauri configuration lives in `apps/desktop/src-tauri/tauri.conf.json`:

- `build.devUrl` is `http://127.0.0.1:5173/desktop` in dev.
- `build.frontendDist` is `../../web/dist` — it resolves relative to the
  `src-tauri` directory that contains the config, so the web app at
  `apps/web` is embedded into the desktop binary.
- `app.windows[0]` uses `decorations: false`; the React shell provides the
  custom title bar through `data-tauri-drag-region`.
- CSP restricts script/style sources; the IPC transport uses
  `connect-src ipc: http://ipc.localhost`.
- Capabilities (`capabilities/default.json`) only grant `core:default`,
  minimize/toggle-maximize/close window permissions, and `opener:default`.

## Packaging and release

`pnpm desktop:build` (equivalently `tauri build` from `apps/desktop`)
performs, in order:

1. `beforeBuildCommand` — `pnpm --filter @kernext/web build` regenerates
   `apps/web/dist`. The bundle embeds those assets through
   `frontendDist`, so a release-mode build always carries the front end it
   was built with; commit no `dist` output, it is generated.
2. A release-profile compile of `shell-desktop` (`kernext-desktop`).
   Plain `cargo build --workspace --release` compiles the desktop crate in
   dev/development mode (the `custom-protocol` feature is only enabled by the
   Tauri CLI), so use `tauri build` for distributable binaries.
3. Installer/bundle generation per platform through the `bundle.targets: all`
   setting:
   - **Windows**: NSIS installer (`.exe`) and WiX MSI, plus the raw
     `kernext-desktop.exe`.
   - **macOS**: `.app` bundle and `.dmg` disk image (Apple Silicon or Intel
     depending on the host).
   - **Linux**: `.deb`, `.rpm`, and `AppImage`.

Bundles land under the cargo target directory's `release/bundle/` path (the
workspace routes target output to an external ext4 directory through
`.cargo/config.toml`; on the Windows checkout that is
`D:\home\trishit\.cache\terminal-target\release\bundle`).

The release workflow (`.github/workflows/release.yaml`) builds desktop
bundles for Windows, Linux, and macOS (both architectures), renames each
artifact with a platform prefix, and attaches a SHA-256 checksum to every
file. It only ever creates a **draft** GitHub Release via manual
`workflow_dispatch`; publishing a release is always a deliberate human step.

Rules to keep releases working:

- The Rust `tauri` crate and npm `@tauri-apps/api` must stay on the same
  major/minor version.
- Keep the MinGW manifest shadowing in `build.rs` (next section) — a GNU
  build without it dies at startup with `STATUS_ENTRYPOINT_NOT_FOUND`.
- macOS bundling must run on a macOS host; the CI job uses GitHub's runners.
- Desktop commands execute through the local Rust runtime only — the public
  web server is never involved, so a desktop bundle has no server
  dependency.

## Architecture

```text
kernext-desktop (Tauri 2 window)
  -> apps/web React front end at route /desktop
  -> tauri invoke IPC
  -> 20 local Tauri commands
       -> engine sessions    shell_server::AppState::local()
       -> PTY sessions       this same exe + KERNEXT_DESKTOP_WORKER=1
                             runs shell_cli::worker::run_embedded()
       -> filesystem         list / create / rename / delete / open
       -> command catalog    registry metadata + real PATH checks
       -> settings           validated TOML in the app config dir
```

- **Engine sessions** execute each command through the same
  `shell-builtins` dispatcher the CLI uses, against a cloned
  `shell-runtime::Session`.
- **PTY sessions** spawn the desktop executable itself with
  `KERNEXT_DESKTOP_WORKER=1`, which runs the embedded CLI worker instead of
  the GUI. ConPTY (Windows) or `openpty` (Unix) provides the pseudoterminal,
  so the desktop terminal is a real interactive shell with stdin, streaming
  stdout/stderr, resize, and Ctrl+C. There is no sidecar binary to ship.
- The frontend terminal stack is transport-aware:
  `apps/web/src/lib/terminal/socket.ts` abstracts a `TerminalSocket`, with
  `WebSocketTerminalSocket` for the web server and a Tauri invoke adapter
  for the desktop. `use-terminal-connection` picks the transport from
  `isTauri()`.

### Tauri commands

`desktop_health`, `desktop_dashboard`, `desktop_create_session`,
`desktop_execute_session`, `desktop_delete_session`,
`desktop_list_sessions`, `desktop_create_pty_session`, `desktop_pty_write`,
`desktop_pty_resize`, `desktop_pty_interrupt`, `desktop_pty_close`,
`desktop_fs_list`, `desktop_fs_create_directory`, `desktop_fs_rename`,
`desktop_fs_delete`, `desktop_fs_open`, `desktop_fs_session_directory`,
`desktop_command_catalog`, `desktop_settings_get`,
`desktop_settings_save`.

Invoke arguments are camelCase on the TypeScript side and map to snake_case
Rust parameters (Tauri's default argument case). Nested struct fields stay
snake_case — for example a session creation request is passed as
`{ workingDirectory }` from TypeScript and read as `working_directory` in
the `CreateSessionRequest` struct fields.

## Desktop settings

Settings are stored as TOML in the Tauri app config directory (for example
`%APPDATA%\dev.kernext.desktop\` on Windows) and validated on load:

```toml
theme = "midnight"                 # midnight | light | system
terminal_font_size = 14
terminal_font_family = "monospace"
terminal_cursor_style = "block"    # block | bar | underline
default_working_directory = ""
history_enabled = true
history_size = 1000
startup_animation = true
reduce_motion = false
accent_color = "bright-cyan"
```

- `theme` selects `midnight`, `light`, or `system`; `midnight` and a
  dark-preferring `system` apply the `dark` class on the document root.
- `terminal_font_size` must be 6–32, `history_size` must be greater than zero.
- Colors accept ANSI names, `colorN` (0–255), or `#RGB`/`#RRGGBB` and are
  validated at load time.
- The Settings screen can reset everything to defaults.

## Windows MinGW manifest handling (`build.rs`)

Some MinGW-w64 distributions (WinLibs, current MSYS2) link a
`default-manifest.o` into every executable through the GCC endfile specs
(`%{!shared:%:if-exists(default-manifest.o%s)}`). That default manifest has
no Common Controls v6 side-by-side dependency, and GNU ld keeps it over
Tauri's application manifest when the two `.rsrc` manifests collide
(`.rsrc merge failure: multiple non-default manifests`). The binary then
fails at startup with `STATUS_ENTRYPOINT_NOT_FOUND` (`0xC0000139`) because
`TaskDialogIndirect` — imported by `tao` — does not exist in the v5
`comctl32.dll` that Windows loads without the manifest dependency.

`apps/desktop/src-tauri/build.rs` fixes this on `*-pc-windows-gnu`:

1. `tauri_build` runs with `WindowsAttributes::new_without_app_manifest()` so
   only one manifest is linked.
2. The script writes a manifest carrying the Common Controls v6 dependency
   (plus `longPathAware` and `asInvoker`, which the MinGW default provided)
   and compiles it with `windres` into `default-manifest.o` inside the build
   output directory.
3. It prints `cargo:rustc-link-arg=-B<out-dir>\`, which prepends that
   directory to the GCC startfile search path. The endfile spec's
   `%s` search resolves `-B` directories first, so the linker picks this
   object instead of the toolchain's, and the manifest survives linking.

The manifest applies to every linked artifact of the package — binaries and
test binaries alike — so `cargo test -p shell-desktop` produces executables
that start correctly on these toolchains. If no `windres` can be found, the
build script emits a `cargo:warning` and falls back to the previous behavior
instead of failing the build. MSVC and non-Windows targets keep Tauri's
default manifest handling untouched.

A second Windows-GNU consideration: the `WebView2Loader.dll` imported by
`webview2-com` is copied to the Cargo target directory root by its build
script. Cargo adds that directory to `PATH` when running test binaries, so
`cargo test` works; launching a test executable directly from
`target/debug/deps/` requires the DLL to be copied next to it.
