# KERNEXT Release Notes

**The Next Core of Computing.**  
OPEN • POWERFUL • INDEPENDENT — Build. Execute. Innovate.

## v0.1.0 (Current)

### Brand and experience
- Official product identity: **KERNEXT**.
- Tagline: **The Next Core of Computing.**
- Values: **OPEN • POWERFUL • INDEPENDENT**.
- Default prompt: `kernext ~ $ `.
- Executable renamed to `kernext` (the internal crate names remain stable).
- Cinematic startup presentation with a responsive wordmark, tagline, and a
  real initialization checklist.
- Continuous, time-derived 24-bit gradient animation across the wordmark while
  the logo is revealed in place.
- Themeable palette for the wordmark, checklist, prompt, and status text using
  ANSI names, `colorN`, or `#RGB`/`#RRGGBB` values.
- `--quiet` and `--no-animation` flags.
- `startup_animation`, `quiet_startup`, and `animation_speed` configuration
  options.
- ANSI styling is disabled automatically for piped output, `NO_COLOR`, and
  `TERM=dumb`; narrow terminals receive a compact logo.
- Portable Home/End, Ctrl+Home/Ctrl+End, and Page Up/Page Down navigation.
- Optional OSC 50 terminal-font hints for terminals that honor them.
- `clear` and Ctrl+L genuinely clear the screen and scrollback on a terminal.
- The installers animate a color-cycling banner and show spinners while real
  download, verification, extraction, and installation work runs.

### Core features
- **Custom lexer/parser**: independent Rust implementation with Unicode support,
  quote metadata, and source spans.
- **AST evaluation**: recursive-descent parser with structured expressions.
- **53 native built-ins**, covering filesystem/text operations, environment and
  scripting helpers, job control, and shell lifecycle commands. See `help` or
  the registry for the complete list.
- **External process execution**: direct `std::process::Command` execution
  without shell wrappers. Windows `.cmd`/`.bat` programs run through the system
  command interpreter with escaped arguments; `.exe`/`.com` launch natively.
- **Windows external command fix**: npm/pnpm tools such as `opencode` and
  `omnirush` no longer fail with `os error 193`. Resolution used to pick the
  extensionless POSIX `sh` shim beside the `.cmd` shim, which `CreateProcess`
  cannot execute. `PATHEXT` is now honoured for `.com`, `.exe`, `.bat`, and
  `.cmd`, and command scripts use a dedicated, injection-tested launcher.
- **Pipelines**: real OS pipes connecting stages, including built-in and
  external stages in the same pipeline.
- **Redirections**: `<`, `>`, `>>`, `2>`, `2>>` with session-relative path
  resolution.
- **Logical chaining**: `&&`, `||`, and `;` with correct precedence.
- **Variables**: assignment (`name = value` and `name=value`), expansion
  (`$name`, `${name}`), and special variables (`$?`, `$$`, `$PWD`, `$HOME`).
- **Environment management**: `export`, `unset`, `env`, `set`, with exported
  status preserved across reassignment.
- **Scripting runtime**: `if`/`else`, `while`, `for`, function definitions and
  calls, `return`, `break`, and `continue`, with scoped parameters and explicit
  recursion/iteration limits.
- **Script files**: `run script.rsh` with nesting and size limits.
- **Background jobs**: `&` starts real detached work tracked by a job table;
  `jobs`, `wait`, `kill`, `fg`, and `bg` operate on shell job numbers.
- **Line editing**: Reedline-powered REPL with persistent history and Ctrl+R
  search.
- **Privacy-aware history**: credential-looking commands, including spaced
  assignments such as `PASSWORD = value`, are kept out of history.
- **Tab completion**: built-ins, executables, paths, and variables, with
  mid-line replacement spans and live session context.
- **Cross-platform code paths**: Linux, macOS, and Windows support in the
  source.

### Platform support
- **Linux (GNU)**: canonical target; release automation builds it.
- **Linux (musl)**: build target configured in CI.
- **macOS**: source-compatible; requires a macOS host and SDK to build.
- **Windows**: source-compatible and exercised by the test suite; requires a
  Windows toolchain to build.

### Security
- No command injection through a shell wrapper.
- No `eval` or dynamic code execution.
- Arguments are passed as separate `Command::arg` values.
- No `sh -c`, `bash -c`, or `cmd /C` is used.
- Windows resolution skips `.bat` and `.cmd` so no implicit shell runs.
- The parent process environment is not modified.
- Child processes receive inherited plus exported variables, minus hidden names.
- Path handling resolves relative names against the session working directory.
- A signal-terminated child is reported as `128 + signal`.

### Known limitations
- **No asynchronous job notifications.** Background jobs run for real and are
  tracked, but the shell does not print completion notices while you type; run
  `jobs`/`wait` to inspect them.
- **No script sourcing.** Functions, loops, and conditionals run natively, but
  there is no `source`/`.` command for loading a script file into the current
  session.
- **No redirections on function calls.** Function bodies may redirect their own
  commands, but `fn arg > file` reports an unsupported-operation message.
- **No full job control.** `fg` waits for a job instead of handing over the
  terminal, `bg` only reports state, and detached shell expressions cannot be
  interrupted safely from another thread.
- **No shell-wide signal handling or process groups.** Ctrl+C at the prompt
  cancels the current line, and external children that die by signal are
  reported correctly, but KERNEXT does not install SIGINT/SIGTERM/SIGHUP
  handlers or own a foreground process group.
- No globbing or pattern matching.
- No comments in scripts.
- No flag/option completion.
- Grouped expressions are not supported as pipeline stages.
- Release automation currently builds Linux targets only. The macOS and Windows
  installers are provided, but their archives must be produced by a matching
  host toolchain until those CI jobs are added.

---

## Future roadmap

### CLI hardening
- Script sourcing and redirections on function calls.
- Asynchronous job notifications and full foreground process-group hand-off.
- Shell-wide signal handling and terminal ownership.
- Globbing, pattern matching, and script comments.
- Richer diagnostics and completion for command flags.

### KERNEXT Web — planned, not implemented
- Browser-based terminal experience sharing the same Rust parser, evaluator,
  and session engine.
- A graphical splash and mountain-wallpaper identity for the web surface.
- No web code exists in this repository yet.

### KERNEXT Desktop — planned, not implemented
- Native desktop shell surface built on the same execution engine.
- The current CLI intentionally remains text-only.
- No desktop code exists in this repository yet.
