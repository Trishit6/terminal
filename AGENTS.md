# AGENTS.md

## Project

Custom Rust CLI shell. Phase 1–8 are implemented. Do not start Phase 9 or add
job control or scripting unless the user explicitly asks.
Pipelines and redirections are implemented natively; keep them that way.

Workspace members:

- `apps/shell-cli` — binary package `shell-cli`; executable name is `custom-shell`
- `crates/shell-core` — lexer, parser, AST, spans, shared errors
- `crates/shell-builtins` — native command registry, AST evaluator, built-ins,
  pipeline engine, external process executor, variable expansion
- `crates/shell-runtime` — `Session` (cwd, config, lifecycle, platform, variables)
- `crates/shell-line-editor` — Reedline integration behind `LineReader`
- `crates/shell-config` — TOML configuration model

Execution flow is:

```text
input -> shell-core lexer -> parser -> AST -> shell-builtins evaluator
      -> shell-builtins pipeline (stages, OS pipes, redirections)
      -> built-in registry OR process executor
      -> CommandOutcome -> shell-cli Repl
```

Do not bypass this by splitting input with `split_whitespace`. `Dispatcher`
must keep using `lex_spanned` + `parse`.

## External execution (Phase 6)

- Executor: `crates/shell-builtins/src/process.rs`.
- Never use `sh -c`, `bash -c`, `cmd /C`, or another shell. Launch the resolved
  executable with `std::process::Command` and pass each argument separately.
- Built-ins always win. `eval.rs` checks the registry before external
  resolution; do not let a `PATH` program override a native command.
- Bare names resolve through `PATH`; names with separators resolve against the
  session working directory; absolute paths run directly.
- Windows resolution uses `PATHEXT` but skips `.bat`/`.cmd` so no other shell is
  involved. Unix checks execute bits.
- Default `StdioMode::Inherit` streams directly to the terminal. Use
  `StdioMode::Capture` for tests and non-interactive callers; it closes stdin.
- `ProcessOptions` carries stdio mode and environment overrides. It does not
  mutate the shell's global environment.
- `CommandOutcome::success()`/`exit_code()` carry the child status. `None`
  exit code means a built-in or a signal-terminated process.

## Pipelines and redirections (Phase 7)

- Engine: `crates/shell-builtins/src/pipeline.rs`.
- `eval.rs` sends both single commands and real pipelines through the engine.
  Do not reintroduce a separate ad-hoc command path.
- Lexer tokens: `Token::ErrorRedirect` (`2>`) and
  `Token::ErrorAppendRedirect` (`2>>`). Recognise them only when the digit starts
  a token, so `file2>log` stays a word plus stdout redirection.
- AST kinds: `RedirectionKind::{Input, Output, Append, Error, ErrorAppend}`.
- External stages use real OS pipes: `ChildStdout` is handed to the next stage as
  `Stdio::from`. Never collect an entire external stream into memory just to
  connect a pipeline.
- Built-ins run in-process. They exchange buffered bytes with neighbours, and
  buffered data is written to an external child on a detached writer thread.
  Callers must join writers before returning; children must be waited for.
- Built-ins inside a real pipeline use a cloned `Session` (`Session` derives
  `Clone`). Filesystem side effects persist; `cd` and similar state changes must
  not leak to the parent shell. A single command still mutates the real session.
- `exit` and `clear` actions propagate only from a single command, never from a
  pipeline stage.
- Redirections resolve paths with `commands::resolve_path` so relative paths use
  the session working directory. `>` truncates, `>>` appends, `2>` truncates
  stderr, `2>>` appends stderr.
- A redirection applies to its own stage. `echo x | cat > out.txt` writes only
  the final stage's output.
- Always close unused pipe ends and reap every child, including on startup
  failure. Kill and wait for already-started children before returning an error.
- Pipeline success and exit code come from the final stage. Do not report
  intermediate `SIGPIPE` termination as an error.
- A built-in stage after a producer whose output was redirected away receives
  empty input, not the terminal and not a missing-operand error.
