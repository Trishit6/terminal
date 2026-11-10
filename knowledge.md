# KERNEXT — project knowledge

## What this is

KERNEXT is an independent, cross-platform command-line shell written in Rust. It
has its own lexer, parser, AST, evaluator, built-in commands and external
process executor — it does **not** wrap `sh`, `bash`, PowerShell or `cmd`.

Branding is user-facing only: the product/executable is `kernext`, while every
crate, package and Rust identifier stays `shell-*`. Do not rename crates to
match the brand.

The Cargo workspace **is** this project root (no nested workspace directory).

```text
<project root>/
├── Cargo.toml               # workspace, edition 2024, rust-version 1.85
├── .cargo/config.toml       # pins build.target-dir (see build gotcha)
├── apps/shell-cli/          # binary `kernext`
├── crates/                  # shell-core, shell-config, shell-runtime,
│                            # shell-builtins, shell-line-editor
├── installers/              # install-linux.sh, install-macos.sh, install-windows.ps1
├── docs/                    # ARCHITECTURE.md, BRANDING.md
├── README.md                # user-facing feature/behavior reference
├── AGENTS.md                # agent rules (authoritative over README)
└── .github/workflows/       # release automation
```

Note: `README.md` and `AGENTS.md` still describe the canonical Linux checkout at
`/run/media/trishit/UBUNTU 26_0/Projects/terminal`, and README's tree diagram
shows an older `terminal/` wrapper. On this Windows workstation the active
checkout is `D:\projects\React-Ts\terminal`. Trust the actual filesystem layout
above.

## Where the code lives

| Path | Responsibility |
| --- | --- |
| `crates/shell-core` | Branding constants, `ShellError`, lexer/tokenizer, parser, AST, spans. No I/O. |
| `crates/shell-config` | Typed `Config`, color/theme validation, TOML load/save, platform config path. |
| `crates/shell-runtime` | `Session` (cwd, variables, functions, last status), `Platform`, history, `JobTable`, script depth. |
| `crates/shell-builtins` | Registry (`registry.rs`), AST evaluator (`eval.rs`), native commands (`commands.rs`), pipelines (`pipeline.rs`), external executor (`process.rs`), expansion (`expand.rs`). |
| `crates/shell-line-editor` | Reedline wrapper, prompt, completer, history backing, screen clearing. |
| `apps/shell-cli` | Clap args, startup presentation (`startup.rs`), REPL (`repl.rs`), `main.rs`. |

Execution flow — keep every command line on this path:

```text
input -> lexer -> parser -> AST -> evaluator -> pipeline engine
      -> built-in registry OR external process executor -> CommandOutcome -> Repl
```

Dependency direction: `shell-cli → shell-builtins → shell-runtime → shell-config → shell-core`,
plus `shell-cli → shell-line-editor → reedline/crossterm`.

## Commands

Run from the workspace root:

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

No install step; Rust 1.85+. Interactive use requires a real TTY — piping input
into the shell is rejected by design.

## Conventions and hard constraints

- **Never split input with `split_whitespace`.** The dispatcher must always use
  `lex_spanned` + `parse`; quotes, escapes and argument boundaries come from the
  lexer only.
- **Never let another shell parse KERNEXT input.** No `sh -c`, `bash -c`,
  `cmd /C`, `powershell -Command` for command syntax. `process.rs` passes each
  argument as a separate `Command::arg` to the resolved executable path. Unix
  checks execute bits.
- **Windows launchers are platform-aware.** `.exe`/`.com` launch natively;
  `.cmd`/`.bat` run through `%COMSPEC%`/`System32\cmd.exe` with
  `/d /v:off /s /c` and a fully quoted command line. `PATHEXT` order is
  honoured but restricted to `.com`, `.exe`, `.bat`, `.cmd`.
- **Extensionless files are not executables on Windows.** npm/pnpm ship a POSIX
  `sh` shim with no extension next to the `.cmd` shim; picking the former is
  what caused `os error 193` for `opencode`/`omnirush`.
- Windows script arguments containing `"`, `%`, CR, LF, or NUL are refused with
  a clear error instead of being rewritten (cmd.exe cannot escape them). Keep
  `crates/shell-builtins/tests/windows_scripts.rs` green.
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
- Startup (`apps/shell-cli/src/startup.rs`) is presentation-only — it must never
  parse, evaluate or launch commands, and must never fake progress or add
  artificial delays. It reports each checklist line only after the real subsystem
  is constructed.
