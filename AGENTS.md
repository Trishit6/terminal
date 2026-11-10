# AGENTS.md

## Project

KERNEXT is a custom Rust CLI shell with three implemented surfaces that share
one engine: the CLI (`apps/shell-cli`), the local web server and React front
end (`apps/shell-server`, `apps/web`), and the Tauri 2 desktop application
(`apps/desktop`). The CLI phase is implemented, including branding, startup
presentation, genuine `clear`/Ctrl+L, and config loading. Do not start new
KERNEXT Web or Desktop feature work unless explicitly asked.

Official identity:

- Product: **KERNEXT**
- Tagline: **The Next Core of Computing.**
- Values: **OPEN • POWERFUL • INDEPENDENT**
- Support phrase: **Build. Execute. Innovate.**
- Prompt: `kernext ~ $ `
- Executable: `kernext`

Internal crate and package names stay `shell-*`. Do not rename crates,
directories, or Rust identifiers just to match the brand. User-facing output,
the executable name, configuration paths, installers, and documentation use
KERNEXT.

Pipelines and redirections are implemented natively; keep them that way.

Workspace members:

- `apps/shell-cli` — binary package `shell-cli`; executable name is `kernext`
- `apps/shell-server` — local Axum server (HTTP sessions, engine and PTY
  WebSockets)
- `apps/web` — React + Vite + Tailwind front end shared by web and desktop
- `apps/desktop` — Tauri 2 desktop application (crate `shell-desktop`,
  binary `kernext-desktop`); `apps/desktop/src-tauri` is the crate root
- `crates/shell-core` — branding constants, lexer, parser, AST, spans, shared errors
- `crates/shell-builtins` — native command registry, AST evaluator, built-ins,
  pipeline engine, external process executor, variable expansion, background syntax
- `crates/shell-runtime` — `Session` (cwd, config, lifecycle, platform, variables, functions)
- `crates/shell-line-editor` — Reedline integration behind `LineReader`, clear helpers
- `crates/shell-config` — TOML configuration model
- `crates/shell-api` — serializable session/terminal API contracts shared by
  the web server and the desktop runtime
- `installers/` — Linux, macOS, Windows installation scripts
- `docs/` — branding, architecture, desktop, and status documentation

Execution flow is:

```text
input -> shell-core lexer -> parser -> AST -> shell-builtins evaluator
      -> shell-builtins pipeline (stages, OS pipes, redirections)
      -> built-in registry OR process executor
      -> CommandOutcome -> shell-cli Repl
```

Do not bypass this by splitting input with `split_whitespace`. `Dispatcher`
must keep using `lex_spanned` + `parse`.

## Branding

- Constants live in `crates/shell-core/src/lib.rs`: `SHELL_NAME`, `PRODUCT_NAME`,
  `TAGLINE`, `VALUES`, `MOTTO`, `VERSION`.
- `SHELL_NAME = "kernext"` drives config paths, Clap help/version, and the binary.
- Do not hardcode versions. Use `VERSION` / `Session::version()`.
- The CLI is text-only. Do not add a graphical splash or mountain wallpaper to
  it; those belong to future Web/Desktop surfaces.
- Keep branding consistent across README, installers, release workflow, help,
  `version`, and startup.
- See `docs/BRANDING.md` for logo, palette, and voice rules.

## Startup experience

- Renderer: `apps/shell-cli/src/startup.rs`. It is presentation-only; it must
  never parse, evaluate, or launch commands.
- `cli::run` emits each checklist item only after the corresponding subsystem
  actually finishes initializing. Never fake progress.
- Modes: `Animated`, `Plain`, `Quiet`. Resolve them with `resolve_mode`.
- Animated only on an interactive terminal with animation enabled.
- Piped/redirected output is plain. `NO_COLOR` and `TERM=dumb` disable ANSI.
- Artwork is the **Cinematic Space & Systems** identity adapted to text: a
  metallic K monogram inside an orbital ring (`EMBLEM_FULL`, 9 rows, 58 columns)
  above the block wordmark. Use only box-drawing/block glyphs so it stays
  single-width in every terminal font.
