# custom-terminal

An independent, cross-platform command-line shell written in Rust. It runs
inside an existing terminal and contains its own lexer, parser, AST, command
dispatcher, built-in commands, and external process executor. It is not a
wrapper around Bash, PowerShell, or CMD.

> **Status:** Phase 8 is complete. The shell parses input with its own lexer and
> parser, runs native built-ins directly, launches non-built-in commands as
> external processes through `std::process::Command`, connects pipeline stages
> with real operating-system pipes, applies input/output/error redirections,
> evaluates `&&`, `||`, and `;`, and implements native variable assignment,
> expansion, and environment management (`export`, `unset`, `env`, `set`).
> Job control and scripting are intentionally not implemented yet.

## Overview

The shell provides:

- an interactive Read-Eval-Print Loop (REPL) with a custom prompt,
- Reedline-backed line editing, history, arrow keys, and Ctrl+R search,
- a custom lexer/tokenizer with quote metadata and source spans,
- a custom recursive-descent parser and AST,
- 21 native built-in commands,
- direct external process execution using the system `PATH`,
- pipelines connected by real operating-system pipes,
- input, output, append, and error redirections (`<`, `>`, `>>`, `2>`, `2>>`),
- correct argument boundaries for quoted and unquoted words,
- current-working-directory and environment propagation,
- inherited terminal I/O for interactive child programs,
- exit-status propagation for future scripting,
- clear errors for missing programs, permission failures, and spawn failures,
- command lists (`;`), groups, and `&&`/`||` evaluation,
- graceful Ctrl+C cancellation and Ctrl+D/EOF shutdown,
- native variable assignment (`name = value`),
- variable expansion (`$name`, `${name}`, `$?`, `$$`),
- environment management (`export`, `unset`, `env`, `set`).

## Supported platforms

The implementation targets Linux, macOS, and Windows and keeps
platform-specific code isolated:

- Executable lookup uses `PATH` on every platform.
- Windows executable resolution reads `PATHEXT` dynamically instead of
  hardcoding extensions.
- Unix executability is checked through the file mode's execute bits.
- Home-directory resolution uses `HOME` or `USERPROFILE`/`HOMEDRIVE`.
- No Unix-only APIs are used outside `#[cfg(unix)]` blocks.

## Requirements

- Rust 1.85 or newer (Rust 2024 edition).
- A working platform linker (a C toolchain on Linux/macOS, MSVC Build Tools or
  MinGW-w64 on Windows).

## Build and run

```sh
cargo build --workspace
cargo run -p shell-cli
```

The binary is `custom-shell` (`custom-shell.exe` on Windows). `--help` and
`--version` are handled by Clap without starting a session.

### VFAT / `showexec` mount note

This project lives on a VFAT filesystem mounted with `showexec`.
`showexec` only allows execution of files with `.exe/.com/.bat/.btm/.cmd` extensions,
which breaks Cargo's build scripts and test binaries (extensionless).

**Permanent fix:** `.cargo/config.toml` sets `build.target-dir = "/home/trishit/.cache/terminal-target"`.
All Cargo commands automatically use the ext4 target directory on the host system.
No `export CARGO_TARGET_DIR` is required.

```sh
# Just run cargo commands directly — the config handles it
cargo check --workspace --all-targets
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p shell-cli
```

Do not create a `target/` directory inside the repository.

## Architecture

```text
input
  -> shell-core lexer           (tokens, quotes, spans)
  -> shell-core parser          (AST)
  -> shell-builtins evaluator   (lists, groups, &&, ||)
  -> shell-builtins pipeline    (stages, OS pipes, redirections)
  -> built-in registry OR external process executor
  -> CommandOutcome             (action, output, success, exit code)
  -> shell-cli REPL             (output, clearing, shutdown)
```

```text
terminal/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── AGENTS.md
├── crates/
│   ├── shell-core/         # Identity, errors, lexer, parser, AST
│   ├── shell-runtime/      # Session, platform, working directory, lifecycle
│   ├── shell-builtins/     # Registry, evaluator, built-ins, process executor
│   ├── shell-config/       # Typed configuration and TOML persistence
│   └── shell-line-editor/  # Reedline integration, prompt, history
└── apps/
    └── shell-cli/          # Binary entry point and REPL orchestration
```