- Grouped pipeline stages are unsupported: report
  `"Grouped pipelines are not supported yet."` Standalone groups still run.

## Canonical checkout and duplicate-copy hazard

The user-designated canonical project is:

```text
/run/media/trishit/UBUNTU 26_0/Projects/terminal
```

A second checkout may exist at `/home/trishit/Projects/custom-shell`. Do not
edit both. Do not create another copy or workspace. If both exist, confirm which
one the user wants active before changing files.

## Critical build constraint (removable drive)

`terminal` is on a **vfat** mount with **`showexec`**, not a literal `noexec`.
`showexec` only executes files with `.exe/.com/.bat/.btm/.cmd` names, so Rust
build scripts and test binaries (extensionless) fail with:

```text
Permission denied (os error 13)
```

Default `cargo check/build/test/run` from `terminal` therefore fails while
compiling build scripts/proc macros. Do not misinterpret this as a code bug.
Do not change mount options without explicit permission.

**Permanent fix:** `.cargo/config.toml` sets `build.target-dir = "/home/trishit/.cache/terminal-target"`.
All Cargo commands automatically use the ext4 target directory.
No `export CARGO_TARGET_DIR` is required.

Sources stay only in `terminal`; all build artifacts land on ext4. Do not
hardcode a target directory in `Cargo.toml` or add OS-specific linker settings.
Do not change mount options without explicit permission.

A source tree on vfat can still be inspected, edited, diffed, formatted, and
used by `cargo metadata --no-deps`.

## Commands

