# Copilot instructions for KERNEXT

## Project overview

KERNEXT is a Rust shell with three implemented surfaces sharing one engine:

- CLI: `apps/shell-cli` (`kernext`)
- Web: `apps/shell-server` + `apps/web`
- Desktop: `apps/desktop` (Tauri 2), executing through the local Rust runtime

The shell is not a Bash/PowerShell wrapper. The execution path is:

`input -> shell-core lexer -> parser -> AST -> shell-builtins evaluator -> pipeline/executor -> CommandOutcome`

Core workspace crates:

- `crates/shell-core`: lexer, parser, AST, spans, shared errors
- `crates/shell-builtins`: built-ins, AST evaluator, pipeline engine, external process execution, variable expansion, job handling
- `crates/shell-runtime`: `Session`, working dir, config, variables, history, jobs
- `crates/shell-line-editor`: Reedline + terminal clear helpers
- `crates/shell-config`: TOML config
- `crates/shell-api`: shared session/terminal API contracts

## Build, test, and lint commands

Prefer the project wrapper when available:

```bash
kernext fmt --check     # cargo fmt --all -- --check
kernext check           # cargo check --workspace --all-targets
kernext build           # cargo build --workspace
kernext test            # cargo test --workspace
kernext lint            # cargo clippy --workspace --all-targets -- -D warnings
kernext run             # cargo run -p shell-cli
kernext doctor          # environment and project inspection
kernext version         # KERNEXT and toolchain status
```

Direct Cargo equivalents also work from the repo root:

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Web/desktop commands:

```bash
npm ci
npm run dev                # Axum server + Vite frontend
npm run web:dev            # Vite frontend only
npm run web:typecheck      # web typecheck
npm run web:test           # web Vitest suite
npm run web:build          # web production build
npm run desktop:dev        # Tauri dev
npm run desktop:build      # Tauri production build
cargo run -p shell-server  # server only
```

Run a single Rust test by name or by test target:

```bash
cargo test -p shell-builtins --test builtins
cargo test -p shell-builtins --test builtins some_test_name
cargo test -p shell-core --test parser
cargo test -p shell-cli --test repl
cargo test -p shell-server
```

When a single target has many tests, limit to one file or one name instead of running the whole workspace.

## High-level architecture

The repository is split into a clear engine + surfaces model:

- `shell-core` owns the lexer, parser, AST, and token semantics
- `shell-builtins` owns the evaluator, built-ins list, pipeline orchestration, and external process launch logic
- `shell-runtime` owns runtime state (`Session`, variables, jobs, history, environment)
- `apps/shell-cli` owns the interactive REPL and startup presentation
- `apps/shell-server` owns the local web API and PTY/engine connections
- `apps/web` owns the React UI shared by web and desktop
- `apps/desktop` owns the Tauri desktop shell and delegates execution to the local Rust runtime

Key execution behavior to preserve:

- Built-ins win before external programs
- Pipelines use real OS pipes; they are not simulated in userland
- Redirections, `&&`, `||`, and `;` are evaluated by the native engine
- `Session::working_directory` is the source of truth for relative paths
- Windows and Unix code paths are kept behind `cfg` gates rather than using platform-specific assumptions everywhere

## Repository conventions and project-specific rules

- Keep internal crate names as `shell-*`; do not rename modules/crates to match the branding.
- Preserve the KERNEXT brand in user-facing strings, config paths, docs, and CLI help, but keep internal identifiers stable.
- Do not bypass the shell parser by splitting input with `split_whitespace`; use the lexer/parser path.
- Do not launch external commands via `sh -c`, `bash -c`, `cmd /C`, or `powershell -Command`. Use `std::process::Command` with separate arguments. The Windows command-script exception is limited to `.cmd`/`.bat` launchers.
- Do not add an artificial delay to startup animation; the reveal should be brief and presentation-only.
- Treat `clear`/`exit` as action results from a single command, not from a pipeline stage.
- In tests, use isolated temp directories and `Session::set_working_directory`; do not mutate the process-wide CWD.
- Keep the repo-root `.cargo/config.toml` target directory setting intact. It routes output outside the source tree; do not add a repo-local `target/` or set `CARGO_TARGET_DIR` manually.
- Do not edit the alternate checkout or create additional copies; this repo’s active checkout is `D:\projects\React-Ts\terminal`.

## Relevant implementation notes

- The web terminal is a DOM renderer, not xterm.js/canvas; keep the buffer/parser/rendering pipeline in `apps/web/src/lib/terminal/` intact.
- The desktop build on GNU Windows keeps a custom manifest shadowing step in `apps/desktop/src-tauri/build.rs`; do not remove it.
- Server mode is account-backed when `DATABASE_URL` is set; otherwise it keeps local loopback mode. Do not assume the database is always present.
- The default web/dev commands are designed around the root workspace setup; keep script changes aligned with the root `package.json` and workspace contracts.

## Working style for this repo

This codebase is unusually feature-complete for a shell project, so the safest changes are the ones that respect the existing engine boundaries rather than introducing a parallel code path. When editing, prefer:

- the existing parser/evaluator path for shell syntax changes
- the command registry for new built-ins rather than ad-hoc string parsing
- the shared runtime/session APIs rather than per-surface state mutations
- existing test files close to the behavior being changed
