# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working in this repository.

KERNEXT is a custom Rust CLI shell with three implemented surfaces sharing one
engine: the CLI (`apps/shell-cli`), the local web server and React front end
(`apps/shell-server`, `apps/web`), and the Tauri 2 desktop application
(`apps/desktop`). `AGENTS.md` is the full project constitution — it
overrides this file when they disagree. This file is the quick-reference version:
how to build, where the pieces live, and the non-obvious constraints.

## Branding

- Product: **KERNEXT**. Tagline: **The Next Core of Computing.**
  Brand statement: **Independent. Native. Engineered.**
  Values: **OPEN • POWERFUL • INDEPENDENT**. Support phrase: **Build. Execute. Innovate.**
- Prompt: `kernext ~ $ `. Executable: `kernext` (desktop: `kernext-desktop`).
- Internal crate/package names stay `shell-*`. Never rename them to match the brand.
- Branding constants live in `crates/shell-core/src/lib.rs` (`SHELL_NAME`,
  `PRODUCT_NAME`, `TAGLINE`, `BRAND_STATEMENT`, `VALUES`, `MOTTO`, `VERSION`).
  `SHELL_NAME = "kernext"`.
- The CLI is text-only. The graphical splash and mountain wallpaper belong to
  the Web and Desktop surfaces in `apps/web` / `apps/desktop`.

## Build constraint (removable drive)

Sources live on a **vfat** mount with `showexec`, which rejects extensionless build
scripts and test binaries. `.cargo/config.toml` routes all build artifacts to
`/home/trishit/.cache/terminal-target` (ext4) automatically. Never `export
CARGO_TARGET_DIR` manually, and never create a repo-local `target/`.

```bash
kernext check          # cargo check --workspace --all-targets
kernext build          # cargo build --workspace
kernext test           # cargo test --workspace
kernext lint           # cargo clippy --workspace --all-targets -- -D warnings
kernext fmt --check    # cargo fmt --all -- --check
kernext run            # cargo run -p shell-cli
kernext doctor
kernext version
kernext help

npm ci
npm run typecheck
npm test
npm run build
npm run web:dev
npm run desktop:dev
npm run desktop:build
cargo run -p shell-server
```

Run a single test or test file:

```bash
cargo test -p shell-builtins --test pipeline
cargo test -p shell-builtins --test external_tools
cargo test -p shell-builtins --test builtins paths_containing_spaces
cargo test -p shell-core --test parser
cargo test -p shell-cli --test repl
cargo test -p shell-server
cargo test -p shell-desktop
```

The interactive shell needs a real TTY. Piping input into `cargo run` is rejected;
use a terminal, `screen`, or `tmux` for manual verification.

## Architecture

Execution flow:

```text
input -> shell-core lexer -> parser -> AST -> shell-builtins evaluator
      -> shell-builtins pipeline (stages, OS pipes, redirections)
      -> built-in registry OR process executor
      -> CommandOutcome -> shell-cli Repl
```

Workspace members:

- `apps/shell-cli` — binary `shell-cli`, executable `kernext`. `cli.rs` (arg parsing,
  mode resolution), `repl.rs` (Reedline loop), `startup.rs` (presentation only — never
  parses or evaluates), `worker.rs` (embedded worker used by desktop PTY sessions),
  `main.rs`.
- `apps/shell-server` — local Axum server: HTTP sessions, engine and PTY WebSockets.
- `apps/web` — React + Vite + Tailwind front end shared by web and desktop.
- `apps/desktop` — Tauri 2 application (crate `shell-desktop`, binary
  `kernext-desktop`); crate root is `apps/desktop/src-tauri`.
- `crates/shell-core` — `lib.rs` (branding), `lexer/`, `parser/`. Hand-written lexer
  and parser; no pest/nom/lalrpop.
- `crates/shell-builtins` — `registry.rs` (56 commands + metadata), `commands.rs`
  (implementations), `eval.rs` (AST evaluator, control flow, background jobs),
  `pipeline.rs` (engine), `process.rs` (external executor), `expand.rs`, `outcome.rs`,
  `pty.rs` (ConPTY/openpty pseudoterminals).