| Crate | Responsibility |
| --- | --- |
| `shell-core` | Application identity, `ShellError`, lexer/tokenizer, parser, AST, spans |
| `shell-runtime` | `Platform`, `Session`, working directory, version, running/stopped state |
| `shell-builtins` | Built-in registry, AST evaluator, native commands, pipeline engine, external process execution |
| `shell-config` | Typed `Config`, validation, TOML load/save, default path |
| `shell-line-editor` | Reedline wrapper, prompt rendering, history, terminal clearing |
| `shell-cli` | Clap arguments, banner, REPL loop, error reporting |

Dependency direction:

```text
shell-cli ──> shell-builtins ──> shell-runtime ──> shell-config ──> shell-core
    │                                │
    │                                └──────────> shell-core
    ├──────────> shell-line-editor ──> reedline / crossterm
    └──────────> shell-core
```

## Command execution

### Built-ins take precedence

Every simple command is parsed first. The dispatcher checks the native registry
before attempting external resolution:

1. Parse the command line with the shell lexer and parser.
2. If the command name is a registered built-in, run the native implementation.
3. Otherwise, resolve the name and launch it with `std::process::Command`.
4. Return an outcome containing output, success, and the child exit code.

A program named `echo`, `cd`, or `pwd` on `PATH` never overrides the native
command. This precedence is deliberate and documented in
`crates/shell-builtins/src/eval.rs`.

Built-in commands:

| Command | Behavior |
| --- | --- |
| `help` | Lists native commands and notes that other names run externally. |
| `pwd` | Prints the session working directory. |
| `cd` | Changes and validates the session working directory. |
| `ls` | Lists directory contents (directories get a trailing separator). |
| `mkdir` | Creates directories, with `-p`/`--parents`. |
| `touch` | Creates files or updates modification times. |
| `cat` | Prints file contents, or standard input when no file is given. |
| `echo` | Prints arguments separated by spaces. |
| `cp` | Copies files, with `-r`/`--recursive` for directories. |
| `mv` | Moves or renames files and directories. |
| `rm` | Removes entries, with `-r`/`-R` and `-f`. |
| `clear` | Clears the terminal through the line editor. |
| `history` | Shows the in-memory command history. |
| `whoami` | Shows the current user name. |
| `date` | Shows the local date and time. |
| `version` | Prints `Custom Shell v<version>`. |
| `exit` | Prints a farewell and shuts down. |
| `export` | Marks variables for export to child processes. |
| `unset` | Removes shell variables. |
| `env` | Shows the environment for child processes. |
| `set` | Shows shell variables (local, exported, inherited). |

### External process execution

The executor lives in `crates/shell-builtins/src/process.rs`. It never sends a
command line to another shell:

- It does **not** use `sh -c`, `bash -c`, `cmd /C`, or `powershell -Command`.
- It does **not** join arguments into a string.
- Arguments decoded by the shell lexer are passed to the operating system as
  separate `Command::arg` values.
- The resolved executable path is passed to `Command::new` directly.

Executable resolution:

- Absolute paths run directly.
- Names containing a path separator are resolved against the session working
  directory (for example `./script` or `subdir/program`).
- Bare names are searched in `PATH`.
- On Windows, `PATHEXT` is read dynamically. `.bat` and `.cmd` files are
  deliberately skipped so no other shell is involved.
- On Unix, a candidate must be a regular file with at least one execute bit.

Argument handling:

```text
echo "Hello World"      -> one argument: Hello World
echo 'Hello World'      -> one argument: Hello World
echo Hello World        -> two arguments: Hello, World
```

Quotes are removed by the lexer and never reach the child process. Spaces
inside quotes are preserved.

### Terminal input and output

- By default, child processes inherit the shell's stdin, stdout, and stderr.
  Output streams directly to the terminal, so long-running programs stay
  responsive and TTY-aware programs keep working.
- The REPL does not buffer child output in memory.
- `StdioMode::Capture` is available for tests and non-interactive callers. It
  pipes stdout/stderr and closes stdin so captured commands cannot block on
  input.
- Child processes run in the session working directory and inherit the process
  environment. `ProcessOptions` can layer environment overrides without
  modifying the shell's global environment.

### Pipelines and redirections

The pipeline engine lives in `crates/shell-builtins/src/pipeline.rs`. A pipeline
runs its stages left to right:

- External stages are connected with real operating-system pipes. A stage's
  `ChildStdout` becomes the next stage's standard input, so data is streamed
  rather than fully buffered.
- Built-ins run in-process. A built-in producer buffers its output only until
  the next stage starts; a built-in consumer reads the previous stage to the
  end. Buffered data is fed to an external child on a writer thread so pipeline
  construction never blocks on a full pipe.
- Built-ins inside a real pipeline run against a cloned `Session`. Filesystem
  side effects persist, but state changes such as `cd` do not leak back into the
  parent shell.
- The final stage determines the pipeline's `success` value and exit code.
  Intermediate stages are waited for, and their normal `SIGPIPE` termination is
  not reported. Failures to start, open, or wait are reported with the stage
  name.
- If a stage cannot start, already-started children are killed and reaped before
  the error is returned.

Supported redirections:

| Syntax | Behavior |
| --- | --- |
| `< file` | Reads standard input from `file`. |
| `> file` | Creates or truncates `file` and writes standard output. |
| `>> file` | Creates `file` if needed and appends standard output. |
| `2> file` | Creates or truncates `file` and writes standard error. |
| `2>> file` | Creates `file` if needed and appends standard error. |

Paths are resolved against the session working directory unless absolute.
Quoted paths keep spaces. A redirection on a pipeline stage applies to that
stage only, so `echo x | cat > out.txt` writes the final stage's output to
`out.txt`. Open failures are reported and the REPL continues.

`2>` and `2>>` are recognised only when `2` starts a token. `file2>log` is the
word `file2` plus an output redirection to `log`.

### Logical chaining

- `left && right` runs `right` only when `left` succeeds.
- `left || right` runs `right` only when `left` fails.
- `left ; right` always runs both.
- Pipelines bind more tightly than `&&` and `||`, so `a | b && c` means
  `(a | b) && c`.
- Operators inside quotes are ordinary argument text.

### Errors and exit status

External failures never terminate the REPL. Representative messages:

```text
Command not found: unknown-command
permission denied: ./not-executable
failed to start 'program': <operating-system error>
invalid executable path 'bad': it contains a NUL byte
```

`CommandOutcome` exposes `success()` and `exit_code()`. The exit code is
`None` for built-ins and for processes terminated by a signal; `success()` is
the value `&&`, `||`, and `;` use and the value a future scripting engine can
expose as `$?`. For a pipeline, the final stage's status is authoritative.

## Shell language

### Lexer (Phase 3 / 3.1)

The lexer converts input into tokens with one-based line and Unicode-scalar
column positions:

- `Word`, `String`, operators (`|`, `>`, `>>`, `2>`, `2>>`, `<`, `;`, `&&`,
  `||`, `(`, `)`), `Variable`, `SpecialVariable`, `Equals`, `Newline`, and
  `EndOfInput`.
- Single and double quotes, with quote style retained in the spanned-token API.
- Escapes: `\\`, `\"`, `\'`, `\$`, `\n`, `\t`, `\r`, `\ `, and escaped
  operator characters.
- `$name` and `${name}` variables are tokenized for expansion.
- `$?` and `$$` are tokenized as special variables.
- `=` is a distinct token to support assignment syntax.

### Parser and AST (Phase 4)

```text
program        := list EOF
list           := (NEWLINE)* expression (separator expression)* separator?
separator      := ';' | NEWLINE

expression     := operand (('&&' | '||') operand)*
operand        := assignment | pipeline
assignment     := NAME '=' word?
pipeline       := stage ('|' stage)*
stage          := simple_command | group

group          := '(' list ')'
simple_command := command_element+
command_element:= word | redirection
redirection    := ('<' | '>' | '>>' | '2>' | '2>>') word
word           := WORD | STRING | VARIABLE | SPECIAL | '=' | adjacent word segments
```

Words remain a sequence of parts (`Unquoted`, `SingleQuoted`, `DoubleQuoted`,
`Variable`, `Special`) so later phases can perform concatenation and expansion
without re-parsing. The AST preserves source spans for every node.

### Variable expansion (Phase 8)

Variables are assigned with `name = value` (spaces around `=` are optional).
The assignment is parsed by the shell's own lexer/parser and never executed as
an external program.

```text
name = Trishit
age = 21
project = "My Shell"
empty =
```

