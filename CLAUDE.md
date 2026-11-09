# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working in this repository.

KERNEXT is a custom Rust CLI shell. `AGENTS.md` is the full project constitution — it
overrides this file when they disagree. This file is the quick-reference version:
how to build, where the pieces live, and the non-obvious constraints.

## Branding

- Product: **KERNEXT**. Tagline: **The Next Core of Computing.**
  Values: **OPEN • POWERFUL • INDEPENDENT**. Support phrase: **Build. Execute. Innovate.**
- Prompt: `kernext ~ $ `. Executable: `kernext`.
- Internal crate/package names stay `shell-*`. Never rename them to match the brand.
- Branding constants live in `crates/shell-core/src/lib.rs` (`SHELL_NAME`,
  `PRODUCT_NAME`, `TAGLINE`, `VALUES`, `MOTTO`, `VERSION`). `SHELL_NAME = "kernext"`.
- The CLI is text-only. No graphical splash — that belongs to future Web/Desktop.

## Build constraint (removable drive)

Sources live on a **vfat** mount with `showexec`, which rejects extensionless build
scripts and test binaries. `.cargo/config.toml` routes all build artifacts to
`/home/trishit/.cache/terminal-target` (ext4) automatically. Never `export
CARGO_TARGET_DIR` manually, and never create a repo-local `target/`.

```bash
cargo check --workspace --all-targets
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo run -p shell-cli
```

Run a single test or test file:

```bash
cargo test -p shell-builtins --test pipeline
cargo test -p shell-builtins --test pipeline paths_containing_spaces
cargo test -p shell-core --test parser
cargo test -p shell-cli --test repl
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
  parses or evaluates), `main.rs`.
- `crates/shell-core` — `lib.rs` (branding), `lexer/`, `parser/`. Hand-written lexer
  and parser; no pest/nom/lalrpop.
- `crates/shell-builtins` — `registry.rs` (53 commands + metadata), `commands.rs`
  (implementations), `eval.rs` (AST evaluator, control flow, background jobs),
  `pipeline.rs` (engine), `process.rs` (external executor), `expand.rs`, `outcome.rs`.
- `crates/shell-runtime` — `Session` (cwd, config, lifecycle, platform, variables,
  functions), `variables.rs`, `history.rs`, `jobs.rs`.
- `crates/shell-line-editor` — Reedline behind `LineReader`; `clear_terminal` /
  `clear_terminal_if` helpers in `editor.rs`.
- `crates/shell-config` — TOML config model.
- `installers/` — Linux, macOS, Windows scripts.
- `docs/` — `BRANDING.md`, `ARCHITECTURE.md`.

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
  (`crates/shell-runtime/src/jobs.py`). Job IDs are shell job numbers, never OS PIDs.

## Git discipline

The repo has multiple commits. The working tree may contain uncommitted changes
and untracked files. **Never** run `git reset --hard`, `git checkout -- .`,
`git clean`, or anything that discards working-tree changes without explicit
permission. Do not commit, amend, rebase, or push unless asked.

On this Windows workstation the canonical checkout is
`D:\projects\React-Ts\terminal`. A second checkout may exist at
`/home/trishit/Projects/custom-shell` — do not edit both.

## Docs

`README.md`, `AGENTS.md`, `RELEASE_NOTES.md`, `docs/BRANDING.md`, and
`docs/ARCHITECTURE.md` should stay in sync with behavior. Trust the source and tests
over the docs when they disagree.