- Fallbacks: full emblem + 6-row wordmark at 76+, 3-row core badge + 3-row
  wordmark at 44–75, 3-row wordmark at 24–43, plain `KERNEXT` below 24.
  `render_emblem` returns an empty string below 44 columns.
- **Alignment is a test invariant:** `emblem_rows_keep_a_constant_display_width`
  requires every emblem row to have the same character count. Never hand-edit a
  row without keeping that true.
- Animated mode reveals the emblem top to bottom, then redraws the wordmark in
  place while a 24-bit gradient derived from `Theme::gradient_rgb` and elapsed
  time sweeps across every character. Keep it continuous and palette-driven.
- `--quiet` and `--no-animation` are CLI flags.
- `startup_animation`, `quiet_startup`, and `animation_speed` (`slow`/`normal`/
  `fast`) are config fields. Speed maps to `frame_delay`/`emblem_frame_delay`;
  the whole reveal stays well under a second at `normal`.
- Startup interrupts are not signal-handled yet: `Drop`/`abort` restore the
  cursor and colors on error paths, but a Ctrl+C during the reveal ends the
  process as the platform dictates. Do not claim otherwise.
- `Startup` implements `Drop` to restore the cursor and reset colors on errors.
- Never add artificial boot delays. The animation step is a short presentation
  pause, not a progress simulation.
- `Repl::with_dispatcher` exists so the command engine stage is reported only
  after `Dispatcher::new()` succeeds.
- `terminal_font` / `apply_terminal_font` are opt-in OSC 50 hints. Many
  terminals ignore them; never require the font to be present.
- Keybindings are explicit: Home/End, Ctrl+Home/Ctrl+End, Page Up (previous
  history), Page Down (next history), and Tab (completion menu). Keep them
  portable and test the bindings.
- The installers animate a color-cycling banner and use real command spinners
  (or an equivalent progress display on Windows). Disable animation for
  non-terminals, `NO_COLOR`, and `TERM=dumb`.

## Screen clearing

- `clear` returns `CommandAction::ClearScreen` only from a single command, never
  from a pipeline stage.
- Shared helpers: `clear_terminal` and `clear_terminal_if` in
  `crates/shell-line-editor/src/editor.rs`.
- Clearing writes `CSI 2 J`, `CSI 3 J`, and `CSI 1;1 H` via `crossterm`; do not
  shell out to `clear`/`cls` and do not print blank lines as a substitute.
- Suppress clearing when stdout is not a terminal.
- Ctrl+L is bound by Reedline's default Emacs keybindings to
  `ReedlineEvent::ClearScreen`; verify it with a test rather than rebinding.
- Keep tests for repeated clear, clear after output, non-interactive clearing,
  and clear inside a pipeline.

## External execution (Phase 6)

- Executor: `crates/shell-builtins/src/process.rs`.
- Never use `sh -c`, `bash -c`, `cmd /C`, `powershell -Command`, or another
  shell to parse KERNEXT syntax. Launch the resolved executable with
  `std::process::Command` and pass each argument separately.
- The one exception is running a Windows **command script** (`.cmd`/`.bat`),
  which Windows cannot start natively. Those go through the system command
  interpreter (`%COMSPEC%`, else `%SystemRoot%\System32\cmd.exe`) with
  `/d /v:off /s /c` and a single pre-escaped command line. This is a launcher,
  not a parser: KERNEXT still owns lexing, pipelines, and built-ins.
- Built-ins always win. `eval.rs` checks the registry before external
  resolution; do not let a `PATH` program override a native command.
- Bare names resolve through `PATH`; names with separators resolve against the
  session working directory; absolute paths run directly.
- Windows resolution honours `PATHEXT` but only for `.com`, `.exe`, `.bat` and
  `.cmd`, in `PATHEXT` order. `.exe`/`.com` launch natively; `.cmd`/`.bat` use
  the script launcher. Other `PATHEXT` entries (`.vbs`, `.js`, `.ps1`) are not
  launched. Unix checks execute bits.
