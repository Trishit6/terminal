# KERNEXT — project knowledge

## What this is

KERNEXT is an independent, cross-platform command-line shell written in Rust. It
has its own lexer, parser, AST, evaluator, built-in commands, and external process
executor — it does **not** wrap `sh`, `bash`, PowerShell, or `cmd`. The same Rust
engine powers three surfaces: the CLI (`apps/shell-cli`), a local Axum web server
+ React front end (`apps/shell-server`, `apps/web`), and a Tauri 2 desktop app
(`apps/desktop`). Branding is user-facing only: product/executable is `kernext`;
every crate, package and Rust identifier stays `shell-*`. Do not rename crates.

The Cargo workspace **is** this project root (no nested workspace directory).

## Where the code lives

| Path | Responsibility |
| --- | --- |
| `crates/shell-core` | Branding constants, `ShellError`, lexer/tokenizer, parser, AST, spans |
| `crates/shell-config` | Typed `Config`, color/theme validation, TOML load/save, platform config path |
| `crates/shell-runtime` | `Session` (cwd, variables, functions, last status), `Platform`, history, `JobTable` |
| `crates/shell-builtins` | Registry, AST evaluator, native commands, pipelines, external executor, expansion |
| `crates/shell-line-editor` | Reedline wrapper, prompt, completer, history backing, screen clearing |
| `crates/shell-api` | Serializable session/terminal API contracts shared by server + desktop |
| `apps/shell-cli` | Clap args, startup presentation, REPL, project commands (`dev.rs`) |
| `apps/shell-server` | Local Axum server: HTTP sessions, engine + PTY WebSockets |
| `apps/web` | React + Vite + Tailwind v4 front end shared by web + desktop |
| `apps/desktop` | Tauri 2 desktop app (`src-tauri` crate root) |

Execution flow — keep every command line on this path:

```
input -> lexer -> parser -> AST -> evaluator -> pipeline engine
      -> built-in registry OR external process executor -> CommandOutcome -> Repl
```

Never split input with `split_whitespace`; the dispatcher must use `lex_spanned` + `parse`.

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

Or via the `kernext` project commands: `kernext build|check|test|fmt|lint|run|clean|doctor|version|help`.

Front end:

```sh
npm ci
npm run web:dev          # vite dev server http://127.0.0.1:5173
npm run web:typecheck
npm run web:build
npm test                 # web Vitest suite
```

Focused tests:

```sh
cargo test -p shell-builtins --test builtins
cargo test -p shell-builtins --test external
cargo test -p shell-builtins --test pipeline
cargo test -p shell-core --test parser
cargo test -p shell-cli --test repl
cargo test -p shell-server
cargo test -p shell-desktop
```

No install step; Rust 1.85+. Interactive use requires a real TTY.

## Conventions and hard constraints

- Never let another shell parse KERNEXT input: no `sh -c`, `bash -c`, `cmd /C`,
  `powershell -Command` for command syntax. `process.rs` passes each argument as
  a separate `Command::arg` to the resolved executable path.
- Built-ins always win over functions, functions over external programs. A `PATH`
  program named `echo`/`cd`/`pwd` must never override a native command.
- `cd` mutates `Session::working_directory` only, never process-wide cwd.
  `commands::resolve_path` is the single resolver for relative paths.
- Built-ins inside a real pipeline run on a cloned `Session`: filesystem effects
  persist, but `cd`/`export`/`unset` must not leak to the parent. A lone command
  still mutates the real session.
- Pipelines use real OS pipes (`ChildStdout` → next stage's `Stdio::from`).
  Never buffer a whole external stream just to connect stages; join writer threads
  and reap every child, including on the error path. Final stage decides
  success/exit code; intermediate `SIGPIPE` is not an error.
- `exit` and `clear` actions propagate only from a single command, never from a
  pipeline stage. `clear`/Ctrl+L writes CSI sequences via crossterm and is
  suppressed when stdout is not a terminal.
- Grouped pipeline stages are rejected with `"Grouped pipelines are not supported yet."`.
- Expansion happens at evaluation time, one word → one argument (no field
  splitting, no re-parsing). `$name`, `${name}`, `$?`, `$$`; single quotes and
  `\$` stay literal. Undefined variables expand to empty.
- `$?` updated after every command/pipeline/assignment: 0 success, external code,
  1 built-in failure, 2 syntax error, 128+signal.
- Cross-platform only: `std::path`, `HOME`/`USERPROFILE`, no Unix-only APIs
  outside `#[cfg(unix)]`.
- Windows: `PATHEXT` honoured but restricted to `.com`, `.exe`, `.bat`, `.cmd`;
  extensionless files are never executables. `.cmd`/`.bat` run through
  `%COMSPEC%`/`System32\cmd.exe` with `/d /v:off /s /c`. Script args containing
  `"`, `%`, CR, LF, or NUL are refused.
- Startup (`apps/shell-cli/src/startup.rs`) is presentation-only: never parse,
  evaluate or launch commands; never fake progress or add artificial delays.
- Never hardcode the version; use `VERSION` / `Session::version()`.
- Emblem rows must all have identical character counts
  (`emblem_rows_keep_a_constant_display_width`).
- Internal crate/package names stay `shell-*`; do not rename.

## Build gotcha: pinned target dir

`.cargo/config.toml` pins `build.target-dir = "/home/trishit/.cache/terminal-target"`
as a workaround for a vfat-with-`showexec` checkout where extensionless build
scripts and test binaries cannot execute.

- On a non-Linux checkout that path may be meaningless; override per command with
  `CARGO_TARGET_DIR=/path/to/local-target cargo test …` rather than editing the
  file unless asked.
- Do not create a repo-local `target/`, and do not add a target dir or
  OS-specific linker settings to `Cargo.toml`.
- Do not change mount options without explicit permission.

## Web front end notes

- Tailwind CSS v4 is integrated via `@tailwindcss/vite` in `apps/web/vite.config.ts`.
  Do **not** add a `postcss.config.js` (it breaks because `package.json` has
  `"type": "module"` and Tailwind v4 needs no PostCSS config). Tailwind config
  lives in `apps/web/src/styles.css` (`@import "tailwindcss"`, `@theme`, etc.).
- `apps/desktop` reuses `apps/web`; its `web:build` delegates to the web
  package.

## Tests

Add tests next to the behavior: unit tests in `src/`, integration tests in
`crates/shell-builtins/tests/`, `crates/shell-core/tests/`,
`apps/shell-cli/tests/`, plus `apps/shell-server` and `apps/shell-desktop`.
Filesystem tests use isolated temp dirs plus `Session::set_working_directory`;
they never mutate the process cwd. External-process and pipeline tests use the
test binary itself as a helper via `CUSTOM_SHELL_TEST_HELPER` /
`CUSTOM_SHELL_PIPELINE_ARGS` — keep them that way. Never weaken, skip or delete
tests to make a build pass.

## Git

Never run `git reset --hard`, `git checkout -- .`, `git clean`, or anything that
discards working-tree changes. Do not commit, amend, rebase or push unless asked.