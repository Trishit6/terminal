# KERNEXT Project Status

Status as of Phase 6 (product integration): the completed CLI, Web, and
Desktop surfaces share one Rust engine, and the product-level work — public
website, cross-interface consistency, documentation, CI/CD, and release
packaging — is in place. This document separates what is **implemented**, what
is **tested**, and what remains **unverified**. It is the authoritative
completion snapshot; `README.md` and `docs/ARCHITECTURE.md` describe how the
system works, while this file records how far verification reached.

Verification host: Windows 11, Rust 1.99.1 (`x86_64-pc-windows-gnu`, WinLibs
MinGW-w64), Node.js + npm 12.0.2.

## Verification results

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo check --workspace --all-targets` | clean |
| `cargo test --workspace` | 470 passed, 0 failed |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `npm ci` | ok |
| `npm run typecheck` | ok |
| `npm test` (Vitest) | 55 passed in 7 files |
| `npm run build` | ok (chunk-size warning only) |
| `npm run build -w @kernext/desktop -- --debug --no-bundle` | ok, application builds |
| `npm run desktop:build` (release bundle) | ok — MSI 9.2 MiB + NSIS 6.3 MiB; release binary launch smoke test ok |
| Desktop launch smoke test | window `KERNEXT` opens and stays alive (debug in Phase 5, release in Phase 6) |
| npm → migration complete | root `package.json` declares workspaces; `package-lock.json` committed; `npm ci`/`npm run typecheck`/`npm test`/`npm run build` all pass; `npm run desktop:dev` and `npm run desktop:build` delegate to `apps/desktop/package.json` scripts with `npm --prefix ../web run …` |

Test counts per suite (Windows host):

| Suite | Tests |
| --- | --- |
| `shell-core` unit | 130 |
| `shell-core` lexer + parser + doctests | 2 + 17 + 2 |
| `shell-builtins` unit | 47 |
| `shell-builtins` builtins | 40 |
| `shell-builtins` external | 15 |
| `shell-builtins` external_tools | 2 |
| `shell-builtins` language | 16 |
| `shell-builtins` pipeline | 30 |
| `shell-builtins` pty (Windows) | 4 |
| `shell-builtins` windows scripts | 20 |
| `shell-runtime` unit | 34 |
| `shell-config` unit | 14 |
| `shell-line-editor` unit | 24 |
| `shell-api` unit | 2 |
| `shell-cli` unit + repl | 24 + 13 |
| `shell-server` unit + api + consistency + ws | 5 + 5 + 7 + 1 |
| `shell-desktop` unit | 16 |
| **Total** | **470** |

The Unix baseline differs only in platform-specific suites (Unix uses
signal/abort tests instead of the Windows launcher and ConPTY suites).

## Implemented

### KERNEXT CLI (`kernext`)

- Own lexer, parser, AST, evaluator; 56 built-in commands (including `host`,
  `sysinfo`, and the registered `[` alias of `test`); direct external process
  execution (no `sh -c`/`cmd /C` for KERNEXT syntax).
- Pipelines over real OS pipes; `<`, `>`, `>>`, `2>`, `2>>` redirections;
  `;`, groups, `&&`/`||`.
- Variables, expansion (`$name`, `${name}`, `$?`, `$$`), `export`/`unset`/
  `env`/`set`, functions, `if`/`else`, `while`, `for`, `return`/`break`/
  `continue`.
- Background jobs (`&`) with a job table and `jobs`/`wait`/`kill`/`fg`/`bg`.
- Startup presentation (animated/plain/quiet), genuine `clear`/Ctrl+L,
  persistent history, tab completion, TOML configuration.

### KERNEXT Web (`apps/shell-server`, `apps/web`)

- Local Axum server: `GET /api/v1/health`, `GET|POST /api/v1/sessions`,
  `GET|DELETE /api/v1/sessions/{id}`, `POST /api/v1/sessions/{id}/execute`,
  `GET /api/v1/sessions/{id}/ws` (engine sessions),
  `GET /api/v1/pty` (real pseudoterminal), plus static serving of the built
  front end with an `index.html` fallback for client-side routes.
- Engine WebSocket sessions execute through the same `Dispatcher` the CLI
  uses; PTY WebSocket relays raw bytes of the real `kernext` binary running on
  ConPTY (Windows) or `openpty` (Unix).
- Security posture: loopback bind by default, optional bearer token
  (`KERNEXT_SERVER_TOKEN`, constant-time comparison), fixed browser-origin
  allowlist, `KERNEXT_MAX_SESSIONS` cap (16), 30-minute idle sweeper, 64 KiB
  command limit. Documented in `docs/SECURITY.md`.
- React 19 + Vite + Tailwind CSS 4 + shadcn/ui + Lucide + Motion +
  TanStack Router front end. **Public website**: Home, Features,
  Documentation, Downloads, About, Contact, and a 404 page — responsive,
  route-level metadata, and a build-time version badge from the workspace
  version (never hardcoded). **Web terminal**: engine mode with the cinematic
  startup animation, metallic K emblem, orbital ring, mountain silhouette, and
  planetary horizon.
- Terminal logic (buffer, engine, ANSI, palette, input) and the Tooltip
  provider contract covered by 55 Vitest
  tests.

### KERNEXT Desktop (`apps/desktop`)

- Tauri 2 application (`shell-desktop`, binary `kernext-desktop`) with custom
  title bar and sidebar: Home (dashboard), Terminal, Command Explorer, File
  Explorer, Sessions, Settings, About KERNEXT.
- Reuses the web front end (`frontendDist ../../web/dist`, route `/desktop`)
  behind a transport-aware terminal (`TerminalSocket` with WebSocket and
  Tauri IPC adapters).
- Local Rust runtime only — 20 Tauri commands covering health, dashboard,
  engine sessions, PTY sessions (write/resize/interrupt/close), filesystem
  (list/create/rename/delete/open), command catalog, and settings. The desktop
  never routes commands through the public web server and never simulates the
  shell in JavaScript.
- Real PTY sessions spawn the desktop executable itself with
  `KERNEXT_DESKTOP_WORKER=1` to run `shell_cli::worker::run_embedded` — no
  sidecar binary.
- Engine sessions reuse `shell_server::AppState::local()`.
- Multi-tab terminal workspace (engine or PTY per tab), terminal font
  family/size and cursor style settings, theme (`midnight`/`light`/`system`),
  default working directory, history, and startup behavior persisted as
  validated TOML in the app config directory.
- Dashboard shows real data (sessions, recent commands, platform); Command
  Explorer lists real registry metadata (all 56 built-ins) and performs real
  `PATH` checks; File Explorer performs real filesystem operations with
  confirmation dialogs and no permission bypass.
- Full icon set generated via `tauri icon` (`.ico`, `.icns`, PNG sizes, iOS and
  Appx assets).
- Windows MinGW manifest handling in `build.rs` (see `DESKTOP.md`) so the
  Common Controls v6 dependency survives linking on `*-pc-windows-gnu`.

### Phase 6 integration work

- **Fixed broken root scripts**: `npm run desktop:dev` / `npm run desktop:build`
  previously ran `pnpm --filter @kernext/desktop tauri …`, which requires a
  package *script* named `tauri`; they now delegate to the package's `dev` /
  `build` scripts and actually run. Verified with a debug build and a full
  release bundle build.
- **Registered the `[` built-in**: the alias was documented and dispatched
  (`"test" | "["` arm, closing-`]` handling) but never present in the registry,
  so `[ 1 = 1 ]` fell through to external resolution and failed. It is now a
  registry entry with tests.
- **`host` and `sysinfo` built-ins** (Phase 6 start): DNS resolution via the
  system resolver and a system/runtime report, with desktop catalog entries.
- **Cross-interface consistency suite** (`apps/shell-server/tests/
  consistency.rs`): help header, `version`, `pwd`/`cd` persistence semantics,
  `ls`, `mkdir`, `echo`, pipelines, redirections, and environment across HTTP
  sessions and the CLI engine. HTTP `cd` persistence was fixed in
  `state.rs` (`directory_overridden`) rather than weakening the test.
- **External-tool compatibility tests** (`external_tools.rs`): `git`, `cargo`,
  `rustc`, `node`, `pnpm`, `python`, `curl`, `code`, and `opencode` execute
  through the full engine; absent tools are skipped, never assumed.
- **Website completion**: full page set (see Web above), honest status badges
  per surface, downloads page linking only to the real GitHub Releases URL.
- **CI workflow** (`ci.yml`): lint (fmt + clippy), web job (typecheck, Vitest,
  build), test matrix on `ubuntu-latest`, `windows-latest`, `macos-latest`
  (Apple Silicon), and `macos-15-intel` (Intel), and a desktop bundling matrix
  with the Linux webkit2gtk dependency set. Actions on Node 24 majors
  (`checkout@v7`, `cache@v6`, `setup-node@v7`, `npm/action-setup@v6`,
  `upload-artifact@v7`, `download-artifact@v7`).
- **Release workflow** (`release.yaml`): `workflow_dispatch`-only, draft-only;
  CLI archives for Windows x64, Linux GNU/musl x86_64, and macOS aarch64/x86_64
  (including `kernext-server`), desktop bundles for all four platforms, a
  SHA-256 checksum for every artifact, a workspace-version guard against
  mislabeled releases, and release creation through the `gh` CLI.
- **Documentation set**: `docs/COMMANDS.md` (56 built-ins, flags verified
  against the implementation), `docs/SECURITY.md`, `docs/DEPLOYMENT.md`,
  `docs/TROUBLESHOOTING.md`, `CONTRIBUTING.md`, extended `docs/DESKTOP.md`
  packaging section, and refreshed `README.md` / `AGENTS.md` / `CLAUDE.md` /
  `RELEASE_NOTES.md` / `knowledge.md`.

## Web terminal HTTP 500 investigation (2026-11-10)

**Root cause.** The backend was healthy. `POST /api/v1/sessions` against Axum
returns HTTP 200 with a real session; 28/28 API-level checks and 12/12
real-browser checks pass through the Vite proxy. The reported
`HTTP 500 (request_failed)` came from the Vite development proxy, not from
Axum: when only `npm run web:dev` was running, Vite answered `/api/v1/health` and
`/api/v1/sessions` with HTTP 500 and an empty/non-JSON body, and the old
client converted that into the generic message. The frontend now distinguishes
a KERNEXT JSON error from a proxy/network failure, probes health before
connecting, and shows a first-run setup notice instead of a red banner. A
single `npm run dev` command starts both the Axum server and Vite.

**Second backend defect found while verifying static hosting.**
`ServeDir::not_found_service(ServeFile::new(index))` served the SPA shell for
client-side routes but forced HTTP 404. `/terminal` therefore looked like a
failed request on refresh. It now uses `ServeDir::fallback`, which keeps the
index response's 200. Covered by
`client_side_routes_serve_the_spa_shell_with_ok`.

**PTY cleanup defect.** The PTY socket handler moved `PtyProcess` into a
blocking wait task, so a browser disconnect could leave `kernext` running until
the child exited on its own. `PtyProcess::try_wait` now lets the socket task
poll the child, and disconnect sends Ctrl+C plus `exit`, then kills and reaps
the process. Verified by starting `sleep 30` over `/api/v1/pty`, observing
`kernext.exe` while connected, and observing no `kernext.exe` after disconnect.

## Tested

- All 470 workspace tests pass on Windows (breakdown above), including the 16
  desktop unit tests and the 18 shell-server tests (unit, api, consistency,
  ws).
- The desktop build script's manifest override is verified by building the
  binary and extracting its embedded manifest: it declares
  `Microsoft.Windows.Common-Controls` 6.0.0.0, and the application launches
  (previously it died with `STATUS_ENTRYPOINT_NOT_FOUND` because
  `TaskDialogIndirect` was missing from comctl32 v5).
- Server API and WebSocket behavior covered by `shell-server` integration
  tests; PTY lifecycle covered by `shell-builtins` ConPTY tests.
- **Web terminal acceptance, real backend:** `node scripts/e2e-terminal.mjs`
  passed 28/28 checks against `http://127.0.0.1:5173` (Vite proxy) and
  `http://127.0.0.1:7878` (Axum). It covers health, session create/delete,
  `help`, `pwd`, `echo KERNEXT`, stderr/exit status, external `node`, clear,
  session isolation, PTY resize, typed command echo, Ctrl+C, disconnect, and
  reconnect.
- **Web terminal acceptance, real browser:** `node scripts/e2e-browser.mjs`
  drives headless Chrome over the DevTools Protocol. It passed 12/12 checks
  against the Vite proxy and against the production bundle served by Axum:
  no failed HTTP responses, no HTTP 500 banner, PTY connection, typed command
  output, Engine-mode switch, engine command output, live session footer, and
  current directory. With the backend stopped, the `--offline` mode passes
  5/5: the setup notice appears, no raw 500 banner appears, and the UI does
  not claim to be connected.
- **SPA deep links:** `/terminal` served by the Axum static fallback returns
  HTTP 200 with the index shell (new API integration test).
- **PTY disconnect cleanup:** with `sleep 30` running, `kernext.exe` was
  present during the session and absent after the WebSocket closed.
- Web and desktop front end: TypeScript typecheck, production build, and the
  55-test Vitest suite pass.
- Launch smoke test: `kernext-desktop.exe` opens a window titled `KERNEXT`,
  keeps running, and terminates cleanly when stopped.
- All nine external developer tools (`git`, `cargo`, `rustc`, `node`, `pnpm`,
  `python`, `curl`, `code`, `opencode`) were executed through the engine on
  this Windows host with exit status 0 (the `external_tools` suite runs them on
  every platform, skipping absent tools).
- Workflow YAML files parse cleanly and every referenced action version exists
  (checked against published releases as of November 2026).

## Unverified

- **CI has run and failed; it has not yet passed.** Run #5 (commit
  `05a4fb2`, 2026-11-10) failed in the `Format and Clippy` job at the Clippy
  step with process exit 101; the test and desktop jobs were skipped. The
  workflow at that commit did not install the Linux WebKitGTK/GTK development
  packages required to compile the Tauri desktop crate, so Clippy could not
  build the workspace on `ubuntu-latest`. The current workflow installs those
  packages in the lint, test, and release check jobs. Job logs required admin
  access, so the exact compiler line could not be downloaded; the failing step
  is recorded in the public job metadata. Do not claim CI is green until a new
  run completes.