Expansion occurs at evaluation time in command arguments, redirection targets,
and pipeline stages:

| Syntax | Behavior |
| --- | --- |
| `$name` | Expands the variable `name`. |
| `${name}` | Braced form; allows adjacent text. |
| `$?` | Expands to the last command's exit status. |
| `$$` | Expands to the shell's process ID. |

Quote behavior:

- Unquoted `$name` expands.
- Double-quoted `"$name"` expands.
- Single-quoted `'$name'` remains literal.
- Escaped `\$name` remains literal.

Undefined variables expand to an empty string. Argument boundaries are preserved:
a value containing spaces does not split into multiple arguments.

Four new built-ins manage the environment:

| Command | Behavior |
| --- | --- |
| `export` | Marks variables for export to child processes. |
| `unset` | Removes shell variables (local and exported). |
| `env` | Shows the effective environment passed to children. |
| `set` | Shows local, exported, and inherited variables separately. |

Environment variables from the parent process are read at startup. Shell-local
variables are distinct from exported ones. Child processes receive only exported
variables plus non-overridden inherited variables. The shell never modifies the
parent process environment.

Special variables:

- `$?` — Updated after every command, pipeline, and assignment (0 on success,
  external exit code, 1 for built-in failure, 2 for syntax errors, 128+signal
  for signal termination).
- `$$` — Current shell process ID.
- `$PWD` — Synchronized with successful `cd` operations.
- `$HOME` — Initialized from the inherited environment or `USERPROFILE`.

## What works today

- Native built-ins, including filesystem operations.
- External programs with absolute, relative, and `PATH`-resolved names.
- Quoted and unquoted arguments with preserved boundaries.
- Working-directory and environment propagation.
- Inherited terminal I/O.
- Successful and non-zero exit statuses.
- Pipelines with two or more stages, including built-in producers, consumers,
  and middle stages.
- Input, output, append, and error redirections, including quoted paths.
- Command lists (`;`), parenthesized groups, and `&&`/`||` short-circuiting.
- REPL survival after unknown commands, missing files, and failed processes.
- Variable assignment, expansion, and environment management (`export`,
  `unset`, `env`, `set`).

## Not implemented yet

- Shell scripts and sourcing.
- Background jobs and job control.
- Globbing, comments, and autocomplete.
- Persistent history and startup configuration loading.
- Grouped expressions as pipeline stages (`(a; b) | c`). Groups still run
  standalone; the evaluator reports a clear error when a group is used as a
  pipeline stage.
- Built-in `cd` updates the session directory, not the process-wide directory.

These are reserved for later phases.

## Configuration

`shell-config` defines and persists:

```toml
prompt = "custom-shell > "
history_size = 1000
color = false
```

Missing fields use defaults, unknown fields are rejected, and `history_size`
must be greater than zero. The CLI currently starts with in-memory defaults;
loading a configuration file at startup is planned later.

## Development

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Focused test commands:

```sh
cargo test -p shell-builtins --test external
cargo test -p shell-builtins --test builtins
cargo test -p shell-builtins --test pipeline
cargo test -p shell-core --test parser
cargo test -p shell-cli --test repl
```

The current suite has **306 tests**. External-process and pipeline tests use the
test binary itself as a portable helper, so they do not depend on `python`,
`git`, `sort`, `grep`, or any other particular program being installed.

## Implementation status

- **Phase 1:** Workspace, CLI startup, banner, Clap arguments, shared errors.
- **Phase 2:** REPL, prompt, Reedline editing/history, session lifecycle.
- **Phase 3 / 3.1:** Lexer, tokens, quoting, spans, Unicode-safe positions.
- **Phase 4:** Recursive-descent parser, AST, precedence, parse errors.
- **Phase 5:** Built-in registry, AST evaluator, 17 native commands, session cwd.
- **Phase 6:** Direct external process execution, executable resolution,
  argument preservation, stdio strategy, exit statuses, error handling.
- **Phase 7:** Pipeline engine with real OS pipes, built-in I/O interop,
  input/output/error redirections, and `&&`/`||`/`;` chaining over pipelines.
- **Phase 8:** Native variable system with assignment (`name = value`), expansion
  (`$name`, `${name}`, `$?`, `$$`), environment management (`export`, `unset`,
  `env`, `set`), special variables, and child process environment inheritance.

## License

No license has been selected for this project yet.