- **Extensionless files are never executables on Windows.** npm/pnpm install a
  POSIX `sh` shim with no extension beside the `.cmd` shim; selecting the
  extensionless one is what produced `os error 193`. Resolution must pick the
  `.cmd`.
- Windows script arguments are quoted, and an argument containing `"`, `%`,
  CR, LF, or NUL is refused with a clear error instead of being rewritten.
  `cmd.exe` has no escaping for those characters. Keep
  `tests/windows_scripts.rs` covering this.
- Default `StdioMode::Inherit` streams directly to the terminal. Use
  `StdioMode::Capture` for tests and non-interactive callers; it closes stdin.
- `ProcessOptions` carries stdio mode and environment overrides. It does not
  mutate the shell's global environment.
- `CommandOutcome::success()`/`exit_code()` carry the child status. `None`
  exit code means a built-in or a signal-terminated process.

## Pipelines and redirections (Phase 7)

- Engine: `crates/shell-builtins/src/pipeline.rs`.
- `eval.rs` sends both single commands and real pipelines through the engine.
  Do not reintroduce a separate ad-hoc command path.
- Lexer tokens: `Token::ErrorRedirect` (`2>`) and
  `Token::ErrorAppendRedirect` (`2>>`). Recognise them only when the digit starts
  a token, so `file2>log` stays a word plus stdout redirection.
- AST kinds: `RedirectionKind::{Input, Output, Append, Error, ErrorAppend}`.
- External stages use real OS pipes: `ChildStdout` is handed to the next stage as
  `Stdio::from`. Never collect an entire external stream into memory just to
  connect a pipeline.
- Built-ins run in-process. They exchange buffered bytes with neighbours, and
  buffered data is written to an external child on a detached writer thread.
  Callers must join writers before returning; children must be waited for.
- Built-ins inside a real pipeline use a cloned `Session` (`Session` derives
  `Clone`). Filesystem side effects persist; `cd` and similar state changes must
  not leak to the parent shell. A single command still mutates the real session.
- `exit` and `clear` actions propagate only from a single command, never from a
  pipeline stage.
- Redirections resolve paths with `commands::resolve_path` so relative paths use
  the session working directory. `>` truncates, `>>` appends, `2>` truncates
  stderr, `2>>` appends stderr.
- A redirection applies to its own stage. `echo x | cat > out.txt` writes only
  the final stage's output.
- Always close unused pipe ends and reap every child, including on startup
  failure. Kill and wait for already-started children before returning an error.
- Pipeline success and exit code come from the final stage. Do not report
  intermediate `SIGPIPE` termination as an error.
- A built-in stage after a producer whose output was redirected away receives
  empty input, not the terminal and not a missing-operand error.
- Grouped pipeline stages are unsupported: report
  `"Grouped pipelines are not supported yet."` Standalone groups still run.

## Canonical checkout and duplicate-copy hazard

The user-designated canonical project is:

```text
/run/media/trishit/UBUNTU 26_0/Projects/terminal
```

A second checkout may exist at `/home/trishit/Projects/custom-shell`. Do not
edit both. Do not create another copy or workspace. If both exist, confirm which
one the user wants active before changing files.

On the current Windows workstation, the active checkout is
`D:\projects\React-Ts\terminal`; the Linux path above is the canonical checkout
when that volume is mounted. No other KERNEXT Cargo workspace was found under
`D:\projects` or the Windows user profile during the Phase 0 audit.

## Critical build constraint (removable drive)

`terminal` is on a **vfat** mount with **`showexec`**, not a literal `noexec`.
`showexec` only executes files with `.exe/.com/.bat/.btm/.cmd` names, so Rust
build scripts and test binaries (extensionless) fail with:

```text
Permission denied (os error 13)
```

