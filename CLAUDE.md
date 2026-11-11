# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**KERNEXT** — "The Next Core of Computing." — is a custom Rust CLI shell with three implemented surfaces sharing one engine:
- **CLI** (`apps/shell-cli`, executable `kernext`) — reference terminal shell
- **Web** (`apps/shell-server` + `apps/web`) — React + Vite + Tailwind front end with Axum server with engine/PTY WebSockets
- **Desktop** (`apps/desktop`, crate `shell-desktop`, binary `kernext-desktop`) — Tauri 2 app reusing the web front end, executing via local Rust runtime only

Brand identity: **OPEN • POWERFUL • INDEPENDENT** | Values: Independent. Native. Engineered. | Prompt: `kernext ~ $ `

Internal crate/package names stay `shell-*`. Never rename them to match the brand.

## Build & Development Commands

### Rust (via KERNEXT project commands - preferred)
```bash
kernext fmt --check    # cargo fmt --all -- --check
kernext check          # cargo check --workspace --all-targets
kernext build          # cargo build --workspace
kernext test           # cargo test --workspace
kernext lint           # cargo clippy --workspace --all-targets -- -D warnings
kernext run            # cargo run -p shell-cli
kernext clean          # cargo clean
kernext doctor         # environment and project inspection
kernext version        # KERNEXT and toolchain versions
kernext help           # branded command reference
```
All project commands accept `--dry-run`, print the underlying command, and propagate exit code. `kernext` with no args starts the interactive shell (needs real TTY).

### Front End (npm workspaces, lockfile is `package-lock.json`)
```bash
npm ci                     # install apps/web and apps/desktop
npm run dev                # full stack: shell-server on 7878 + vite on 5173
npm run typecheck          # web typecheck
npm test                   # web Vitest tests
npm run build              # web production build
npm run web:dev            # vite dev server only (http://127.0.0.1:5173)
npm run web:build          # web production build
npm run web:preview        # preview built web
npm run web:typecheck      # web typecheck
npm run web:test           # web tests
npm run desktop:dev        # tauri dev (vite + debug window)
npm run desktop:build      # tauri build (production bundle)
cargo run -p shell-server  # web API server only
```
Root `desktop:*` scripts delegate to `apps/desktop/package.json` scripts. Tauri hooks use `npm --prefix ../web run …`.

### Focused Cargo Tests
```bash
cargo test -p shell-builtins --test external
cargo test -p shell-builtins --test external_tools
cargo test -p shell-builtins --test builtins
cargo test -p shell-builtins --test pipeline
cargo test -p shell-core --test parser
cargo test -p shell-cli --test repl
cargo test -p shell-server
cargo test -p shell-desktop
```
External tests use the test binary itself as helper via `CUSTOM_SHELL_TEST_HELPER` / `CUSTOM_SHELL_PIPELINE_ARGS` — no external deps required.

## Critical Build Constraint (VFAT + showexec)

Sources live on a **vfat** mount with `showexec`, which rejects extensionless build scripts and test binaries. `.cargo/config.toml` routes all artifacts to `/home/trishit/.cache/terminal-target` (ext4) automatically.
- **Never** `export CARGO_TARGET_DIR` manually
- **Never** create a repo-local `target/`
- All `cargo` commands work directly; the config handles it

## Architecture

### Execution Flow
```text
input -> shell-core lexer -> parser -> AST -> shell-builtins evaluator
      -> shell-builtins pipeline (stages, OS pipes, redirections)
      -> built-in registry OR process executor
      -> CommandOutcome -> shell-cli Repl
```

### Workspace Members

| Location | Role |
|---|---|
| `apps/shell-cli` | Binary `shell-cli` (executable `kernext`); `cli.rs` args/mode, `repl.rs` Reedline loop, `startup.rs` presentation only (never parses/evaluates), `worker.rs` embedded worker for desktop PTY sessions |
| `apps/shell-server` | Local Axum server: HTTP sessions, engine and PTY WebSockets, MariaDB-backed accounts |
| `apps/web` | React + Vite + Tailwind front end (shared by web & desktop) |
| `apps/desktop` | Tauri 2 app (crate `shell-desktop`, binary `kernext-desktop`); crate root `apps/desktop/src-tauri` |
| `crates/shell-core` | Branding constants (`lib.rs`), `lexer/`, `parser/`. Hand-written lexer/parser; no pest/nom/lalrpop |
| `crates/shell-builtins` | `registry.rs` (56 commands), `commands.rs`, `eval.rs` (AST evaluator, control flow, jobs), `pipeline.rs`, `process.rs` (external executor), `expand.rs`, `outcome.rs`, `pty.rs` |
| `crates/shell-runtime` | `Session` (cwd, config, lifecycle, platform, variables, functions), `variables.rs`, `history.rs`, `jobs.rs` |
| `crates/shell-line-editor` | Reedline behind `LineReader`; `clear_terminal` / `clear_terminal_if` in `editor.rs` |
| `crates/shell-config` | TOML config model |
| `crates/shell-api` | Serializable session/terminal contracts shared by server and desktop |