- Never hardcode the version; use `VERSION` / `Session::version()`.
- Emblem rows must all have identical character counts
  (`emblem_rows_keep_a_constant_display_width`); never hand-edit a row blindly.

## Current feature set

- The registry (`crates/shell-builtins/src/registry.rs`) defines
  `DEFAULT_BUILTINS: [BuiltinCommand; 56]` — including `tree`, `find`, `stat`,
  `head`, `tail`, `wc`, `grep`, `sort`, `uniq`, `which`, `basename`, `dirname`,
  `realpath`, `jobs`, `wait`, `kill`, `fg`, `bg`, `run` (script file), `test`,
  `[`, `expr`, `sleep`, `true`, `false`, `host`, `sysinfo`, and the scripting
  helpers `print`, `input`, `len`, `type`, `str`, `int`, `float`, `bool`.
- Background `&` is **real**: external commands are spawned and tracked via
  `Child` handles, other expressions run on a detached thread with a cloned
  session, and `crates/shell-runtime/src/jobs.rs` provides `JobTable`
  (`Running/Done/Failed/Killed`). There are no async completion notices, `fg`
  waits instead of taking the terminal, and `bg` only reports state.
- `run <file>` executes a script natively with limits
  `MAX_SCRIPT_DEPTH = 32` and `MAX_SCRIPT_BYTES = 8 MiB`. Functions stop at
  `MAX_FUNCTION_DEPTH = 128`; loops at `MAX_LOOP_ITERATIONS = 1_000_000`.
- Startup artwork is the Cinematic Space & Systems lockup: a K monogram in an
  orbital ring above the block wordmark. `animation_speed`
  (`slow`/`normal`/`fast`) controls the reveal pace; `--quiet` and
  `--no-animation` override config.
- Config lives in `crates/shell-config/src/lib.rs`, loaded from the platform
  config dir (`kernext/config.toml`); missing file → defaults, invalid file →
  warning + defaults. Unknown fields are rejected and `history_size > 0`.

Still genuinely unimplemented — do not describe these as working: globbing,
`#` comments, `source`/`.`, redirections on function calls, grouped expressions
as pipeline stages, full job control (`fg` waits rather than resuming a process
group), POSIX signal handling, macOS/Windows release builds in CI (only Linux
GNU/musl are published).

Future KERNEXT Web and Desktop are plans only. Do not start them unless asked,
and never present them as implemented.

## Build gotcha: pinned target dir

`.cargo/config.toml` pins
`build.target-dir = "/home/trishit/.cache/terminal-target"` — a workaround for a
vfat-with-`showexec` checkout on the original Linux host, where extensionless
build scripts and test binaries cannot be executed.

- On a non-Linux checkout that path is meaningless. Override per command with
  `CARGO_TARGET_DIR=/path/to/ext4-or-local-target cargo test …` rather than
  editing the file, unless the user asks.
- Do not create a repo-local `target/`, and do not add a target dir or
  OS-specific linker settings to `Cargo.toml`.

## Tests

Add tests next to the behavior: unit tests in `src/`, integration tests in
`crates/shell-builtins/tests/`, `crates/shell-core/tests/`,
`apps/shell-cli/tests/`. The full suite is 469 tests on Windows (467
unit/integration plus 2 `shell-core` doc-tests), 20 of which
live in `crates/shell-builtins/tests/windows_scripts.rs` and only build on
Windows. The Unix baseline was 399 before the Windows-only suites were added
(Unix covers signal/abort termination instead).

- Filesystem tests use isolated temp dirs plus `Session::set_working_directory`;
  they never mutate the process cwd.
- External-process and pipeline tests use the test binary itself as a helper via
  the `CUSTOM_SHELL_TEST_HELPER` / `CUSTOM_SHELL_PIPELINE_ARGS` env vars, so no
  particular program (`python`, `grep`, `sort`, `git`) has to be installed.
  Keep them that way.
- Never weaken, skip or delete tests to make a build pass.

## Git

Eight commits on `main`; all feature work after `first commit` is uncommitted
and many files are untracked/modified. Never run `git reset --hard`,
`git checkout -- .`, `git clean` or anything that discards working-tree changes.
Do not commit, amend, rebase or push unless asked.
