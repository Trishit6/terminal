# custom-shell

A cross-platform command-line shell written in Rust. It runs inside an existing
terminal (Windows Terminal, PowerShell, CMD, Linux terminals, and macOS
Terminal) and does not include any graphical interface.

> **Status:** Phase 3 is complete. The workspace now includes an independent,
> tested lexer/tokenizer for the custom shell language. The Phase 2 REPL and its
> four built-in commands remain unchanged.

## Overview

The shell provides:

- an interactive Read-Eval-Print Loop (REPL),
- a custom prompt with command history and arrow-key navigation,
- a custom lexer/tokenizer for words, strings, variables, operators, and
  newlines,
- operating-system and working-directory detection,
- built-in `help`, `version`, `clear`, and `exit` commands,
- friendly errors for unknown commands while keeping the shell open,
- graceful Ctrl+C cancellation and Ctrl+D/EOF shutdown,
- `--help` and `--version` startup arguments,
- clean exit codes and startup error reporting.

The workspace is split into small crates so parsing, execution, built-ins,
configuration, and line editing can grow independently without circular
dependencies.

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) 1.85 or newer (Rust 2024
  edition). Development and verification used Rust 1.99.1.
- Cargo, included with Rust.
- A platform linker:
  - **Linux / macOS:** the standard system C toolchain.
  - **Windows:** either Visual Studio Build Tools with the C++ workload
    (MSVC toolchain), or the GNU toolchain with MinGW-w64 installed and its
    `bin` directory on `PATH`:

    ```sh
    rustup toolchain install stable-x86_64-pc-windows-gnu
    rustup default stable-x86_64-pc-windows-gnu
    ```

    A maintained MinGW-w64 distribution such as WinLibs provides the required
    `gcc`, `dlltool`, and linker tools.

Install Rust with rustup:

```sh
rustup toolchain install stable
rustup default stable
```

## Architecture

```text
custom-shell/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── .gitignore
├── crates/
│   ├── shell-core/         # Identity, errors, and the custom lexer/tokenizer
│   ├── shell-runtime/      # Session state, platform detection, future execution
│   ├── shell-builtins/     # Built-in registry and command dispatcher
│   ├── shell-config/       # Configuration model and TOML persistence
│   └── shell-line-editor/  # Reedline integration, prompt, and history
└── apps/
    └── shell-cli/          # Binary entry point, startup, and REPL orchestration
```

### Crate responsibilities

| Crate | Current responsibility | Planned responsibility |
| --- | --- | --- |
| `shell-core` | Application name, version, shared `ShellError`, and the Phase 3 lexer/tokenizer | Parser, AST, values |
| `shell-runtime` | `Platform`, `Session`, working directory, version, running/stopped state | Execution engine and richer command lifecycle |
| `shell-builtins` | Command registry, Phase 2 dispatcher, command outcomes | Native command implementations and full dispatch |
| `shell-config` | Typed `Config`, validation, TOML load/save, default path | Config loading at startup |
| `shell-line-editor` | Reedline wrapper, prompt rendering, history, terminal clearing | Autocomplete and richer editing |
| `shell-cli` | Clap arguments, banner, REPL loop, error reporting, integration tests | Additional application modes |

Dependency direction:

```text
shell-cli ──> shell-builtins ──> shell-runtime ──> shell-config ──> shell-core
    │                                │
    │                                └──────────> shell-core
    ├──────────> shell-line-editor ──> reedline / crossterm
    └──────────> shell-core
```

There are no cycles. Terminal handling stays in `shell-line-editor`; command
dispatch returns outcomes instead of writing directly to the terminal, which
keeps the REPL and commands testable.

## Dependencies

Dependencies are intentionally limited:

| Dependency | Why it is needed |
| --- | --- |
| `clap` (derive) | Parses `--help`, `--version`, and future startup flags with generated usage output. |
| `serde` (derive) | Defines the configuration schema once for deserialization and serialization. |
| `toml` | Provides the actual TOML format implementation for `serde`; used by the config persistence API. |
| `reedline` | Cross-platform interactive line editing, prompt rendering, key handling, and history navigation. |
| `crossterm` | Implements the `clear` command's screen clear and cursor reset. Reedline re-exports key types but not terminal commands, so a direct dependency is required. |

