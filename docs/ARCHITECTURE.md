# KERNEXT Architecture

This document describes the implemented CLI architecture and the intended
reuse boundary for future KERNEXT Web and Desktop surfaces. Web and Desktop are
**plans only**; no code for them exists in this repository.

## Current CLI data flow

```text
input
  -> shell-core lexer           (tokens, quotes, spans)
  -> shell-core parser          (AST)
  -> shell-builtins evaluator   (lists, groups, &&, ||, loops, functions)
  -> shell-builtins pipeline    (stages, OS pipes, redirections)
  -> built-in registry OR external process executor
  -> CommandOutcome             (action, output, success, exit code)
  -> shell-cli REPL             (startup, output, clearing, shutdown)
```

The application never splits input with `split_whitespace`. The dispatcher uses
`lex_spanned` plus `parse` for every command line.

## Crates

| Crate | Responsibility | Terminal dependency |
| --- | --- | --- |
| `shell-core` | Branding constants, `ShellError`, lexer, parser, AST, spans | None |
| `shell-config` | Typed `Config`, validation, TOML load/save, default paths | None |
| `shell-runtime` | `Session`, `Platform`, working directory, variables, lifecycle | None |
| `shell-builtins` | Registry, evaluator, native commands, pipelines, process executor | stdio only |
| `shell-line-editor` | Reedline integration, prompt, history, terminal clearing | Reedline/crossterm |
| `shell-cli` | Clap arguments, KERNEXT startup, REPL orchestration | Terminal |

Dependency direction:

```text
shell-cli ──> shell-builtins ──> shell-runtime ──> shell-config ──> shell-core
    │                                │
    │                                └──────────> shell-core
    ├──────────> shell-line-editor ──> reedline / crossterm
    └──────────> shell-core
```

## Reusable core boundary

The engine is designed so a non-terminal front end can reuse it:

- `shell-core` performs no I/O and owns parsing.
- `shell-builtins::Dispatcher` accepts a command line or AST and returns a
  `CommandOutcome`.
- `CommandOutcome` exposes combined display output plus `success`, `exit_code`,
  and a `CommandAction`. Internal command results keep stdout and stderr
  separate so redirections can target them individually.
- `shell-runtime::Session` is cloneable, which lets pipeline stages run against
  an isolated copy while filesystem side effects persist.

A future front end should call this boundary rather than reimplementing shell
semantics. The CLI remains the reference implementation.

### What is stable

- `SHELL_NAME`, `PRODUCT_NAME`, `TAGLINE`, `VALUES`, `MOTTO`, and `VERSION`.
- Lexer/parser entry points (`lex`, `lex_spanned`, `parse`).
- `Dispatcher`, `CommandOutcome`, `CommandAction`.
- `Session` construction, working directory, and variable APIs.
- `Config` fields and TOML format.

### What is terminal-specific

- Startup animation and cursor control (`apps/shell-cli/src/startup.rs`).
- Prompt rendering and line editing (`shell-line-editor`).
- Screen clearing (`shell-line-editor::clear_terminal`).
- Process-wide stdin/stdout/stderr behavior in `shell-builtins::process`.

A Web or Desktop surface should supply its own renderer and its own I/O
transport while reusing parsing, evaluation, and session state.

## Execution semantics

### Built-in precedence

`eval.rs` checks the native registry before external resolution. A `PATH`
program named `echo`, `cd`, or `pwd` can never override a native command.

### Pipelines

- External stages use real OS pipes: `ChildStdout` becomes the next stage's
  `Stdio::from`.
- Built-ins run in-process and exchange buffered bytes with neighbours.
- Buffered data is written to an external child on a detached writer thread;
  callers join writers and reap children.
- Built-ins inside pipelines use a cloned `Session`, so `cd` and variable
  changes do not leak to the parent.
- The final stage's status determines pipeline success and exit code.
- Intermediate `SIGPIPE` termination is not an error.
- Grouped stages are rejected with
  `"Grouped pipelines are not supported yet."`; standalone groups still run.

### Redirections

Supported kinds: `Input`, `Output`, `Append`, `Error`, `ErrorAppend`. Paths are
resolved through the session working directory. A redirection applies only to
its own stage.

### Actions