- **Release workflow has never been triggered.** It is draft-only and
  `workflow_dispatch`-only; no release run exists.
- **No GitHub Release has been published**; the downloads page links to the
  releases URL, which has no assets yet.
- **Interactive desktop GUI usage.** Typing in the PTY terminal, tab
  creation/switching, resizing, Ctrl+C behavior, `clear`, history, and arrow
  keys inside the desktop app were not exercised interactively in this
  session; only the launch smoke test ran. The same terminal components are
  exercised by automated tests on the web side, but a human pass over the
  desktop UI is still needed.
- **`tauri dev` hot-reload flow** was not run in this session.
- **MSI/NSIS installer *installation*** — the bundles were built, but actually
  installing them was not performed; the release binary itself was launched as
  the smoke test.
- **macOS.** No macOS host is available here; the app cannot be built or
  launched on macOS from this machine, and macOS tool verification
  (`external_tools`) has not run there.
- **Linux desktop/web GUI.** The canonical Linux checkout is not mounted on
  this Windows host; only the Windows builds were verified. Linux verification
  should run from `/run/media/trishit/UBUNTU 26_0/Projects/terminal` when
  mounted.
- **Remote GitHub Actions result after the workflow fix.** Run #5 failed at
  Clippy before these changes. The workflow now installs the Linux Tauri
  development packages, but a new run still has to be pushed and observed by
  someone with repository access. Local runners cannot prove the hosted
  matrix.