Default `cargo check/build/test/run` from `terminal` therefore fails while
compiling build scripts/proc macros. Do not misinterpret this as a code bug.
Do not change mount options without explicit permission.

**Permanent fix:** `.cargo/config.toml` sets
`build.target-dir = "/home/trishit/.cache/terminal-target"`.
All Cargo commands automatically use the ext4 target directory.
No `export CARGO_TARGET_DIR` is required. On Windows the same path resolves
under the current drive (for this checkout:
`D:\home\trishit\.cache\terminal-target`), still outside the source tree.

Sources stay only in `terminal`; all build artifacts land on ext4. Do not
hardcode a target directory in `Cargo.toml` or add OS-specific linker settings.
Do not create a repo-local `target/` directory. Do not change mount options
without explicit permission.

A source tree on vfat can still be inspected, edited, diffed, formatted, and
used by `cargo metadata --no-deps`.

## Desktop build notes (Windows GNU)

- `apps/desktop/src-tauri/build.rs` must keep shadowing the MinGW
  `default-manifest.o` on `*-pc-windows-gnu` targets. Some MinGW-w64
  distributions link a default manifest without the Common Controls v6
  dependency; GNU ld keeps it over Tauri's manifest and the binary dies at
  startup with `STATUS_ENTRYPOINT_NOT_FOUND` (`TaskDialogIndirect` missing
  from comctl32 v5). The build script compiles a replacement manifest object
  with `windres` and prepends its directory through
  `cargo:rustc-link-arg=-B<out-dir>\` so the GCC endfile spec picks it. Do not
  remove this or "simplify" it back to plain `tauri_build::build()` on GNU.
- On GNU targets the script uses
  `WindowsAttributes::new_without_app_manifest()`; MSVC and non-Windows
  targets keep Tauri's default manifest. See `docs/DESKTOP.md`.
- `tauri.conf.json` `frontendDist` is `../../web/dist` — Tauri resolves it
  relative to `src-tauri`, not the package root.
- Keep the Rust `tauri` crate and the npm `@tauri-apps/api` on the same
  major/minor version, or `tauri build` aborts with a version-mismatch error.
- Desktop commands execute through the local Rust runtime only — never the
  public web server, never a JavaScript simulation of the shell.

## Commands

Run from the workspace root of the buildable checkout.
The `.cargo/config.toml` automatically routes build artifacts to `/home/trishit/.cache/terminal-target`.

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p shell-cli
```

Front end and desktop (pnpm only; `package.json` pins `pnpm@11.25.0`):

```bash
pnpm install --frozen-lockfile
pnpm typecheck          # web typecheck (alias of web:typecheck)
pnpm test               # web Vitest suite (alias of web:test)
pnpm build              # web production build (alias of web:build)
pnpm web:dev            # vite dev server (http://127.0.0.1:5173)
pnpm desktop:dev        # tauri dev (web + desktop window)
pnpm desktop:build      # tauri build (production bundle)
cargo run -p shell-server   # local web API server
```

The root `desktop:*` scripts delegate to the `dev`/`build` scripts inside
`apps/desktop/package.json`. `pnpm --filter <pkg> <bin>` does not work — it
only runs package scripts — so do not change them back to
`pnpm --filter @kernext/desktop tauri …`.

Focused tests:

```bash
cargo test -p shell-builtins --test external
cargo test -p shell-builtins --test external_tools
cargo test -p shell-builtins --test builtins
cargo test -p shell-builtins --test builtins paths_containing_spaces
cargo test -p shell-builtins --test pipeline
cargo test -p shell-core --test parser
cargo test -p shell-cli --test repl
cargo test -p shell-server
cargo test -p shell-desktop
```

External-process and pipeline tests use the test binary itself as a portable
helper; they do not require `python`, `git`, `sort`, `grep`, or any other
particular program. `external_tools.rs` checks common developer tools
(`git`, `cargo`, `rustc`, `node`, `pnpm`, `python`, `curl`, `code`,
`opencode`) through the full engine and skips any that are not installed.

