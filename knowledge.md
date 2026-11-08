# KERNEXT — project knowledge

## What this is

KERNEXT is an independent, cross-platform command-line shell written in Rust. It
has its own lexer, parser, AST, evaluator, built-in commands and external
process executor — it does **not** wrap `sh`, `bash`, PowerShell or `cmd`.

Branding is user-facing only: the product/executable is `kernext`, while every
crate, package and Rust identifier stays `shell-*`. Do not rename crates to
match the brand.

**Layout gotcha:** the Freebuff project root is not the Cargo workspace. The
Rust workspace lives one level down in `terminal/` (next to a `.agents/` folder
of agent type definitions). Run every Cargo command from `terminal/`.

```text
<project root>/
├── knowledge.md
├── .agents/types/          # Freebuff agent definition types (not used at runtime)
└── terminal/               # the Cargo workspace — README.md, AGENTS.md live here
    ├── Cargo.toml
    ├── .cargo/config.toml
    ├── apps/shell-cli/     # binary `kernext`
    ├── crates/…
    ├── installers/         # bash + PowerShell installers
    ├── docs/               # ARCHITECTURE.md, BRANDING.md
    └── .github/workflows/release.yaml
```

## Where the code lives

| Path | Responsibility |
| --- | --- |
| `terminal/crates/shell-core` | Branding constants, `ShellError`, lexer/tokenizer, parser, AST, spans. No I/O. |
| `terminal/crates/shell-config` | Typed `Config`, color/theme validation, TOML load/save, platform config path. |
| `terminal/crates/shell-runtime` | `Session` (cwd, variables, functions, last status), `Platform`, history, `JobTable`. |
| `terminal/crates/shell-builtins` | Registry, AST evaluator (`eval.rs`), native commands (`commands.rs`), pipelines (`pipeline.rs`), external executor (`process.rs`), expansion (`expand.rs`). |
| `terminal/crates/shell-line-editor` | Reedline wrapper, prompt, completer, history backing, screen clearing. |
| `terminal/apps/shell-cli` | Clap args, startup presentation (`startup.rs`), REPL (`repl.rs`), `main.rs`. |

Execution flow — keep every command line on this path:

```text
input -> lexer -> parser -> AST -> evaluator -> pipeline engine
      -> built-in registry OR external process executor -> CommandOutcome -> Repl
```

Dependency direction: `shell-cli → shell-builtins → shell-runtime → shell-config → shell-core`,
plus `shell-cli → shell-line-editor → reedline/crossterm`.

## Commands

Run from `terminal/`:

```sh
cargo build --workspace
cargo run -p shell-cli          # binary is kernext / kernext.exe
cargo test --workspace
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
```

Focused tests:

```sh
cargo test -p shell-builtins --test builtins
cargo test -p shell-builtins --test external
cargo test -p shell-builtins --test pipeline
cargo test -p shell-builtins --test language
cargo test -p shell-core --test parser
cargo test -p shell-core --test lexer
cargo test -p shell-cli --test repl
```

No install step; Rust 1.85+ (edition 2024). Interactive use requires a real
TTY — piping input into the shell is rejected by design.

## Conventions and hard constraints

- **Never split input with `split_whitespace`.** The dispatcher must always use
  `lex_spanned` + `parse`; quotes, escapes and argument boundaries come from the
  lexer only.
- **Never spawn another shell.** No `sh -c`, `bash -c`, `cmd /C`,
  `powershell -Command`. `process.rs` passes each argument as a separate
  `Command::arg` to the resolved executable path. Windows skips `.bat`/`.cmd`
  for the same reason; Unix checks execute bits.
- **Precedence is built-ins → functions → external programs.** A `PATH` program
  named `echo`/`cd`/`pwd` must never override a native command.
- **`cd` mutates `Session::working_directory` only**, never the process-wide cwd
  (tests depend on this isolation). `commands::resolve_path` is the single
  resolver for relative paths.
- **Built-ins inside a real pipeline run on a cloned `Session`**: filesystem
  effects persist, but `cd`/`export`/`unset` must not leak to the parent. A lone
  command still mutates the real session.
