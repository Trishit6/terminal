# Product

<!-- impeccable:product-schema 1 -->

## Platform

adaptive

## Stack

Rust workspace (cargo): 6 crates — `shell-cli` (binary), `shell-core` (lexer/parser/AST), `shell-builtins` (commands, pipeline, executor), `shell-runtime` (session, variables), `shell-line-editor` (Reedline), `shell-config` (TOML). Build via `CARGO_TARGET_DIR="$HOME/.cache/terminal-target"` on ext4; sources on vfat mount with `showexec`.

## Users

All types: developers and power users wanting a modern scriptable shell; learners and hobbyists exploring shell internals or Rust; embedded/constrained environments needing a lightweight custom shell.

## Product Purpose

A custom Rust CLI shell (`custom-shell` executable) that provides a fully native, memory-safe shell experience with no external shell dependencies. It implements pipelines, redirections, variables, and 21 built-in commands natively in Rust, using `std::process::Command` directly for external execution — never invoking `sh -c`, `bash -c`, or `cmd /C`.

## Positioning

Native Rust implementation: the entire shell (lexer, parser, AST, evaluator, pipeline engine, process executor, line editor) is written in Rust with zero external shell invocations. This yields fast startup, memory safety, and cross-platform behavioral parity that shells invoking subprocesses cannot guarantee.

## Operating Context

- Terminal/REPL usage on Linux, macOS, Windows.
- Phased implementation: Phases 1–8 complete (lexer, parser, AST, evaluator, built-ins, external execution, pipelines, redirections, variables). Phase 9+ (job control, scripting) not started.
- Sources live on a removable vfat mount with `showexec` (only `.exe/.com/.bat/.btm/.cmd` executable); all build artifacts land on ext4 via `CARGO_TARGET_DIR="$HOME/.cache/terminal-target"`.
- Interactive use requires a real TTY; piping into `cargo run` fails Reedline.
- Test suite (306 tests) uses the test binary itself as a portable helper via `CUSTOM_SHELL_TEST_HELPER` and `CUSTOM_SHELL_PIPELINE_ARGS` — no `python`, `git`, `sort`, `grep`, or other external programs required.

## Capabilities and Constraints

- **Custom lexer/parser/AST**: Hand-written in `shell-core` (no pest/nom/lalrpop).
- **Built-in command registry**: 21 native commands — `help`, `pwd`, `cd`, `ls`, `mkdir`, `touch`, `cat`, `echo`, `cp`, `mv`, `rm`, `clear`, `history`, `whoami`, `date`, `version`, `exit`, `export`, `unset`, `env`, `set`.
- **Variable system**: `$name`, `${name}`, `$?` (last exit status), `$$` (PID); expansion at evaluation time; double-quoted expands, single-quoted literal; escaped `\$` literal; one word → one argument (no field splitting).
- **Pipeline engine**: Real OS pipes (`ChildStdout` handed to next stage via `Stdio::from`); built-ins in pipelines run on cloned `Session` (filesystem effects persist, `cd`/state changes don't leak); redirections `>`, `>>`, `2>`, `2>>` resolve paths via session working directory.
- **External execution**: `std::process::Command` with args passed separately; bare names resolve via `PATH`; paths with separators resolve against session cwd; Windows uses `PATHEXT` skipping `.bat`/`.cmd`; Unix checks execute bits.
- **REPL**: Reedline integration via `LineReader` (history, completions, prompt).
- **Configuration**: TOML via `shell-config`.
- **No external shell dependencies**: Never calls `sh`, `bash`, `cmd`, or any other shell.
- **Cross-platform parity**: Single codebase for Linux, macOS, Windows.

## Brand Commitments

- Executable name: `custom-shell`
- Binary package: `shell-cli`
- Workspace name: `terminal`

## Evidence on Hand

- Full source code in `apps/` and `crates/`
- 306 passing tests (`cargo test --workspace`)
- `AGENTS.md` — complete project constitution (phases, architecture, constraints, test commands)
- `README.md` — implemented Phase 1–7 state, build workflow for vfat mount
- `Cargo.toml` workspace with 6 members

## Product Principles

1. **Native Rust first** — No shell dependencies; `std::process::Command` only.
2. **Cross-platform parity** — Identical behavior on Linux, macOS, Windows.
3. **Phased, tested implementation** — Each phase completes with tests; no phase skipped.
4. **Library-first, embeddable** — Crates are reusable; `shell-core`, `shell-builtins`, `shell-runtime` can be used independently.
5. **Build resilience** — Sources on constrained mounts; artifacts on fast ext4 via `CARGO_TARGET_DIR`.

## Accessibility & Inclusion

Terminal accessibility: screen-reader compatible output (plain text, no raw escape sequences in non-interactive mode); high-contrast color defaults; keyboard-only operation; no mouse dependency; configurable prompt and keybindings via Reedline.