CI lives in `.github/workflows/ci.yml` (fmt, clippy, web job, test matrix
across Windows x64, Linux x64, macOS Apple Silicon + Intel, and a desktop
bundling matrix). Releases live in `.github/workflows/release.yaml`
(`workflow_dispatch` only, draft-only; CLI archives + desktop bundles +
SHA-256 checksums per artifact). Do not trigger the release workflow without
explicit authorization, and do not claim a workflow passed unless its run
completed.

The interactive shell requires a real TTY. Piping input directly into
`cargo run` is rejected because KERNEXT is interactive. Use a terminal, or
`screen`/`tmux`, for manual verification.

## Background jobs

- Lexer: `Token::Background` for `&`. Parser: `Separator::Background`.
  AST: `Expression::Background`. `parse_list` recognizes `&` as a separator.
- `run_background` + `spawn_background` in `eval.rs` implement it: a simple
  external command is spawned directly so the job table owns its `Child`;
  anything else runs on a detached thread with a cloned session.
- Job model: `crates/shell-runtime/src/jobs.rs` (`JobTable`, `JobInfo`,
  `JobStatus::{Running, Done, Failed, Killed}`). The table is shared through
  `Arc<Mutex<_>>` so pipeline clones see the same jobs.
- Built-ins: `jobs`, `wait`, `kill`, `fg`, `bg` in `commands.rs`.
- Background evaluation is isolated: `exit`, `clear`, `break`, and `continue`
  inside the expression cannot control the foreground shell or enclosing loop.
  Quoted `&` stays literal.
- **Limits to keep documented:** there is no asynchronous completion notice
  (the user must run `jobs`/`wait`), `fg` waits instead of handing over the
  terminal, `bg` only reports state, and a detached shell-expression job cannot
  be interrupted from another thread. Job IDs are shell job numbers, never
  OS PIDs.
- Do not reintroduce synchronous-only background evaluation, and do not claim
  full process-group job control.

## Scripting runtime

- Variable store: `crates/shell-runtime/src/variables.rs` (`VariableStore`).
- Session integration: `crates/shell-runtime/src/lib.rs` (methods on `Session`).
- Expansion: `crates/shell-builtins/src/expand.rs` (`expand_word`, etc.).
- Assignment evaluation: `crates/shell-builtins/src/eval.rs` (`run_assignment`).
- Built-ins: `crates/shell-builtins/src/commands.rs` (`export_builtin`,
  `unset_builtin`, `env_builtin`, `set_builtin`).
- Registry: `crates/shell-builtins/src/registry.rs` (56 commands).
- Lexer tokens: `Token::Equals`, `Token::SpecialVariable`,
  `SpecialVariable::{LastStatus, ProcessId}`.
- AST: `Expression::Assignment`, `WordPart::Special`, variable-name helpers in
  `shell-core`.
- `if`/`else`, `while`, `for`, functions, `return`, `break`, and `continue`
  are evaluated natively in `eval.rs`.
- `for` accepts word lists, variables containing whitespace-separated words, and
  ascending/descending numeric ranges (`1..5`, `5..1`).
- Function parameters are scoped to each call and restored afterwards.
  `MAX_FUNCTION_DEPTH = 128` and `MAX_LOOP_ITERATIONS = 1_000_000` bound
  runaway recursion and loops.
- Built-ins win over functions, and functions win over external programs.
- Known gaps: no `source`/`.` script loading, no redirections on function calls,
  and no closures or arrays. Do not describe those as implemented.

### Assignment syntax

- `name = value` and `name=value` are both parsed as assignments.
- Exactly one value word; extra unquoted words produce a parse error.
- Empty value allowed (`name =`).
- Assignments cannot be pipeline stages or carry redirections.
- Assignments allowed as operands of `&&`/`||` and inside groups.

### Expansion