Run from the workspace root of the buildable checkout.
The `.cargo/config.toml` automatically routes build artifacts to `/home/trishit/.cache/terminal-target`.

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p shell-cli
```

Focused tests:

```bash
cargo test -p shell-builtins --test external
cargo test -p shell-builtins --test builtins
cargo test -p shell-builtins --test builtins paths_containing_spaces
cargo test -p shell-builtins --test pipeline
cargo test -p shell-core --test parser
cargo test -p shell-cli --test repl
```

External-process and pipeline tests use the test binary itself as a portable
helper; they do not require `python`, `git`, `sort`, `grep`, or any other
particular program.

The interactive shell requires a real TTY. Piping input directly into
`cargo run` fails Reedline with a cursor-position error. Use a terminal, or
`screen`/`tmux`, for manual verification.

## Variable system (Phase 8)

- Variable store: `crates/shell-runtime/src/variables.rs` (`VariableStore`).
- Session integration: `crates/shell-runtime/src/lib.rs` (methods on `Session`).
- Expansion: `crates/shell-builtins/src/expand.rs` (`expand_word`, etc.).
- Assignment evaluation: `crates/shell-builtins/src/eval.rs` (`run_assignment`).
- Built-ins: `crates/shell-builtins/src/commands.rs` (`export_builtin`, `unset_builtin`, `env_builtin`, `set_builtin`).
- Registry: `crates/shell-builtins/src/registry.rs` (4 new commands, total 21).
- Lexer tokens: `Token::Equals`, `Token::SpecialVariable`, `SpecialVariable::{LastStatus, ProcessId}`.
- AST: `Expression::Assignment`, `WordPart::Special`, variable-name helpers in `shell-core`.

### Assignment syntax

- `name = value` and `name=value` are both parsed as assignments.
- Exactly one value word; extra unquoted words produce a parse error.
- Empty value allowed (`name =`).
- Assignments cannot be pipeline stages or carry redirections.
- Assignments allowed as operands of `&&`/`||` and inside groups.

### Expansion

- Occurs at evaluation time, not during parsing.
- `$name` and `${name}` expand to variable value (empty string if undefined).
- `$?` expands to last command exit status.
- `$$` expands to shell process ID.
- Double-quoted variables expand; single-quoted stay literal.
- Escaped `\$` stays literal.
- One word → one argument; no field splitting or re-parsing.
- Expansion applies to command names, arguments, and redirection targets.

### Environment

- `VariableStore` snapshots process environment at session creation.
- Three layers: local variables (with export flag), hidden inherited names, inherited base.
- Windows: case-insensitive keys; Unix: case-sensitive.
- Child env = inherited (minus hidden) + exported locals + PWD + overrides.
- `Session::with_environment` for deterministic tests.

### Special variables

- `$?` — updated after every command/pipeline/assignment:
  - 0 on success / built-in success / assignment
  - External exit code
  - 1 for built-in failure
  - 2 for syntax/lex errors
  - 128+signal for signal termination
- `$$` — process ID from `std::process::id()`.
- `$PWD` — initialized exported, updated by `set_working_directory`.
- `$HOME` — initialized exported from env or `USERPROFILE`.

### Pipeline behavior

- Built-ins in pipelines use cloned `Session`; `export`/`unset` inside pipeline do not leak to parent.
- Single command still mutates real session.
- `$?` updated from final stage outcome.

## Built-in behavior

- Registry and command metadata: `crates/shell-builtins/src/registry.rs`.
- Implementations: `crates/shell-builtins/src/commands.rs`.
- AST evaluator: `crates/shell-builtins/src/eval.rs`.
- 21 commands: help, pwd, cd, ls, mkdir, touch, cat, echo, cp, mv, rm, clear,
  history, whoami, date, version, exit, export, unset, env, set.
- `Session::working_directory` is the single source of truth for relative
  paths. `cd` validates/canonicalizes and updates the session; it deliberately
  does not call process-wide `env::set_current_dir`, preserving test isolation.
- Lists (`;`), groups, `&&`/`||`, pipelines, redirections, and assignments are evaluated.
- Pipelines bind more tightly than `&&` and `||`. Quoted operators are ordinary
  argument text.
- `CommandResult` keeps stdout and stderr separate. `CommandContext::stdin`
  carries pipeline or `<` input. `cat` with no operands reads that input and
  reports a missing operand only when no input was supplied.
- Non-built-in names go to `process.rs`, never to `split_whitespace` or another
  shell. Quotes are decoded by the lexer; arguments stay boundary-preserving.
- `date` uses `chrono`; `shell-builtins` also depends on `shell-core` for the
  lexer/parser and `std::process` for external execution.
- Keep Linux, macOS, and Windows compatibility. Use `std::path`, resolve home
  from `HOME`/`USERPROFILE`, and avoid Unix-only APIs.

## Tests

- Filesystem tests use isolated temp dirs and call
  `Session::set_working_directory`; never mutate the process cwd in tests.
- Add focused tests next to the behavior: unit tests in `src/`, integration
  tests in `crates/shell-builtins/tests/`, `crates/shell-core/tests/`, and
  `apps/shell-cli/tests/`.
- External-process tests live in `crates/shell-builtins/tests/external.rs` and
  use the test binary itself as a helper via `CUSTOM_SHELL_TEST_HELPER`. Do not
  replace them with tests that require a particular external program.
- Pipeline tests live in `crates/shell-builtins/tests/pipeline.rs` and use the
  same test binary as a helper via `CUSTOM_SHELL_PIPELINE_ARGS`. Their marker
  lines (`PIPELINE:`) separate helper output from libtest harness output.
- Variable tests: `crates/shell-runtime/src/variables.rs` (unit tests),
  `crates/shell-builtins/src/expand.rs` (expansion unit tests).
- The full suite currently has 306 tests; do not weaken or delete tests to make
  a build pass.

## Git and uncommitted work

The repository has one commit (`first commit`). All Phase 3.1–7 work is
uncommitted, and several Phase 5/6/7 files are untracked. **Never** run
`git reset --hard`, `git checkout -- .`, `git clean`, or any command that
discards working-tree changes without explicit permission. Do not commit,
amend, rebase, or push unless asked.

Before risky migration or cleanup, snapshot tracked/untracked work outside the
repo.

## Documentation

`README.md` describes the implemented Phase 1–7 state, including external
execution, pipelines, redirections, and the mounted-drive `CARGO_TARGET_DIR`
requirement. Keep it in sync when behavior changes; trust the source and tests
if they ever disagree.
