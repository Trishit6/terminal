# KERNEXT

**The Next Core of Computing.**  
**Independent. Native. Engineered.**  
**OPEN • POWERFUL • INDEPENDENT** — *Build. Execute. Innovate.*

KERNEXT is an independent, cross-platform command-line shell written in Rust.
It runs inside an existing terminal and contains its own lexer, parser, AST,
command dispatcher, built-in commands, and external process executor. It is not
a wrapper around Bash, PowerShell, or CMD.

The same Rust engine now powers three surfaces that share one codebase:

- **KERNEXT CLI** — the reference terminal shell (`kernext`).
- **KERNEXT Web** — a React + Vite front end served by a local Axum server with
  engine-backed sessions and real pseudoterminal (ConPTY/openpty) streaming.
- **KERNEXT Desktop** — a native Tauri 2 application reusing the web front end
  while executing everything through a local Rust runtime, never a browser
  simulation.

> **Status:** The KERNEXT CLI is implemented. It parses input with its own lexer
> and parser, runs native built-ins directly, launches non-built-in commands as
> external processes through `std::process::Command`, connects pipeline stages
> with real operating-system pipes, applies input/output/error redirections, and
> evaluates `&&`, `||`, and `;`. Native variable assignment, expansion,
> environment management, loops, conditionals, functions, `return`, `break`, and
> `continue` are implemented, and configuration is loaded at startup.
> Background `&` starts real detached work tracked by a job table, and `jobs`,
> `wait`, `kill`, `fg`, and `bg` operate on shell job numbers.
>
> The Web server and Desktop application are implemented on Windows and verified
> through their automated suites; interactive GUI verification on Linux and
> macOS hosts is still pending. Signal handling and process-group ownership in
> the CLI remain incomplete. See
> [Not implemented yet](#not-implemented-yet) and
> [`docs/PROJECT_STATUS.md`](docs/PROJECT_STATUS.md).

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

The CLI is intentionally text-only. The graphical splash, mountain wallpaper,
and richer visual surfaces belong to the Web and Desktop products, which now
exist under `apps/web` and `apps/desktop`.

## Overview

The shell provides:

- an interactive Read-Eval-Print Loop (REPL) with the `kernext ~ $ ` prompt,
- a KERNEXT startup wordmark with a continuous color-changing gradient,
- a themeable palette for the wordmark, checklist, prompt, and status text,
- Reedline-backed line editing, history, arrow keys, and Ctrl+R search,
- Home/End and Page Up/Page Down navigation on Linux, macOS, and Windows,
- a custom lexer/tokenizer with quote metadata and source spans,
- a custom recursive-descent parser and AST,
- 56 native built-in commands,
- direct external process execution using the system `PATH`,
- pipelines connected by real operating-system pipes,
- input, output, append, and error redirections (`<`, `>`, `>>`, `2>`, `2>>`),
- correct argument boundaries for quoted and unquoted words,
- current-working-directory and environment propagation,
- inherited terminal I/O for interactive child programs,
- clear errors for missing programs, permission failures, and spawn failures,
- command lists (`;`), groups, and `&&`/`||` evaluation,
- Ctrl+C line cancellation at the prompt and Ctrl+D/EOF shutdown,
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
  hardcoding extensions, and supports native `.exe`/`.com` images plus
  `.cmd`/`.bat` command scripts.
- Unix executability is checked through the file mode's execute bits.
- Home-directory resolution uses `HOME` or `USERPROFILE`/`HOMEDRIVE`.
- No Unix-only APIs are used outside `#[cfg(unix)]` blocks.

## Requirements

- Rust 1.85 or newer (Rust 2024 edition).
- A working platform linker (a C toolchain on Linux/macOS, MSVC Build Tools or
  MinGW-w64 on Windows).
- Node.js 22+ and npm 10+ for the Web front end and Desktop shell
  (declared in `package.json` under `engines`; npm workspaces live in
  `apps/web` and `apps/desktop`).
- A real terminal for the interactive shell. Piping input into KERNEXT reports
  that standard input is not a terminal.

## Build and run

KERNEXT owns its developer workflow. The `kernext` binary exposes project
commands that wrap the Rust toolchain, so the toolchain stays an
implementation detail:

```sh
kernext build      # cargo build --workspace
kernext check      # cargo check --workspace --all-targets
kernext test       # cargo test --workspace
kernext fmt        # cargo fmt --all (--check to verify only)
kernext lint       # cargo clippy --workspace --all-targets -- -D warnings
kernext run        # cargo run -p shell-cli
kernext clean      # cargo clean
kernext doctor     # inspect the toolchain, front end, and project state
kernext version    # KERNEXT and toolchain versions
kernext help       # branded command reference
```

Every project command returns the underlying tool's exit code, prints the
invocation it ran, and accepts `--dry-run`. Cargo remains the Rust build tool:
technical commands (`cargo build --workspace`, `cargo test -p shell-core`, and
so on) still work exactly as before.

The binary is `kernext` (`kernext.exe` on Windows). `--help` and `--version`
are handled by Clap without starting a session; `kernext` with no command
starts the interactive shell.

### KERNEXT Web

```sh
npm ci
npm run web:dev          # vite dev server on http://127.0.0.1:5173
cargo run -p shell-server   # local API server (default http://127.0.0.1:7878)
```

The browser terminal talks to the server over
`/api/v1/sessions/{id}/ws` (engine sessions) and `/api/v1/pty`
(real pseudoterminal). The web front end builds with:

```sh
npm run web:typecheck
npm run web:build
```

### KERNEXT Desktop

```sh
npm run desktop:dev      # tauri dev: vite + debug desktop window
npm run desktop:build    # tauri build: production bundle
```

The desktop application embeds the same React front end and executes commands
through the local Tauri runtime only — it never calls the public web server and
never simulates the shell in JavaScript. See
[`docs/DESKTOP.md`](docs/DESKTOP.md) for configuration, architecture, and the
Windows MinGW manifest handling.

### Accounts and authentication

Accounts are optional and MariaDB-backed. When `DATABASE_URL` is set, the
server enables registration, Argon2id password hashing, HttpOnly session
cookies, CSRF double-submit protection, email verification, password recovery,
and per-account ownership of every web terminal session. Without
`DATABASE_URL`, the server keeps its loopback-only local mode and the CLI and
desktop shell work without any database.

Exact development sequence from the workspace root:

```sh
npm ci          # install web/desktop workspaces
npm run dev     # Axum on 127.0.0.1:7878 + Vite on 127.0.0.1:5173
```

Then open <http://127.0.0.1:5173/sign-in>. To run the halves separately, use
`cargo run -p shell-server` (Axum) and `npm run web:dev` (Vite). The backend
reads `./.env`, then the workspace-root `.env`, so the launch directory does
not decide whether accounts load. Stop any existing Vite process before
`npm run dev`; it stops the stack when either service fails to start.

Development configuration (workspace-root `.env`, placeholders in
`.env.example`):

```env
DATABASE_URL=mysql://kernext:CHANGE_ME@127.0.0.1:3307/kernext
KERNEXT_DEV_MAIL=1
KERNEXT_SEED_ADMIN=true
KERNEXT_ADMIN_EMAIL=admin@admin.com
KERNEXT_ADMIN_PASSWORD=password
```

The optional seed creates `admin@admin.com` / `password` once, with role
`admin`, and requires a password change on first sign-in. It never runs in
production and never overwrites an existing account. `GET /api/v1/health/database`
reports database connectivity, and a missing `DATABASE_URL` in production is a
startup error rather than a silent local-mode fallback.

Account routes live under `/api/auth` (register, login, logout, me, recovery,
verification, sessions, preferences); administrators can read
`/api/admin/users`. `GET /api/v1/health/database` reports connectivity. The CLI
never reads `DATABASE_URL`; local execution remains independent of database
availability.

The home page hero is a three-layer CSS 3D composition: independently rotating
outer and inner orbital planes, a stationary sphere and emblem, and labels
outside the 3D transform so they stay readable. Reduced Motion stops the
orbits; the terminal preview never rotates. See
[`docs/AUTHENTICATION.md`](docs/AUTHENTICATION.md),
[`docs/DATABASE.md`](docs/DATABASE.md), and
[`docs/BACKEND_ARCHITECTURE.md`](docs/BACKEND_ARCHITECTURE.md).

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

The `.github/workflows/release.yaml` workflow builds CLI archives for Linux
(GNU and musl), Windows, and macOS (Apple Silicon and Intel), plus desktop
bundles per platform, and attaches a SHA-256 checksum for every artifact. It
creates only a **draft** GitHub Release, is triggered manually through
`workflow_dispatch`, and never publishes or tags on its own. macOS and Windows
installers expect matching archives; until a release is actually published,
download from <https://github.com/Trishit6/terminal/releases> or
[build from source](#build-and-run).
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

The animation is a presentation layer, not a fake boot delay. The startup
lockup is the **Cinematic Space & Systems** identity adapted to a terminal:

```text
          ╭────────────────────────────────────╮
      ╭───╯                                    ╰───╮
  ╱          ◇           ██╗  ██╗           ◇          ╲
│                        ██║ ██╔╝                        │
│                        █████╔╝                         │
│                        ██╔═██╗                         │
  ╲          ◇           ██║  ██╗           ◇          ╱
      ╰───╮                                    ╭───╯
          ╰────────────────────────────────────╯
```

The metallic K monogram sits inside a luminous orbital ring, above the block
wordmark. It is composed only of box-drawing and block glyphs, so it stays
text-only, inherits the terminal font, and keeps a fixed 58-column width.

On a real terminal the emblem is revealed top to bottom, then the wordmark is
redrawn in place for a short, bounded sequence of frames while a 24-bit gradient
travels across every character. Colors come from elapsed time and the configured
palette, so the sweep is continuously color-changing rather than a fixed
two-tone logo. Initialization stages are still reported only after the
corresponding subsystem actually finishes. The whole reveal is well under a
second at the default pace and never delays readiness.

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
| Tab | Open completion for the current built-in, path, or variable |
| Ctrl+L | Clear screen and scrollback |
| Ctrl+C | Cancel the current input line |

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
- **Narrow terminal** — the compact core badge appears at 44–75 columns; below
  44 columns the emblem is omitted entirely.
- **Startup failure** — a failed stage is reported as `[FAIL]` and terminal
  state is restored.
- **Ctrl+C during startup** — the shell has no signal handler yet, so an
  interrupt during the sub-second reveal ends the process the way the platform
  handles Ctrl+C. Destructor-based cursor/colour restoration covers error paths
  but is not guaranteed on a signal exit.

Artwork layout:

- **76+ columns**: full nine-row orbital emblem + six-row block wordmark.
- **44–75 columns**: three-row core badge + three-row wordmark.
- **24–43 columns**: three-row wordmark only.
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
├── package.json               # npm workspaces + scripts (web:*, desktop:*)
├── package-lock.json          # npm lockfile for apps/web and apps/desktop
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
    ├── shell-cli/          # CLI binary entry point, startup, REPL
    ├── shell-server/       # Local Axum API server (HTTP + WebSocket)
    ├── web/                # React + Vite front end (web + desktop UI)
    └── desktop/            # Tauri 2 desktop application (src-tauri)
```

| Crate | Responsibility |
| --- | --- |
| `shell-core` | Branding constants, `ShellError`, lexer/tokenizer, parser, AST, spans |
| `shell-runtime` | `Platform`, `Session`, working directory, version, running/stopped state |
| `shell-builtins` | Built-in registry, AST evaluator, native commands, pipeline engine, external process execution |
| `shell-config` | Typed `Config`, validation, TOML load/save, default path |
| `shell-line-editor` | Reedline wrapper, prompt rendering, history, terminal clearing |
| `shell-cli` | Clap arguments, KERNEXT startup, REPL loop, error reporting |
| `shell-api` | Serializable session/terminal API contracts shared by server and desktop |
| `shell-server` | Local Axum server: sessions, execution, engine and PTY WebSockets |
| `shell-desktop` | Tauri 2 runtime: local engine, PTY, filesystem, settings commands |

Dependency direction:

```text
shell-cli ──> shell-builtins ──> shell-runtime ──> shell-config ──> shell-core
    │                                │
    │                                └──────────> shell-core
    ├──────────> shell-line-editor ──> reedline / crossterm
    └──────────> shell-core

shell-server ──> shell-api ──> shell-builtins ──> shell-runtime ──> shell-core
shell-desktop ─> shell-server (engine sessions) + shell-cli (embedded worker)
```

The reusable core boundary is `shell-core` + `shell-runtime` +
`shell-builtins`. The Web and Desktop surfaces call the same lexer, parser,
evaluator, and outcome types rather than reimplementing shell semantics.

### Web and Desktop execution flow

```text
browser / desktop window
  -> React front end (apps/web)
  -> WebSocket (web) OR Tauri invoke IPC (desktop)
  -> shell-server (web) OR shell-desktop commands (desktop)
  -> shell-builtins evaluator + pipeline        (engine sessions)
  -> OR a real PTY (ConPTY / openpty)           (pty sessions)
```

Desktop commands execute through the local Rust runtime only. The desktop
application never routes commands through the public web server and never
simulates the shell in JavaScript. PTY sessions on the desktop spawn this same
executable with `KERNEXT_DESKTOP_WORKER=1` to run the embedded CLI worker, so
an interactive terminal is available without shipping a sidecar binary.

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
| `tree` | Displays a directory tree. |
| `find` | Searches for files and directories with name/type filters. |
| `stat` | Shows file metadata. |
| `mkdir` | Creates directories, with `-p`/`--parents`. |
| `touch` | Creates files or updates modification times. |
| `cat` | Prints file contents, or standard input when no file is given. |
| `head` / `tail` | Shows the first or last lines of files or standard input. |
| `wc` | Counts lines, words, and bytes. |
| `grep` | Searches text for a pattern. |
| `sort` | Sorts lines of text. |
| `uniq` | Reports or omits repeated lines. |
| `echo` | Prints arguments separated by spaces. |
| `print` | Scripting form of `echo`. |
| `input` | Reads one line of piped/redirected input into a variable. |
| `len` / `type` | Reports a value's length or inferred type. |
| `str` / `int` / `float` / `bool` | Converts a value, failing with a clear message when it cannot. |
| `cp` | Copies files, with `-r`/`--recursive` for directories. |
| `mv` | Moves or renames files and directories. |
| `rm` | Removes entries, with `-r`/`-R` and `-f`. |
| `which` | Resolves a built-in or executable. |
| `basename` / `dirname` / `realpath` | Path helpers. |
| `clear` | Clears the terminal and scrollback through the line editor. |
| `history` | Shows the in-memory command history. |
| `jobs` | Lists tracked background jobs. |
| `wait` | Waits for one job or all jobs. |
| `kill` | Terminates a background job by job number. |
| `fg` | Waits for a background job and prints its command. |
| `bg` | Reports a background job's state (see job control limits). |
| `run` | Runs a KERNEXT script file natively. |
| `test` / `[` | Evaluates a conditional expression. |
| `expr` | Evaluates arithmetic and comparison expressions. |
| `sleep` | Pauses for a number of seconds. |
| `true` / `false` | Returns a fixed exit status. |
| `whoami` | Shows the current user name. |
| `date` | Shows the local date and time. |
| `host` | Resolves a hostname to its addresses. |
| `sysinfo` | Shows system and runtime information. |
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
- On Windows, `PATHEXT` is read dynamically and only `.com`, `.exe`, `.bat`,
  and `.cmd` are launched, in `PATHEXT` order. A name that already has an
  extension is not extended again. Extensionless files are never treated as
  executables, because Windows cannot start them.
- On Unix, a candidate must be a regular file with at least one execute bit.

Windows launchers:

- `.exe` and `.com` are native images and start directly.
- `.cmd` and `.bat` are command scripts and start through the system command
  interpreter (`%COMSPEC%`, else `%SystemRoot%\System32\cmd.exe`) using
  `/d /v:off /s /c`. `/d` ignores AutoRun commands, `/v:off` disables delayed
  expansion, and `/s` makes the surrounding quotes behave predictably.
- Every script argument is quoted, so `&`, `|`, `<`, `>`, `(`, `)`, `^`, and
  `!` arrive as literal text and cannot become extra commands.
- An argument containing `"`, `%`, CR, LF, or NUL is refused with a clear error
  rather than silently rewritten; `cmd.exe` has no escaping for those.
- This is why npm-installed tools work: npm writes an extensionless POSIX `sh`
  shim plus a `.cmd` shim, and KERNEXT selects the `.cmd`.

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

`CommandOutcome` exposes `success()` and `exit_code()`. Its `output()` is the
combined display text (the internal command result keeps stdout and stderr
separate so redirections can target them individually). The exit code is `Some`
for an external process that exits normally and for a single `exit` command; it
is `None` for other built-ins and for processes terminated by a signal.
`success()` is the value `&&`, `||`, and `;` use and the value the shell exposes
as `$?`. A signal-terminated child sets `$?` to `128 + signal`. For a pipeline,
the final stage's status is authoritative. `exit [status]` also becomes the
shell process exit status.

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

### Background jobs

- `command &` starts real background work. `&` has the same precedence as `;`
  and newline, and a quoted `&` inside `"hello & world"` stays literal.
- A backgrounded external command is spawned directly so its `Child` handle is
  owned by the job table; it can therefore be waited for and killed.
- Any other background expression runs on a detached thread with a cloned
  session, so its variables and working directory stay isolated from the
  foreground shell. `exit`, `clear`, `break`, and `continue` inside `&` cannot
  terminate the shell or the enclosing loop.
- `jobs` lists job number, state (`running`, `done`, `failed`, `killed`), and
  exit code. `wait [id]` blocks until the job (or every job) finishes. `kill id`
  terminates a running job. `fg id` waits for a job and prints its command.
- Background jobs are announced only when `jobs`/`wait`/`fg` inspect them;
  there is no asynchronous completion notification yet.
- A detached shell-expression job cannot be interrupted safely from another
  thread, so `kill` marks it killed and the job may still finish independently.
- These are shell job numbers, never operating-system PIDs, so an invalid job
  number cannot terminate an unrelated process.

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
- Native scripting helpers: `run script.rsh`, `print`, `input`, `len`, `type`,
  `str`, `int`, `float`, `bool`, `test`, and `expr`.
- Host and system introspection: `host` (DNS resolution) and `sysinfo`
  (platform, architecture, CPU count, process ID, directories, user).
- Background jobs with a job table and `jobs`, `wait`, `kill`, `fg`, `bg`.
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

- **Asynchronous job notifications.** Background jobs run for real and are
  tracked, but the shell does not print completion notices while you type; run
  `jobs`/`wait` to inspect them.
- **Script sourcing.** `if`, `else`, `while`, `for`, functions, recursion, and
  `return`/`break`/`continue` run natively, but there is no `source`/`.` command
  for loading a script file yet.
- **Redirections on function calls.** Function bodies can redirect their own
  commands, but `fn arg > file` reports a clear unsupported-operation message.
- Full job control. `jobs`, `wait`, `kill`, `fg`, and `bg` exist and operate on
  job numbers, but there is no process-group hand-off: `fg` waits for a job
  instead of giving it the terminal, and `bg` only reports state.
- Shell-wide signal handling (SIGINT, SIGTERM, SIGHUP, SIGSTOP, SIGCONT) and
  process-group/terminal ownership management. Ctrl+C at the prompt cancels the
  current Reedline line; a child terminated by a signal sets `$?` to
  `128 + signal`, but the shell does not install its own signal handlers.
- Globbing and pattern matching.
- Comments in scripts (`# comment`).
- Grouped expressions as pipeline stages (`(a; b) | c`). Groups still run
  standalone; the evaluator reports a clear error when a group is used as a
  pipeline stage.
- Built-in `cd` updates the session directory, not the process-wide directory.
- Flag/option completion.
- Linux and macOS hosts for the Web and Desktop applications. Both are
  implemented and tested on Windows; their GUIs still need interactive
  verification on the other platforms, and macOS bundling requires a macOS
  host.

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
# Startup animation pace: "slow", "normal", or "fast".
animation_speed = "normal"

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
directory unless it is absolute. Every color is validated at load time,
`animation_speed` must be `slow`, `normal`, or `fast`, and
`apply_terminal_font = true` requires a non-empty `terminal_font`.

Startup mode precedence:

1. `quiet_startup = true` or `--quiet` → no startup output.
2. Piped/redirected output, `startup_animation = false`, or
   `--no-animation` → static output.
3. Otherwise, on an interactive terminal → animated output.

## Development

```sh
kernext fmt --check
kernext check
kernext build
kernext test
kernext lint
kernext doctor
npm ci
npm run typecheck
npm test
npm run build
```

The same checks through Cargo, which the KERNEXT commands call:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The `npm` scripts are defined at the workspace root: `web:dev`, `web:build`,
`web:preview`, `web:typecheck`, `web:test` (Vitest logic tests for the terminal
engine), `desktop:dev`, `desktop:build`, plus the short aliases `typecheck`,
`build`, and `test`. `npm ci` installs both workspaces from
`package-lock.json`.

Focused test commands:

```sh
cargo test -p shell-builtins --test external
cargo test -p shell-builtins --test external_tools
cargo test -p shell-builtins --test builtins
cargo test -p shell-builtins --test pipeline
cargo test -p shell-core --test parser
cargo test -p shell-cli --test repl
cargo test -p shell-server
cargo test -p shell-desktop
```

External-process and pipeline tests use the test binary itself as a portable
helper, so they do not depend on `python`, `git`, `sort`, `grep`, or any other
particular program being installed. `external_tools.rs` additionally verifies
the common developer tools (`git`, `cargo`, `rustc`, `node`, `npm`, `python`,
`curl`, `code`, `opencode`) through the full engine and skips any that are not
installed on the host.

CI (`.github/workflows/ci.yml`) runs fmt, clippy, and tests on Windows x64,
Linux x64, and macOS (Apple Silicon and Intel), a web job (typecheck, Vitest,
production build), and a desktop bundling job across the same operating systems.

### Further documentation

- [`docs/COMMANDS.md`](docs/COMMANDS.md) — every built-in command with usage.
- [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — workspace and data flow.
- [`docs/BACKEND_ARCHITECTURE.md`](docs/BACKEND_ARCHITECTURE.md) — server
  modules, routes, and request security.
- [`docs/DATABASE.md`](docs/DATABASE.md) — MariaDB schema, migrations, and the
  test database.
- [`docs/AUTHENTICATION.md`](docs/AUTHENTICATION.md) — account flows, cookies,
  CSRF, recovery, and terminal authorization.
- [`docs/DESKTOP.md`](docs/DESKTOP.md) — desktop build, packaging, and MinGW
  manifest handling.
- [`docs/SECURITY.md`](docs/SECURITY.md) — web terminal security model.
- [`docs/DEPLOYMENT.md`](docs/DEPLOYMENT.md) — deploying the web server.
- [`docs/TROUBLESHOOTING.md`](docs/TROUBLESHOOTING.md) — common problems.
- [`docs/BRANDING.md`](docs/BRANDING.md) — logo, palette, and voice.
- [`CONTRIBUTING.md`](CONTRIBUTING.md) — how to contribute.
- [`docs/PROJECT_STATUS.md`](docs/PROJECT_STATUS.md) — per-surface status.

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
- **Phase 10:** Background execution (`&`) with a job table, real detached
  external processes, threaded isolation for shell expressions, and
  `jobs`/`wait`/`kill`/`fg`/`bg` over shell job numbers.
- **Phase 11:** Job control, signal handling, and process groups are not
  implemented.
- **KERNEXT CLI phase:** Brand identity, startup presentation, `clear`/Ctrl+L,
  and configuration loading.
- **Web phase 1–4:** Local Axum server (`shell-server`) with session and
  execution endpoints, engine WebSocket, real ConPTY/openpty WebSocket relay,
  and the React + Tailwind + shadcn/ui web front end with a cinematic
  startup animation, marketing site, docs page, and interactive terminal.
- **Desktop phase 5:** Tauri 2 application (`shell-desktop`) reusing the web
  front end, with local engine sessions, real PTY sessions running the
  embedded CLI worker, multi-tab terminal, dashboard, command explorer, file
  explorer, settings, and about screens, plus a custom title bar and sidebar.
- **Product integration (Phase 6):** complete public website (home, features,
  documentation, downloads, about, contact, 404) with a build-time version
  badge; `host`/`sysinfo` built-ins and a registered `[` alias; a
  cross-interface consistency suite; external-tool compatibility tests;
  full documentation set (commands, security, deployment, troubleshooting,
  contributing); cross-platform CI matrices and a draft-only release workflow
  with per-artifact checksums; fixed the broken root `desktop:*` scripts.
- **Developer experience (Phase 7):** `kernext build`/`check`/`test`/`fmt`/
  `lint`/`run`/`clean`/`doctor`/`version`/`help` project commands, a branded
  doctor report, npm workspaces with a committed `package-lock.json`, and
  npm-based CI and release workflows.

## Roadmap

### CLI

- Complete real background jobs and job control.
- Add script sourcing, function-call redirections, globbing, script comments,
  and richer diagnostics.
- Run the configured CI/release workflows on GitHub to verify macOS, Windows,
  and Intel macOS builds for real (they have not been executed yet).

### KERNEXT Web — implemented

The browser surface runs on `apps/shell-server` + `apps/web`: engine-backed
sessions over a WebSocket, a real pseudoterminal relay (ConPTY on Windows,
`openpty` on Unix), the cinematic splash and mountain-wallpaper visual
identity, and the full public website — marketing home page, features,
documentation, downloads, about, and contact pages. The server is a local
companion by design: loopback bind, optional bearer token, origin allowlist,
session caps, and idle cleanup, all documented in
[`docs/SECURITY.md`](docs/SECURITY.md) with a deployment guide in
[`docs/DEPLOYMENT.md`](docs/DEPLOYMENT.md).

### KERNEXT Desktop — implemented

The native desktop application runs on Tauri 2 (`apps/desktop`) and reuses the
web front end. It offers a multi-tab interactive terminal (engine or PTY
modes), dashboard, command explorer, file explorer, sessions, settings, and
about screens. Remaining work: interactive verification on Linux and macOS,
macOS bundling on a macOS host, and desktop release automation.

## License

No license has been selected for this project yet.
