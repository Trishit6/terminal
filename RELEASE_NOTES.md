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
- `startup_animation` and `quiet_startup` configuration options.
- ANSI styling is disabled automatically for piped output, `NO_COLOR`, and
  `TERM=dumb`; narrow terminals receive a compact logo.
- Portable Home/End, Ctrl+Home/Ctrl+End, and Page Up/Page Down navigation.
- Optional OSC 50 terminal-font hints for terminals that honor them.
- `clear` and Ctrl+L genuinely clear the screen and scrollback on a terminal.
- Installers animate a color-cycling banner and show spinners while real
  download, verification, extraction, and installation work runs.

### Core features
- **Custom lexer/parser**: independent Rust implementation with Unicode support,
  quote metadata, and source spans.
- **AST evaluation**: recursive-descent parser with structured expressions.
- **21 native built-ins**: `help`, `pwd`, `cd`, `ls`, `mkdir`, `touch`, `cat`,
  `echo`, `cp`, `mv`, `rm`, `clear`, `history`, `whoami`, `date`, `version`,
  `exit`, `export`, `unset`, `env`, `set`.
- **External process execution**: direct `std::process::Command` execution
  without shell wrappers.
- **Pipelines**: real OS pipes connecting stages, including built-in and
  external stages in the same pipeline.
- **Redirections**: `<`, `>`, `>>`, `2>`, `2>>` with session-relative path
  resolution.
- **Logical chaining**: `&&`, `||`, and `;` with correct precedence.
- **Variables**: assignment (`name = value`), expansion (`$name`, `${name}`),
  and special variables (`$?`, `$$`, `$PWD`, `$HOME`).
- **Environment management**: `export`, `unset`, `env`, `set`.
- **Scripting runtime**: `if`/`else`, `while`, `for`, function definitions and
  calls, `return`, `break`, and `continue`, with scoped parameters and explicit
  recursion/iteration limits.
- **Line editing**: Reedline-powered REPL with persistent history and Ctrl+R
  search.
- **Privacy-aware history**: credential-looking commands are kept out of
  in-memory and persistent history.
- **Tab completion**: built-ins, executables, paths, and variables.
- **Cross-platform code paths**: Linux, macOS, and Windows support in the
  source; verified locally on Linux.

### Platform support
- **Linux (GNU)**: verified locally.
- **Linux (musl)**: build target configured in CI; not verified locally.
- **macOS**: source-compatible; requires a macOS host and SDK to build.
- **Windows**: source-compatible; requires a Windows or mingw-w64 toolchain to
  build.

### Security
- No command injection through a shell wrapper.
- No `eval` or dynamic code execution.
- Arguments are passed as separate `Command::arg` values.
- No `sh -c`, `bash -c`, or `cmd /C` is used.
- The parent process environment is never modified.
- Child processes receive inherited plus exported variables, minus hidden names.
- Path handling resolves relative names against the session working directory.

### Known limitations
- **Background execution is synchronous.** `&` evaluates the expression
  immediately in an isolated evaluator state and then returns; it does not
  create a detached job, track a PID, or notify on completion.
- **No script sourcing.** Functions, loops, and conditionals run natively, but
  there is no `source`/`.` command for loading a script file.
- **No redirections on function calls.** Function bodies may redirect their own
  commands, but `fn arg > file` reports an unsupported-operation message.
- No job control (`jobs`, `fg`, `bg`, `wait`, `kill`).
- No globbing or pattern matching.
- No comments in scripts.
- No signal handling beyond Ctrl+C and Ctrl+D.
- No flag/option completion.
- Release automation currently builds Linux targets only. The macOS and Windows
  installers are provided, but their archives must be produced by a matching
  host toolchain until those CI jobs are added.

---

## Future roadmap

### CLI hardening
- Complete background jobs with PID tracking, reaping, and notifications.
- Add script sourcing and redirections on function calls.
- Job control and signal handling.
- Globbing and pattern matching.
- Script comments and richer diagnostics.

### KERNEXT Web — planned, not implemented
- Browser-based terminal experience sharing the same parser and evaluator.
- A graphical splash and mountain-wallpaper identity for the web surface.
- No web code exists in this repository yet.

### KERNEXT Desktop — planned, not implemented
- Native desktop shell surface built on the same execution engine.
- The current CLI intentionally remains text-only.
- No desktop code exists in this repository yet.