`CommandAction::{Continue, ClearScreen, ClearHistory, Exit}` propagates from a
single command. No action propagates from a pipeline stage, and background
evaluation cannot stop or clear the foreground shell.

### Status and exit codes

- `$?` is updated after every command, pipeline, and assignment: `0` on success,
  the child's code for a normal external exit, `1` for a built-in failure, `2`
  for lexer/parser errors, and `128 + signal` for a signal-terminated child.
- `exit [status]` sets both `$?` and the shell process exit status.
- The final pipeline stage determines pipeline success and exit status.
- Control-flow no-ops (a false `if` without `else`, an empty loop, a function
  definition, or `break`/`continue`/`return`) reset a stale failure status to
  success rather than leaking the previous command's result.

## Startup architecture

`apps/shell-cli/src/startup.rs` is presentation-only:

1. `resolve_mode` chooses `Animated`, `Plain`, or `Quiet`.
2. `Startup::intro` renders the logo, version, tagline, and values.
   In `Animated` mode the wordmark is redrawn in place for a bounded number of
   frames while a palette-derived 24-bit gradient travels across each character.
3. `cli::run` emits each checklist stage only after the real subsystem is
   constructed:
   - `Session::new`
   - history path preparation
   - `Dispatcher::new`
   - `ShellEditor`
   - `Repl`
4. `Startup::welcome` restores the cursor and prints the welcome message.
5. `Startup` implements `Drop` to restore the cursor and reset colors even on
   early error paths.

The animation never invents progress. It only reveals real stages. The palette
comes from `Config::theme()`; colors are validated at config load time.
`terminal_font` / `apply_terminal_font` opt into an OSC 50 hint, which many
terminals ignore.

## Configuration and paths

`Config::default_path()` resolves:

- Linux: `$XDG_CONFIG_HOME/kernext/config.toml` or `~/.config/kernext/...`
- macOS: `~/Library/Application Support/kernext/config.toml`
- Windows: `%APPDATA%\kernext\config.toml`

`Config::load_or_default` returns defaults for a missing file and a recoverable
error for an invalid file. The CLI warns and falls back to defaults on parse
failure.

History is stored beside the config file unless `history_file` is absolute.

## Security properties

- No other shell parses KERNEXT input: no `sh -c`, `bash -c`, or `cmd /C` for
  command syntax. The lexer and parser always own tokenisation and pipelines.
- Arguments are passed as separate `Command::arg` values to native programs.
- Windows `.cmd`/`.bat` files cannot be started natively, so they are launched
  through the system command interpreter with `/d /v:off /s /c` and one
  pre-escaped command line. Arguments containing `"`, `%`, CR, LF, or NUL are
  rejected rather than altered, because `cmd.exe` has no escaping for them.
- Windows resolution honours `PATHEXT` order but only for `.com`, `.exe`,
  `.bat`, and `.cmd`; extensionless files are never treated as executables.
  This prevents launching npm/pnpm POSIX `sh` shims (the cause of Windows
  `os error 193`).
- The parent process environment is never mutated.
- Child processes receive inherited variables plus exported locals, minus
  hidden names.
- Relative paths resolve against the session working directory, not a global
  process mutation.

## Build environment

The canonical checkout is on a VFAT mount with `showexec`. `.cargo/config.toml`
sets `build.target-dir = "/home/trishit/.cache/terminal-target"` so build
artifacts land on ext4. Do not create a repo-local `target/` directory.

## Future: KERNEXT Web (planned)

Proposed shape, not implemented:

```text
browser UI
  -> WebSocket/HTTP transport
  -> KERNEXT engine service
       -> shell-core parser
       -> shell-builtins evaluator
       -> sandboxed execution backend
  -> streamed stdout/stderr/outcomes
```

Open questions: execution isolation, filesystem model, authentication, and how
interactive line editing maps to the browser. A graphical splash and mountain
wallpaper belong here, not in the CLI.

## Future: KERNEXT Desktop (planned)

Proposed shape, not implemented:

```text
native window / terminal surface
  -> shared line-editor abstraction
  -> shell-core parser
  -> shell-builtins evaluator
  -> local process executor
```

The desktop surface should reuse the same engine boundary as the CLI and Web,
with a platform-native renderer. No desktop code exists today.