## Installation

From the repository root:

```sh
cargo build --release
```

The executable is written to:

- `target/release/custom-shell` on Linux and macOS,
- `target\release\custom-shell.exe` on Windows.

For local development, run it through Cargo from an interactive terminal:

```sh
cargo run -p shell-cli
```

## Usage

```text
$ custom-shell
========================================
              CUSTOM SHELL
          Independent Rust CLI
========================================

Custom Shell v0.1.0
Platform: Windows
Working directory: D:\projects\React-Ts\terminal
Type 'help' for available commands.
Type 'exit' to close the shell.

custom-shell > help
Available commands:
  help     Show this help message
  version  Show the shell version
  clear    Clear the terminal screen
  exit     Exit the shell
custom-shell > version
Custom Shell v0.1.0
custom-shell > unknowncommand
Command not found: unknowncommand
custom-shell > clear
custom-shell > exit
Goodbye.
```

`--help` and `--version` are handled by Clap and exit successfully without
starting a session:

```sh
custom-shell --help
custom-shell --version
```

Starting the interactive shell with redirected/non-terminal standard input
returns a clear error instead of attempting to read terminal events.

### Built-in commands

| Command | Behavior |
| --- | --- |
| `help` | Lists the available commands. |
| `version` | Prints `Custom Shell v<version>`. |
| `clear` | Clears the visible terminal and redraws the prompt. |
| `exit` | Prints a farewell and shuts the shell down cleanly. |

Command names are case-sensitive. Unknown names print
`Command not found: <name>` and leave the shell running.

### Keyboard behavior

- Up/Down arrows navigate command history.
- Left/Right, Home/End, Backspace, and Delete use Reedline's standard Emacs
  bindings.
- Ctrl+C cancels the current input line and shows a new prompt.
- Ctrl+D on an empty line requests EOF and shuts down cleanly.
- Ctrl+R opens Reedline's reverse history search.

History is currently held in memory for the lifetime of the session.

## Shell language lexer (Phase 3)

The lexer lives in `shell-core` and is deliberately independent of REPL command
execution. It converts raw input into tokens and reports lexical errors with
one-based line and Unicode-scalar column positions.

```rust
use shell_core::{lex, Token};

let tokens = lex(r#"echo "Hello World""#).expect("valid input");

assert_eq!(
    tokens,
    vec![
        Token::Word("echo".to_owned()),
        Token::String("Hello World".to_owned()),
        Token::EndOfInput,
    ]
);
```

### Token types

| Token | Meaning |
| --- | --- |
| `Word(String)` | Unquoted word with escapes decoded. |
| `String(String)` | Quoted string with quotes removed and escapes decoded. |
| `Pipe` | Single pipe operator. |
| `OutputRedirect` | `>` |
| `AppendRedirect` | `>>` |
| `InputRedirect` | `<` |
| `Semicolon` | `;` |
| `And` | `&&` |
| `Or` | Double pipe operator. |
| `LeftParenthesis` | `(` |
| `RightParenthesis` | `)` |
| `Variable(String)` | Variable name without the leading `$`. |
| `Newline` | `\n`, `\r\n`, or a lone `\r`. |
| `EndOfInput` | Always the final token, including for empty input. |

### Syntax rules

- Words end at whitespace or at `| & > < ; ( ) ' " $`.
- `"double"` and `'single'` strings produce `String` tokens. Strings may span
  physical lines. Quote style is not retained in the Phase 3 token model.
- Backslash escapes work inside and outside quotes. Supported escapes are
  `\\`, `\"`, `\'`, `\$`, `\n`, `\t`, `\r`, `\ `, and the escaped operator
  characters `\| \& \> \< \; \( \)`. Other escapes and trailing backslashes
  are errors.
- `$name` produces `Variable("name")`. Names start with `_` or a Unicode
  alphabetic character and continue with `_` or Unicode alphanumeric
  characters. `$` inside strings is literal; `${name}` is not supported yet.
- Operators are recognized with or without surrounding whitespace. A single
  `&` is rejected.
- Spaces, tabs, and other Unicode whitespace are skipped. Physical newlines
  produce `Newline` tokens.
- `#`, `*`, and `?` are ordinary word characters in Phase 3; comments and glob
  expansion are not implemented.