- **Pipelines use real OS pipes** (`ChildStdout` → next stage's `Stdio::from`).
  Never buffer a whole external stream just to connect stages; join writer
  threads and reap every child, including on the error path. Final stage decides
  success/exit code; intermediate `SIGPIPE` is not an error.
- `exit` and `clear` actions propagate only from a single command, never from a
  pipeline stage. `clear`/Ctrl+L writes CSI sequences via crossterm and is
  suppressed when stdout is not a terminal.
- Grouped pipeline stages are rejected with
  `"Grouped pipelines are not supported yet."`; standalone groups still run.
- Expansion happens at evaluation time, one word → one argument (no field
  splitting, no re-parsing). `$name`, `${name}`, `$?`, `$$`; single quotes and
  `\$` stay literal. Undefined variables expand to empty.
- `$?` is updated after every command/pipeline/assignment: 0 success, external
  code, 1 built-in failure, 2 syntax error, 128+signal.
- Cross-platform code only: `std::path`, `HOME`/`USERPROFILE`, no Unix-only APIs
  outside `#[cfg(unix)]`.
- Startup (`startup.rs`) is presentation-only — it must never parse, evaluate or
  launch commands, and must never fake progress or add artificial delays. It
  reports each checklist line only after the real subsystem is constructed.
- Never hardcode the version; use `VERSION` / `Session::version()`.

## Current feature set

- The registry (`crates/shell-builtins/src/registry.rs`) defines **53 built-ins**,
  including `tree`, `find`, `stat`, `head`, `tail`, `wc`, `grep`, `sort`,
  `uniq`, `which`, `basename`, `dirname`, `realpath`, `jobs`, `wait`, `kill`,
  `fg`, `bg`, `run` (script file), `test`, `[`, `expr`, `sleep`, `true`, `false`,
  and the scripting helpers `print`, `input`, `len`, `type`, `str`, `int`,
  `float`, `bool`.
- Background `&` is **real**: external commands are spawned and tracked via
  `Child` handles, other expressions run on a detached thread with a cloned
  session, and `crates/shell-runtime/src/jobs.rs` provides `JobTable`
  (`Running/Done/Failed/Killed`). There are no async completion notices, `fg`
  waits instead of taking the terminal, and `bg` only reports state.
- `run <file>` executes a script with nesting-depth and byte-size limits
  (`MAX_SCRIPT_DEPTH`, `MAX_SCRIPT_BYTES`).
- Startup artwork is the Cinematic Space & Systems lockup: a K monogram in an
  orbital ring above the block wordmark. Every emblem row must keep an identical
  character count (there is a test for it). `animation_speed`
  (`slow`/`normal`/`fast`) controls the reveal pace.

Still genuinely unimplemented — do not describe these as working: globbing,
`#` comments, redirections on function calls, grouped expressions as pipeline
stages, full job control semantics (`fg` waits rather than resuming a
foreground process group), POSIX signal handling, macOS/Windows release builds
in CI (only Linux GNU/musl are published).

Future KERNEXT Web and Desktop are plans only. Do not start them unless asked,
and never present them as implemented.

## Build gotcha: hardcoded target dir

`terminal/.cargo/config.toml` pins
`build.target-dir = "/home/trishit/.cache/terminal-target"`, a workaround for a
vfat-with-`showexec` checkout on the original Linux host (where extensionless
build scripts and test binaries cannot be executed). Consequences:

- On this Windows checkout that Unix path is meaningless; override it per
  command with `CARGO_TARGET_DIR=…` rather than editing the file if builds fail
  oddly.
- Do not create a repo-local `target/`, and do not add a target dir or
  OS-specific linker settings to `Cargo.toml`.

## Tests

379 tests pass (`cargo test --workspace`). Add tests next to the behavior: unit tests in `src/`,
integration tests in `crates/shell-builtins/tests/`, `crates/shell-core/tests/`,
`apps/shell-cli/tests/`.

- Filesystem tests use isolated temp dirs plus `Session::set_working_directory`;
  they never mutate the process cwd.
- External-process and pipeline tests use the test binary itself as a helper via
  the `CUSTOM_SHELL_TEST_HELPER` / `CUSTOM_SHELL_PIPELINE_ARGS` env vars, so no
  particular program (`python`, `grep`, `sort`, `git`) has to be installed.
  Keep them that way.
- Never weaken, skip or delete tests to make a build pass.

## Git

The repository has a single commit (`first commit`). The workspace was moved
into `terminal/`, so tracked paths show as deleted while the real tree is
untracked — do not "fix" that without being asked. Never run `git reset --hard`,
`git checkout -- .`, `git clean` or anything that discards working-tree changes.
Do not commit, rebase or push unless asked.