### Dependency Direction
```text
shell-cli ──> shell-builtins ──> shell-runtime ──> shell-config ──> shell-core
    │                                │
    │                                └──────────> shell-core
    ├──────────> shell-line-editor ──> reedline / crossterm
    └──────────> shell-core

shell-server ──> shell-api ──> shell-builtins ──> shell-runtime ──> shell-core

shell-desktop ─> shell-server (engine sessions) + shell-cli (embedded worker)
               ─> shell-api, shell-builtins, shell-runtime, shell-core
```

## Key Invariants

1. **Built-ins always win** — `eval.rs` checks native registry before external resolution; no `PATH` program overrides native command
2. **Pipelines use real OS pipes** — `ChildStdout` handed to next stage via `Stdio::from`; built-ins in pipelines run on cloned `Session` — filesystem side effects persist, but `cd`/state changes must not leak to parent shell
3. **No external shell** — Never use `sh -c`, `bash -c`, `cmd /C`, `powershell -Command`; launch resolved executables with `std::process::Command`, args passed separately
4. **`clear`/`exit` actions propagate only from a single command**, never from a pipeline stage
5. **Screen clearing** writes `CSI 2 J`, `CSI 3 J`, `CSI 1;1 H` via crossterm; never shell out to `clear`/`cls` and never print blank lines
6. **`$?`** updated after every command/pipeline/assignment: 0 success, external exit code, 1 built-in failure, 2 syntax/lex errors, 128+signal for signal termination
7. **Background `&`** spawns real detached work tracked by job table (`crates/shell-runtime/src/jobs.rs`); job IDs are shell numbers, never OS PIDs
8. **Desktop executes locally** — commands run through local Tauri Rust runtime only; never public web server, never JS simulation. `apps/desktop/src-tauri/build.rs` must keep shadowing MinGW `default-manifest.o` on `*-pc-windows-gnu`; removing it causes `STATUS_ENTRYPOINT_NOT_FOUND` at startup
9. **Front end uses npm** — workspaces in root `package.json`; lockfile `package-lock.json`; CI runs `npm ci`; never pnpm/yarn/bun and never second lockfile
10. **Root desktop scripts delegate** — `desktop:dev`/`desktop:build` call `dev`/`build` in `apps/desktop/package.json` with `npm run … -w @kernext/desktop`; Tauri hooks use `npm --prefix ../web run …`
11. **`kernext` project commands wrap Cargo** (`apps/shell-cli/src/dev.rs`); they never build shell command strings and always propagate child exit code

## Git Discipline

Repo has multiple commits with uncommitted changes. **Never** run `git reset --hard`, `git checkout -- .`, `git clean`, or anything that discards working-tree changes without explicit permission. Do not commit, amend, rebase, or push unless asked.

Canonical checkout: `D:\projects\React-Ts\terminal` (Windows). A second checkout may exist at `/home/trishit/Projects/custom-shell` — do not edit both.

## Documentation to Keep In Sync

`README.md`, `AGENTS.md`, `RELEASE_NOTES.md`, `CONTRIBUTING.md`,
`docs/BRANDING.md`, `docs/ARCHITECTURE.md`, `docs/COMMANDS.md`,
`docs/DESKTOP.md`, `docs/DEPLOYMENT.md`, `docs/TROUBLESHOOTING.md`,
`docs/SECURITY.md`, and `docs/PROJECT_STATUS.md` should stay consistent with behavior. Trust source and tests over docs when they disagree.

## Not Yet Implemented

- Asynchronous job notifications
- Script sourcing (`.`/`source`)
- Redirections on function calls
- Full job control (process-group handoff)
- Signal handling and process-group/terminal ownership
- Globbing/pattern matching
- Comments in scripts
- Grouped expressions as pipeline stages (`(a; b) | c`)
- Flag/option completion
- Linux/macOS GUI verification for Web and Desktop

## Branding Constants

Live in `crates/shell-core/src/lib.rs`:
- `SHELL_NAME = "kernext"` — drives config paths, Clap help/version, binary name
- `PRODUCT_NAME`, `TAGLINE`, `BRAND_STATEMENT`, `VALUES`, `MOTTO`, `VERSION`

Never hardcode versions; use `VERSION` / `Session::version()`. CLI is text-only; graphical splash/wallpaper belong to Web and Desktop surfaces.