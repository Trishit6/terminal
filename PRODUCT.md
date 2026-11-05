# Product

<!-- impeccable:product-schema 1 -->

## Platform

adaptive

## Stack

Rust workspace (cargo): 6 crates — `shell-cli` (binary `kernext`), `shell-core` (branding, lexer/parser/AST), `shell-builtins` (commands, pipeline, executor), `shell-runtime` (session, variables), `shell-line-editor` (Reedline, clear helpers), `shell-config` (TOML). Build via `.cargo/config.toml` target dir `$HOME/.cache/terminal-target` on ext4; sources on a vfat mount with `showexec`.

## Users

All types: developers and power users wanting a modern scriptable shell; learners and hobbyists exploring shell internals or Rust; embedded/constrained environments needing a lightweight custom shell.

## Product Purpose

KERNEXT is an independent, memory-safe CLI shell written in Rust with no external shell dependencies. It implements pipelines, redirections, variables, startup presentation, and 21 built-in commands natively, using `std::process::Command` directly for external execution — never invoking `sh -c`, `bash -c`, or `cmd /C`.

Official identity:

- Product: **KERNEXT**
- Tagline: **The Next Core of Computing.**
- Values: **OPEN • POWERFUL • INDEPENDENT**
- Support phrase: **Build. Execute. Innovate.**
- Prompt: `kernext ~ $ `
- Executable: `kernext`

## Positioning

Native Rust implementation: the entire shell (lexer, parser, AST, evaluator, pipeline engine, process executor, line editor, startup renderer) is written in Rust with zero external shell invocations. This yields fast startup, memory safety, and cross-platform behavioral parity that shells invoking subprocesses cannot guarantee.

## Operating Context

- Terminal/REPL usage on Linux, macOS, Windows.
- Implemented: lexer, parser, AST, evaluator, built-ins, external execution, pipelines, redirections, variables, config loading, KERNEXT startup, genuine `clear`/Ctrl+L.
- Partial: background `&` evaluates synchronously; scripting runtime does not iterate `for` loops or execute function calls through real call/return semantics.
- Sources live on a removable vfat mount with `showexec` (only `.exe/.com/.bat/.btm/.cmd` executable); all build artifacts land on ext4 via `.cargo/config.toml`.
- Interactive use requires a real TTY; piped stdin is rejected.
- Test suite (329 tests) uses the test binary itself as a portable helper via `CUSTOM_SHELL_TEST_HELPER` and `CUSTOM_SHELL_PIPELINE_ARGS` — no `python`, `git`, `sort`, `grep`, or other external programs required.

## Capabilities and Constraints

- **Custom lexer/parser/AST**: Hand-written in `shell-core` (no pest/nom/lalrpop).
- **Built-in command registry**: 21 native commands — `help`, `pwd`, `cd`, `ls`, `mkdir`, `touch`, `cat`, `echo`, `cp`, `mv`, `rm`, `clear`, `history`, `whoami`, `date`, `version`, `exit`, `export`, `unset`, `env`, `set`.
- **Variable system**: `$name`, `${name}`, `$?` (last exit status), `$$` (PID); expansion at evaluation time; double-quoted expands, single-quoted literal; escaped `\$` literal; one word → one argument (no field splitting).
- **Pipeline engine**: Real OS pipes (`ChildStdout` handed to next stage via `Stdio::from`); built-ins in pipelines run on cloned `Session` (filesystem effects persist, `cd`/state changes don't leak); redirections `>`, `>>`, `2>`, `2>>` resolve paths via session working directory.
- **External execution**: `std::process::Command` with args passed separately; bare names resolve via `PATH`; paths with separators resolve against session cwd; Windows uses `PATHEXT` skipping `.bat`/`.cmd`; Unix checks execute bits.
- **Startup presentation**: KERNEXT logo, tagline, values, and a real initialization checklist. Modes are animated/plain/quiet. Honors TTY, `NO_COLOR`, `TERM=dumb`, `--quiet`, `--no-animation`, `startup_animation`, and `quiet_startup`.
- **Screen clearing**: `clear` and Ctrl+L clear screen and scrollback via crossterm; suppressed for non-terminal stdout; never propagates from a pipeline stage.
- **Configuration**: TOML at the platform config path (`~/.config/kernext/config.toml` on Linux), loaded at startup with safe defaults on error.
- **REPL**: Reedline integration via `LineReader` (history, completions, prompt).
- **No external shell dependencies**: Never calls `sh`, `bash`, `cmd`, or any other shell.
- **Cross-platform parity**: Single codebase for Linux, macOS, Windows; Linux verified locally.

## Brand Commitments

- Product name: `KERNEXT`
- Tagline: `The Next Core of Computing.`
- Values: `OPEN • POWERFUL • INDEPENDENT`
- Support phrase: `Build. Execute. Innovate.`
- Executable name: `kernext`
- Binary package: `shell-cli` (unchanged for stability)
- Workspace name: `terminal` (unchanged for stability)

## Evidence on Hand

- Full source code in `apps/` and `crates/`
- 329 passing tests (`cargo test --workspace`)
- `AGENTS.md` — project constitution (architecture, branding, constraints, test commands)
- `README.md` — implemented CLI state, startup, clear, configuration, limitations, roadmap
- `docs/BRANDING.md` — logo, palette, voice, surface boundaries
- `docs/ARCHITECTURE.md` — crate boundaries and future Web/Desktop reuse plan
- `Cargo.toml` workspace with 6 members

## Product Principles

1. **Native Rust first** — No shell dependencies; `std::process::Command` only.
2. **Cross-platform parity** — Identical behavior on Linux, macOS, Windows.
3. **Honest capability reporting** — Never claim partial background or scripting support as complete.
4. **Phased, tested implementation** — Each phase completes with tests; no phase skipped.
5. **Library-first, embeddable** — `shell-core`, `shell-builtins`, and `shell-runtime` can be reused by future surfaces.
6. **Build resilience** — Sources on constrained mounts; artifacts on fast ext4 via the Cargo target config.

## Accessibility & Inclusion

Terminal accessibility: screen-reader compatible output (plain text, no raw escape sequences in non-interactive mode); high-contrast color defaults; keyboard-only operation; no mouse dependency; configurable prompt and keybindings via Reedline; startup can be suppressed for reduced motion and quiet environments.