- `crates/shell-runtime` — `Session` (cwd, config, lifecycle, platform, variables,
  functions), `variables.rs`, `history.rs`, `jobs.rs`.
- `crates/shell-line-editor` — Reedline behind `LineReader`; `clear_terminal` /
  `clear_terminal_if` helpers in `editor.rs`.
- `crates/shell-config` — TOML config model.
- `crates/shell-api` — serializable session/terminal contracts shared by server and
  desktop.
- `installers/` — Linux, macOS, Windows scripts.
- `docs/` — `BRANDING.md`, `ARCHITECTURE.md`, `COMMANDS.md`, `DESKTOP.md`,
  `DEPLOYMENT.md`, `PROJECT_STATUS.md`, `SECURITY.md`, `TROUBLESHOOTING.md`;
  plus root `CONTRIBUTING.md`.

## Key invariants

- **Built-ins always win.** `eval.rs` checks the registry before external resolution;
  a `PATH` program never overrides a native command.
- **Pipelines use real OS pipes.** `ChildStdout` is handed to the next stage via
  `Stdio::from`. Built-ins in pipelines run on a cloned `Session` — filesystem side
  effects persist, but `cd`/state changes must not leak to the parent shell.
- **No external shell.** Never use `sh -c`, `bash -c`, `cmd /C`. Launch resolved
  executables with `std::process::Command`, args passed separately.
- **`clear`/`exit` actions propagate only from a single command**, never from a
  pipeline stage.
- **Screen clearing** writes `CSI 2 J`, `CSI 3 J`, `CSI 1;1 H` via crossterm; never
  shell out to `clear`/`cls` and never print blank lines as a substitute.
- **`$?`** is updated after every command/pipeline/assignment: 0 on success, external
  exit code, 1 for built-in failure, 2 for syntax/lex errors, 128+signal for signal
  termination.
- **Background `&`** spawns real detached work tracked by a job table
  (`crates/shell-runtime/src/jobs.rs`). Job IDs are shell job numbers, never OS PIDs.
- **Desktop executes locally.** Desktop commands run through the local Tauri
  Rust runtime only — never the public web server, never a JavaScript shell
  simulation. `apps/desktop/src-tauri/build.rs` must keep shadowing the MinGW
  `default-manifest.o` on `*-pc-windows-gnu` (see `docs/DESKTOP.md`); removing
  it makes the binary die at startup with `STATUS_ENTRYPOINT_NOT_FOUND`.
- **Front end uses npm.** Workspaces are declared in the root `package.json`;
  the lockfile is `package-lock.json`; CI runs `npm ci`. Never reintroduce
  pnpm, yarn, or bun, and never add a second lockfile.
- **Root desktop scripts delegate.** `desktop:dev`/`desktop:build` call the
  `dev`/`build` scripts in `apps/desktop/package.json` with
  `npm run … -w @kernext/desktop`; Tauri's hooks use
  `npm --prefix ../web run …`.
- **`kernext` project commands wrap Cargo.** `apps/shell-cli/src/dev.rs`;
  they never build a shell command string and always propagate the child exit
  code.

## Git discipline

The repo has multiple commits. The working tree may contain uncommitted changes
and untracked files. **Never** run `git reset --hard`, `git checkout -- .`,
`git clean`, or anything that discards working-tree changes without explicit
permission. Do not commit, amend, rebase, or push unless asked.

On this Windows workstation the canonical checkout is
`D:\projects\React-Ts\terminal`. A second checkout may exist at
`/home/trishit/Projects/custom-shell` — do not edit both.

## Docs

`README.md`, `AGENTS.md`, `RELEASE_NOTES.md`, `CONTRIBUTING.md`,
`docs/BRANDING.md`, `docs/ARCHITECTURE.md`, `docs/COMMANDS.md`,
`docs/DESKTOP.md`, `docs/SECURITY.md`, `docs/DEPLOYMENT.md`,
`docs/TROUBLESHOOTING.md`, and `docs/PROJECT_STATUS.md`
should stay in sync with behavior. Trust the source and tests
over the docs when they disagree.