- Occurs at evaluation time, not during parsing.
- `$name` and `${name}` expand to variable value (empty string if undefined).
- `$?` expands to last command exit status.
- `$$` expands to shell process ID.
- Double-quoted variables expand; single-quoted stay literal.
- Escaped `\$` stays literal.
- One word → one argument; no field splitting or re-parsing.
- Expansion applies to command names, arguments, and redirection targets.

### Environment

- `VariableStore` snapshots process environment at session creation.
- Three layers: local variables (with export flag), hidden inherited names, inherited base.
- Windows: case-insensitive keys; Unix: case-sensitive.
- Child env = inherited (minus hidden) + exported locals + PWD + overrides.
- `Session::with_environment` for deterministic tests.

### Special variables

- `$?` — updated after every command/pipeline/assignment:
  - 0 on success / built-in success / assignment
  - External exit code
  - 1 for built-in failure
  - 2 for syntax/lex errors
  - 128+signal for signal termination
- `$$` — process ID from `std::process::id()`.
- `$PWD` — initialized exported, updated by `set_working_directory`.
- `$HOME` — initialized exported from env or `USERPROFILE`.

### Pipeline behavior

- Built-ins in pipelines use cloned `Session`; `export`/`unset` inside pipeline do not leak to parent.
- Single command still mutates real session.
- `$?` updated from final stage outcome.

## Configuration

- Model: `crates/shell-config/src/lib.rs`.
- CLI loads `Config::default_path()` with `load_or_default` at startup.
- Missing file → defaults. Invalid file → warning plus defaults.
- Fields: `prompt`, `history_size`, `color`, `history_enabled`, `history_file`,
  `history_filter_secrets`, `suggestions_enabled`, `completion_hidden_files`,
  `startup_animation`, `quiet_startup`, `animation_speed`, `accent_color`,
  `secondary_color`, `highlight_color`, `success_color`, `error_color`,
  `prompt_color`, `terminal_font`, `apply_terminal_font`.
- Defaults: prompt `kernext ~ $ `, `color = true`, `startup_animation = true`,
  `quiet_startup = false`, `animation_speed = "normal"`,
  `accent_color = "bright-cyan"`,
  `secondary_color = "bright-blue"`, `highlight_color = "bright-magenta"`,
  `success_color = "bright-green"`, `error_color = "bright-red"`,
  `prompt_color = "bright-cyan"`, `terminal_font = ""`,
  `apply_terminal_font = false`.
- Colors accept ANSI names (`cyan`, `bright-cyan`), `colorN`/bare `0`–`255`,
  and `#RGB`/`#RRGGBB`. Validate every color with `parse_color`.
- `terminal_font` must be control-character-free; `apply_terminal_font = true`
  requires a non-empty font. OSC 50 is a hint, not a guarantee.
- `SHELL_NAME` determines config directory names (`kernext`).
- `history_file` resolves relative to the config directory unless absolute.
- Keep unknown fields rejected (`deny_unknown_fields`) and `history_size > 0`.

## Built-in behavior

- Registry and command metadata: `crates/shell-builtins/src/registry.rs`.
- Implementations: `crates/shell-builtins/src/commands.rs`.
- AST evaluator: `crates/shell-builtins/src/eval.rs`.
- 56 commands: help, pwd, cd, ls, tree, find, stat, mkdir, touch, cat, head,
  tail, wc, grep, sort, uniq, echo, print, input, len, type, str, int, float,
  bool, cp, mv, rm, which, basename, dirname, realpath, clear, history, jobs,
  wait, kill, fg, bg, run, test, `[`, expr, sleep, true, false, whoami, date,
  host, sysinfo, version, exit, export, unset, env, set. `[` is the bracket
  alias of `test` and requires a closing `]`.
- `print`/`input`/`len`/`type`/`str`/`int`/`float`/`bool` are the native
  scripting helpers. Value commands print a bare value (no trailing newline,
  like `expr`/`wc`); `print` behaves like `echo`. `input <variable>` reads one
  line from pipeline or `<` input and assigns it; it never reads the terminal,
  so interactive use without redirection errors clearly.