- `=` is an ordinary word character; assignment semantics are not implemented.
- Adjacent word/string segments are separate tokens. For example, `foo"bar"`
  becomes `Word("foo")`, `String("bar")`.

The lexer is not yet wired into REPL command execution. The existing `help`,
`version`, `clear`, and `exit` commands continue to use the Phase 2 dispatcher.

## Configuration

`shell-config` defines the configuration schema and a tested persistence API:

```toml
prompt = "custom-shell > "
history_size = 1000
color = false
```

- Missing fields use defaults.
- Unknown fields are rejected.
- `history_size` must be greater than zero.
- `color` controls Reedline ANSI coloring; it defaults to `false` for a clean,
  low-color prompt.
- `Config::load`, `Config::load_or_default`, and `Config::save` perform real
  file I/O and create missing parent directories.
- `Config::default_path()` resolves the conventional per-user location using
  `APPDATA` on Windows, `HOME/Library/Application Support` on macOS, and
  `XDG_CONFIG_HOME` or `HOME/.config` on other platforms.

The CLI currently starts with in-memory defaults. Loading a configuration file
at startup is planned for a later phase.

## Development

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo check --workspace --all-targets
cargo build --workspace
cargo test --workspace
cargo run -p shell-cli -- --help
cargo run -p shell-cli -- --version
```

Tests cover:

- error formatting and error sources,
- platform/session detection, application version, and running/stopped state,
- configuration defaults, validation, TOML round trips, unknown fields, and
  on-disk persistence,
- registry registration and duplicate handling,
- command dispatch for empty input, help, version, clear, exit, unknown
  commands, and whitespace handling,
- prompt rendering and history capacity/navigation,
- lexer tokenization for words, quoting, escapes, operators, variables,
  whitespace, newlines, empty input, multiple commands, Unicode, and error
  locations,
- REPL help/version/unknown/exit flow, Ctrl+C continuation, EOF shutdown,
  empty input, clear behavior, and clean session shutdown.

## Current implementation status

Completed in Phase 1:

- Cargo workspace with five library crates and one binary crate.
- Minimal dependency set with a documented rationale.
- Startup banner with application name, version, platform, and current working
  directory.
- `--help` and `--version` support through Clap.
- Shared error type and clean exit codes (`0` success, `1` startup failure,
  `2` invalid command-line usage from Clap).
- Tested foundations for configuration, built-in metadata, and history.

Completed in Phase 2:

- Interactive REPL with a custom prompt.
- Reedline-backed line editing, history, arrow-key navigation, Backspace, and
  Delete.
- Session state with working directory, application version, and running/
  stopped lifecycle.
- Internal `help`, `version`, `clear`, and `exit` commands.
- Friendly unknown-command handling that keeps the shell open.
- Ctrl+C cancellation and Ctrl+D/EOF shutdown.
- Testable command outcomes and scripted REPL integration tests.
- Real pseudo-terminal verification on Windows.

Completed in Phase 3:

- Custom lexer/tokenizer in `shell-core`, with no parser or shell-engine
  delegation.
- Token model for words, strings, variables, operators, parentheses, newlines,
  and end of input.
- Single/double quoting, explicit escapes, Unicode-safe scanning, and
  whitespace/newline handling.
- Location-aware lexical errors for unterminated quotes, invalid/dangling
  escapes, invalid variables, and unexpected characters.
- Unit, integration, and documentation tests for the lexer.
- The Phase 2 REPL and its four commands remain unchanged.

Not implemented yet:

- Parser, AST, quoting semantics, expansion, or actual variable assignment.
- External command execution, pipelines, redirection, or job control.
- Additional built-ins such as `cd` or `pwd`.
- Persistent history across sessions.
- Autocomplete.
- Configuration loading during startup.
- Wiring the lexer into REPL command execution.

## Roadmap

- **Phase 4:** parser, AST, and shell-language validation.
- **Phase 5:** execution engine, external commands, pipelines, redirection, and
  exit-status propagation.
- **Phase 6:** additional built-ins, configuration integration, autocomplete,
  and persistent history.
- **Phase 7:** cross-platform hardening, broader integration tests, and release
  packaging.

## License

No license has been selected for this project yet.
