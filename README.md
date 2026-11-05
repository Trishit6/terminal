# KERNEXT

**The Next Core of Computing.**  
**OPEN • POWERFUL • INDEPENDENT** — *Build. Execute. Innovate.*

KERNEXT is an independent, cross-platform command-line shell written in Rust.
It runs inside an existing terminal and contains its own lexer, parser, AST,
command dispatcher, built-in commands, and external process executor. It is not
a wrapper around Bash, PowerShell, or CMD.

> **Status:** The KERNEXT CLI is implemented. It parses input with its own lexer
> and parser, runs native built-ins directly, launches non-built-in commands as
> external processes through `std::process::Command`, connects pipeline stages
> with real operating-system pipes, applies input/output/error redirections, and
> evaluates `&&`, `||`, and `;`. Native variable assignment, expansion,
> environment management, loops, conditionals, functions, `return`, `break`, and
> `continue` are implemented, and configuration is loaded at startup.
> Background `&` syntax is evaluated synchronously but isolated from the parent
> shell; detached jobs and job control are still incomplete. See
> [Not implemented yet](#not-implemented-yet).

## Brand

| | |
| --- | --- |
| Product | **KERNEXT** |
| Tagline | **The Next Core of Computing.** |
| Values | **OPEN • POWERFUL • INDEPENDENT** |
| Support phrase | **Build. Execute. Innovate.** |
| Prompt | `kernext ~ $ ` |
| Executable | `kernext` (`kernext.exe` on Windows) |

The internal Rust crates and package names (`shell-cli`, `shell-core`,
`shell-builtins`, and so on) remain unchanged so the engine stays stable and
reusable. Only the user-facing product name and executable are KERNEXT.

The CLI is intentionally text-only. A graphical splash, mountain wallpaper, and
richer visual surfaces belong to the planned Web and Desktop products, not to
this terminal binary.

## Overview

The shell provides:

- an interactive Read-Eval-Print Loop (REPL) with the `kernext ~ $ ` prompt,
- a KERNEXT startup wordmark with a continuous color-changing gradient,
- a themeable palette for the wordmark, checklist, prompt, and status text,
- Reedline-backed line editing, history, arrow keys, and Ctrl+R search,
- Home/End and Page Up/Page Down navigation on Linux, macOS, and Windows,
- a custom lexer/tokenizer with quote metadata and source spans,
- a custom recursive-descent parser and AST,
- 21 native built-in commands,
- direct external process execution using the system `PATH`,
- pipelines connected by real operating-system pipes,
- input, output, append, and error redirections (`<`, `>`, `>>`, `2>`, `2>>`),
- correct argument boundaries for quoted and unquoted words,
- current-working-directory and environment propagation,
- inherited terminal I/O for interactive child programs,
- clear errors for missing programs, permission failures, and spawn failures,
- command lists (`;`), groups, and `&&`/`||` evaluation,
- graceful Ctrl+C cancellation and Ctrl+D/EOF shutdown,
- native variable assignment (`name = value`),
- variable expansion (`$name`, `${name}`, `$?`, `$$`),
- environment management (`export`, `unset`, `env`, `set`),
- `if`/`else`, `while`, `for`, functions, recursion, and loop control,
- genuine `clear` and Ctrl+L screen clearing,
- TOML configuration loaded from the platform config directory,
- optional OSC 50 terminal-font hints for terminals that honor them.

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
- A real terminal for the interactive shell. Piping input into KERNEXT reports
  that standard input is not a terminal.

## Build and run

```sh
cargo build --workspace
cargo run -p shell-cli
```

The binary is `kernext` (`kernext.exe` on Windows). `--help` and `--version`
are handled by Clap without starting a session.

### Startup flags

```text
kernext                  # animated startup on a real terminal
kernext --no-animation   # same information, no reveal animation
kernext --quiet          # skip the startup banner entirely
```

`--help` and `--version` also go through Clap and do not start a session.

### Installers

The scripts in `installers/` download a published release, verify its SHA-256
checksum when one is available, and install the `kernext` binary. Run them with
the interpreter rather than relying on the executable bit:

```sh
bash installers/install-linux.sh
bash installers/install-macos.sh
powershell -File installers/install-windows.ps1
```

The installers animate their banner and show progress while real download,
verification, extraction, and installation work runs. Animation is disabled
automatically for piped output, `NO_COLOR`, and `TERM=dumb`. Set
`KERNEXT_VERSION` (or pass `-Version` on Windows) to install a different
release.

The release workflow currently publishes Linux GNU and musl archives. The macOS
and Windows installers expect matching archives, which must be produced by a
host with the corresponding toolchain until those CI jobs are added.

### VFAT / `showexec` mount note

This project lives on a VFAT filesystem mounted with `showexec`.
`showexec` only allows execution of files with `.exe/.com/.bat/.btm/.cmd` extensions,
which breaks Cargo's build scripts and test binaries (extensionless).

**Permanent fix:** `.cargo/config.toml` sets
`build.target-dir = "/home/trishit/.cache/terminal-target"`. All Cargo commands
automatically use the ext4 target directory on the host system. No
`export CARGO_TARGET_DIR` is required.

```sh
# Just run cargo commands directly — the config handles it
cargo check --workspace --all-targets
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p shell-cli
```

Do not create a `target/` directory inside the repository.

## Startup experience

The startup sequence lives in `apps/shell-cli/src/startup.rs` and is deliberately
separate from the execution engine. It renders text and terminal-control
sequences only; it never parses, evaluates, or launches commands.

Every checklist line is emitted at the point where the corresponding subsystem
has actually finished initializing:

```text
[OK] Initializing KERNEXT Core
[OK] Preparing Runtime Environment
     KERNEXT v0.1.0  ·  Linux x86_64  ·  /current/directory
[OK] Loading Command Engine
[OK] Configuring Shell Interface
[OK] Starting Session
```

The animation is a presentation layer, not a fake boot delay. On a real
terminal the wordmark is drawn in place for a short, bounded sequence of frames
while a 24-bit gradient travels across every character. The colors are derived
from elapsed time and the configured palette, so the reveal is continuously
color-changing rather than a fixed two-tone logo. Initialization stages are
still reported only after the corresponding subsystem actually finishes.

### Theme and navigation

The palette is configurable with named ANSI colors, `colorN` (0–255), or
`#RGB`/`#RRGGBB` values:

```toml
accent_color = "bright-cyan"
secondary_color = "bright-blue"
highlight_color = "bright-magenta"
success_color = "bright-green"
error_color = "bright-red"
prompt_color = "bright-cyan"
```

`Theme::gradient_rgb` interpolates around the accent → secondary → highlight
loop. The prompt is colored with `prompt_color`; Reedline strips ANSI before
measuring width, so cursor math stays correct.

Key navigation is explicit and portable:

| Key | Action |
| --- | --- |
| Home / Ctrl+Home | Move to the start of the line |
| End / Ctrl+End | Move to the end of the line |
| Page Up | Previous history entry |
| Page Down | Next history entry |
| Ctrl+L | Clear screen and scrollback |
| Ctrl+C | Cancel the current line or command |

The shell does not assume a graphical font exists. Set `terminal_font` and
`apply_terminal_font = true` to emit an OSC 50 hint; terminals that do not
support OSC 50 safely ignore it, and the shell continues normally.

Behavior by environment:

- **Interactive terminal** — animated reveal with ANSI styling.
- **Piped or redirected output** — static, uncolored output; no cursor control.
- **`NO_COLOR` set** — no ANSI color sequences.
- **`TERM=dumb`** — no ANSI color sequences.
- **`--no-animation` or `startup_animation = false`** — static output with no
  reveal; color still follows the terminal and `NO_COLOR`.
- **`--quiet`, `startup_animation`, or `quiet_startup = true`** — no startup
  output at all.
- **Narrow terminal** — compact 3-row logo at 24–79 columns and the plain
  `KERNEXT` wordmark below 24 columns.
- **Ctrl+C during startup** — the cursor is restored and colors are reset
  before the program exits.
- **Startup failure** — a failed stage is reported as `[FAIL]` and terminal
  state is restored.

Logo layout:

- **80+ columns**: full six-row block wordmark.
- **24–79 columns**: compact three-row wordmark.
- **Below 24 columns**: plain `KERNEXT` wordmark.

## Screen clearing

`clear` and Ctrl+L both clear the visible screen and the scrollback buffer:

- `clear` is a native built-in. The evaluator returns `CommandAction::ClearScreen`
  only for a single command, never from a pipeline stage.
- The REPL calls `LineReader::clear_screen`, which writes `CSI 2 J` (clear
  screen), `CSI 3 J` (purge scrollback), and `CSI 1;1 H` (cursor home) through
  `crossterm`.
- Clearing is suppressed when stdout is not a terminal, so redirected output
  never contains escape sequences.
- Ctrl+L is already bound to `ReedlineEvent::ClearScreen` by Reedline's default
  Emacs keybindings; KERNEXT verifies that binding with a test instead of
  overriding it.
- Repeated `clear` commands are supported, and `clear` after command output
  still clears correctly.

## Architecture

```text
input
  -> shell-core lexer           (tokens, quotes, spans)
  -> shell-core parser          (AST)
  -> shell-builtins evaluator   (lists, groups, &&, ||)
  -> shell-builtins pipeline    (stages, OS pipes, redirections)
  -> built-in registry OR external process executor
  -> CommandOutcome             (action, output, success, exit code)
  -> shell-cli REPL             (startup, output, clearing, shutdown)
```

```text
terminal/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── AGENTS.md
├── RELEASE_NOTES.md
├── DESIGN.md
├── PRODUCT.md
├── .cargo/config.toml
├── .github/workflows/      # Release automation
├── installers/             # Linux, macOS, Windows installers
├── crates/
│   ├── shell-core/         # Identity, errors, lexer, parser, AST
│   ├── shell-runtime/      # Session, platform, working directory, lifecycle
│   ├── shell-builtins/     # Registry, evaluator, built-ins, process executor
│   ├── shell-config/       # Typed configuration and TOML persistence
│   └── shell-line-editor/  # Reedline integration, prompt, history, clear
└── apps/
    └── shell-cli/          # Binary entry point, startup, REPL orchestration
```

| Crate | Responsibility |
| --- | --- |
| `shell-core` | Branding constants, `ShellError`, lexer/tokenizer, parser, AST, spans |
| `shell-runtime` | `Platform`, `Session`, working directory, version, running/stopped state |
| `shell-builtins` | Built-in registry, AST evaluator, native commands, pipeline engine, external process execution |
| `shell-config` | Typed `Config`, validation, TOML load/save, default path |
| `shell-line-editor` | Reedline wrapper, prompt rendering, history, terminal clearing |
| `shell-cli` | Clap arguments, KERNEXT startup, REPL loop, error reporting |

Dependency direction:

```text
shell-cli ──> shell-builtins ──> shell-runtime ──> shell-config ──> shell-core
    │                                │
    │                                └──────────> shell-core
    ├──────────> shell-line-editor ──> reedline / crossterm
    └──────────> shell-core
```

The reusable core boundary is `shell-core` + `shell-runtime` +
`shell-builtins`. Future Web and Desktop surfaces are intended to call the same
lexer, parser, evaluator, and outcome types rather than reimplementing shell
semantics. No Web or Desktop code exists yet.

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
| `help` | Lists native commands under `KERNEXT — Available commands:`. |
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
| `clear` | Clears the terminal and scrollback through the line editor. |
| `history` | Shows the in-memory command history. |
| `whoami` | Shows the current user name. |
| `date` | Shows the local date and time. |
| `version` | Prints `KERNEXT v<version>`. |
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
the value `&&`, `||`, and `;` use and the value the shell exposes as `$?`. For a
pipeline, the final stage's status is authoritative.

## Shell language

### Lexer

The lexer converts input into tokens with one-based line and Unicode-scalar
column positions:

- `Word`, `String`, operators (`|`, `>`, `>>`, `2>`, `2>>`, `<`, `;`, `&&`,
  `||`, `&`, `(`, `)`), `Variable`, `SpecialVariable`, `Equals`, `Newline`,
  and `EndOfInput`.
- Single and double quotes, with quote style retained in the spanned-token API.
- Escapes: `\\`, `\"`, `\'`, `\$`, `\n`, `\t`, `\r`, `\ `, and escaped
  operator characters.
- `$name` and `${name}` variables are tokenized for expansion.
- `$?` and `$$` are tokenized as special variables.
- `=` is a distinct token to support assignment syntax.

### Parser and AST

```text
program        := list EOF
list           := (NEWLINE)* expression (separator expression)* separator?
separator      := ';' | NEWLINE | '&'

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
`Variable`, `Special`) so expansion happens at evaluation time without
re-parsing. The AST preserves source spans for every node.

The parser also recognises function definitions, `if`/`else`, `while`, `for`,
`return`, `break`, and `continue` nodes. The evaluator runs them with scoped
parameters, real loop iteration, and non-local control flow. Limits are
deliberate: function recursion stops at 128 nested calls and loops stop after
1,000,000 iterations. `for` accepts a word list, a variable containing
whitespace-separated words, or a numeric range such as `1..5` / `5..1`.
Redirections on function calls and sourcing scripts from files are not
implemented yet.

### Variable expansion

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

Four built-ins manage the environment:

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

### Background syntax

- `command &` is accepted by the lexer and parser.
- `&` has the same precedence as `;` and newline.
- A quoted `&` inside `"hello & world"` stays literal.
- The evaluator currently runs the background expression immediately and then
  returns success. It does not detach a process, track a PID, or notify on
  completion. Treat this as syntax that parses, not as working job control.
- Background evaluation is isolated: `exit`, `clear`, `break`, and `continue`
  inside a `&` expression cannot terminate the foreground shell or the
  enclosing loop.

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
- `if`/`else`, `while`, `for`, function definitions/calls, recursion limits,
  and `return`/`break`/`continue`.
- Persistent history with file-backed storage and credential filtering.
- Tab completion for built-ins, executables, paths, and variables.
- Animated, continuously color-changing startup with a configurable theme.
- Portable Home/End/Page Up/Page Down navigation.
- KERNEXT startup presentation, `--quiet`, `--no-animation`, and config-driven
  startup modes.
- Genuine `clear` and Ctrl+L clearing on a real terminal.
- TOML configuration loading from the platform config directory.
- Linux (GNU) is verified locally. macOS and Windows code paths are maintained
  but require their respective hosts and toolchains to verify.

## Not implemented yet

- **Real background execution.** `&` parses and is accepted, but `run_background`
  evaluates the expression synchronously and returns; no detached job, PID
  tracking, reaping, or completion notification exists.
- **Script sourcing.** `if`, `else`, `while`, `for`, functions, recursion, and
  `return`/`break`/`continue` run natively, but there is no `source`/`.` command
  for loading a script file yet.
- **Redirections on function calls.** Function bodies can redirect their own
  commands, but `fn arg > file` reports a clear unsupported-operation message.
- Full job control (`fg`, `bg`, `jobs`, `wait`, `kill` with job IDs).
- Signal handling (SIGINT, SIGTERM, SIGHUP, SIGSTOP, SIGCONT).
- Process group/terminal ownership management.
- Globbing and pattern matching.
- Comments in scripts (`# comment`).
- Grouped expressions as pipeline stages (`(a; b) | c`). Groups still run
  standalone; the evaluator reports a clear error when a group is used as a
  pipeline stage.
- Built-in `cd` updates the session directory, not the process-wide directory.
- Flag/option completion.
- macOS and Windows release builds in CI. The installers are present; Linux is
  the only target produced by the current workflow.
- KERNEXT Web and KERNEXT Desktop. Both are future products and have no code in
  this repository.

## Configuration

KERNEXT loads configuration from the platform-specific path selected by
`Config::default_path()`:

- Linux: `$XDG_CONFIG_HOME/kernext/config.toml` or
  `~/.config/kernext/config.toml`.
- macOS: `~/Library/Application Support/kernext/config.toml`.
- Windows: `%APPDATA%\kernext\config.toml`.

If the file is missing, built-in defaults are used. If it exists but cannot be
parsed, KERNEXT prints a warning and falls back to defaults so a damaged file
cannot make the shell unusable.

```toml
prompt = "kernext ~ $ "
history_size = 1000
color = true
history_enabled = true
history_file = "history.txt"
history_filter_secrets = true
suggestions_enabled = true
completion_hidden_files = false
startup_animation = true
quiet_startup = false

# Theme: ANSI names, colorN (0-255), or #RGB/#RRGGBB.
accent_color = "bright-cyan"
secondary_color = "bright-blue"
highlight_color = "bright-magenta"
success_color = "bright-green"
error_color = "bright-red"
prompt_color = "bright-cyan"

# Optional terminal font hint (OSC 50). Many terminals ignore it.
terminal_font = ""
apply_terminal_font = false
```

Missing fields use defaults, unknown fields are rejected, and `history_size`
must be greater than zero. `history_file` is resolved relative to the config
directory unless it is absolute. Every color is validated at load time, and
`apply_terminal_font = true` requires a non-empty `terminal_font`.

Startup mode precedence:

1. `quiet_startup = true` or `--quiet` → no startup output.
2. Piped/redirected output, `startup_animation = false`, or
   `--no-animation` → static output.
3. Otherwise, on an interactive terminal → animated output.

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

External-process and pipeline tests use the test binary itself as a portable
helper, so they do not depend on `python`, `git`, `sort`, `grep`, or any other
particular program being installed.

## Implementation status

- **Phase 1:** Workspace, CLI startup, banner, Clap arguments, shared errors.
- **Phase 2:** REPL, prompt, Reedline editing/history, session lifecycle.
- **Phase 3 / 3.1:** Lexer, tokens, quoting, spans, Unicode-safe positions.
- **Phase 4:** Recursive-descent parser, AST, precedence, parse errors.
- **Phase 5:** Built-in registry, AST evaluator, native commands, session cwd.
- **Phase 6:** Direct external process execution, executable resolution,
  argument preservation, stdio strategy, exit statuses, error handling.
- **Phase 7:** Pipeline engine with real OS pipes, built-in I/O interop,
  input/output/error redirections, and `&&`/`||`/`;` chaining over pipelines.
- **Phase 8:** Native variable system with assignment (`name = value`), expansion
  (`$name`, `${name}`, `$?`, `$$`), environment management (`export`, `unset`,
  `env`, `set`), special variables, and child process environment inheritance.
- **Phase 9:** Runtime support for functions, recursion limits, `if`/`else`,
  `while`, `for`, and `return`/`break`/`continue`; script sourcing remains
  future work.
- **Phase 10:** Background syntax (`&`) parses and evaluates synchronously in an
  isolated evaluator state; it is not asynchronous job control.
- **Phase 11:** Job control, signal handling, and process groups are not
  implemented.
- **KERNEXT CLI phase:** Brand identity, startup presentation, `clear`/Ctrl+L,
  and configuration loading.

## Roadmap

### CLI

- Complete real background jobs and job control.
- Add script sourcing, function-call redirections, globbing, script comments,
  and richer diagnostics.
- Verify macOS and Windows release builds.

### KERNEXT Web — planned, not implemented

A browser-based KERNEXT surface sharing the same parser and evaluator, with a
graphical splash and a mountain-wallpaper visual identity. No web code exists
yet, and none is started in this phase.

### KERNEXT Desktop — planned, not implemented

A native desktop application built on the same core engine. The terminal CLI
remains text-only. No desktop code exists yet.

## License

No license has been selected for this project yet.