- `version` prints `KERNEXT v<version>`. `help` starts with
  `KERNEXT — Available commands:`.
- `Session::working_directory` is the single source of truth for relative
  paths. `cd` validates/canonicalizes and updates the session; it deliberately
  does not call process-wide `env::set_current_dir`, preserving test isolation.
- Lists (`;`), groups, `&&`/`||`, pipelines, redirections, and assignments are evaluated.
- Pipelines bind more tightly than `&&` and `||`. Quoted operators are ordinary
  argument text.
- `CommandResult` keeps stdout and stderr separate. `CommandContext::stdin`
  carries pipeline or `<` input. `cat` with no operands reads that input and
  reports a missing operand only when no input was supplied.
- Non-built-in names go to `process.rs`, never to `split_whitespace` or another
  shell. Quotes are decoded by the lexer; arguments stay boundary-preserving.
- `date` uses `chrono`; `shell-builtins` also depends on `shell-core` for the
  lexer/parser and `std::process` for external execution.
- Keep Linux, macOS, and Windows compatibility. Use `std::path`, resolve home
  from `HOME`/`USERPROFILE`, and avoid Unix-only APIs.

## Tests

- Filesystem tests use isolated temp dirs and call
  `Session::set_working_directory`; never mutate the process cwd in tests.
- Add focused tests next to the behavior: unit tests in `src/`, integration
  tests in `crates/shell-builtins/tests/`, `crates/shell-core/tests/`,
  `apps/shell-cli/tests/`, and `apps/shell-server/tests/`.
- External-process tests live in `crates/shell-builtins/tests/external.rs` and
  use the test binary itself as a helper via `CUSTOM_SHELL_TEST_HELPER`. Do not
  replace them with tests that require a particular external program.
- Pipeline tests live in `crates/shell-builtins/tests/pipeline.rs` and use the
  same test binary as a helper via `CUSTOM_SHELL_PIPELINE_ARGS`. Their marker
  lines (`PIPELINE:`) separate helper output from libtest harness output.
- Variable tests: `crates/shell-runtime/src/variables.rs` (unit tests),
  `crates/shell-builtins/src/expand.rs` (expansion unit tests).
- Startup tests live in `apps/shell-cli/src/startup.rs`; clear/Ctrl+L tests live
  in `apps/shell-cli/tests/repl.rs` and
  `crates/shell-line-editor/src/editor.rs`.
- The full suite currently has 469 tests on Windows (467 unit/integration plus
  2 `shell-core` doc-tests), including 20 in
  `crates/shell-builtins/tests/windows_scripts.rs`, 2 in `external_tools.rs`,
  16 in `shell-desktop`, and 17 in `shell-server` (unit, `api`, `consistency`,
  and `ws` integration suites). Unix runs
  signal/abort tests instead of the Windows launcher and ConPTY suites; do not
  weaken or delete tests to make a build pass.

## Git and uncommitted work

The repository has multiple commits. The working tree may contain uncommitted
changes and untracked files. **Never** run `git reset --hard`,
`git checkout -- .`, `git clean`, or any command that discards working-tree
changes without explicit permission. Do not commit, amend, rebase, or push
unless asked.

Before risky migration or cleanup, snapshot tracked/untracked work outside the
repo.

## Documentation

Keep `README.md`, `AGENTS.md`, `CLAUDE.md`, `CONTRIBUTING.md`,
`RELEASE_NOTES.md`, `docs/BRANDING.md`, `docs/ARCHITECTURE.md`,
`docs/COMMANDS.md`, `docs/DESKTOP.md`, `docs/SECURITY.md`,
`docs/DEPLOYMENT.md`, `docs/TROUBLESHOOTING.md`, and
`docs/PROJECT_STATUS.md` in sync with behavior. Trust the source and tests if
docs disagree. Web and Desktop are implemented surfaces now; describe their
actual behavior and their verification state rather than calling them future
plans. The built-in count in prose must always match
`crates/shell-builtins/src/registry.rs` (`DEFAULT_BUILTINS`).