- **Desktop file explorer destructive operations** (delete/rename through the
  UI) are implemented with confirmations but were not interactively driven.

## Known limitations

- Engine-mode WebSocket sessions are line-oriented. The Interrupt control and
  Ctrl+C clear the client input line, but they cannot cancel an engine command
  that is already running on the server. PTY mode forwards Ctrl+C to the real
  terminal process, and that path is verified.
- Engine sessions have no terminal size, so resize messages only affect the
  client's local rendering; the PTY transport does resize the real process.
- CLI signal handling and process-group job control remain unimplemented
  (`fg` waits rather than handing over the terminal; `bg` only reports state).
- No asynchronous background-job completion notice; run `jobs`/`wait`.
- No `source`/`.` script loading, no redirections on function calls, no
  globbing, no script comments, no `2>&1` merge form.
- Plain `cargo build --workspace --release` compiles the desktop crate in dev
  mode (no embedded front end); only `tauri build` produces distributable
  binaries — see `docs/DESKTOP.md`.
- Desktop PTY sessions run the embedded CLI worker in this same executable; a
  release-installed app therefore requires the `KERNEXT_DESKTOP_WORKER` path
  to remain wired (it is, through `shell_cli::worker::run_embedded`).
- The desktop uses `tauri-plugin-opener` only; capabilities are restricted to
  the commands the front end actually invokes.
- `macos-15-intel` (the last x86_64 GitHub runner image) is scheduled for
  retirement in August 2027; the Intel matrix leg will need a replacement
  after that date.

## How to re-verify

From the workspace root (the `.cargo/config.toml` target redirect applies
automatically):

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm ci
npm run typecheck
npm test
npm run build
```

Desktop, from the root:

```sh
npm run build -w @kernext/desktop -- --debug --no-bundle   # fast debug build
npm run desktop:build                           # release bundle (installers)
```
