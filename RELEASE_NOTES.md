# KERNEXT Release Notes

**The Next Core of Computing.**  
OPEN • POWERFUL • INDEPENDENT — Build. Execute. Innovate.

## v0.1.0 (Current)

### Build and release tooling
- The repository is pnpm-only. `package-lock.json` was removed and
  `pnpm-lock.yaml` is the single authoritative lockfile; npm, yarn, and bun are
  not supported. pnpm settings live in `pnpm-workspace.yaml`.
- `scripts/build-web.mjs` is pnpm-only and fails with a clear message when pnpm
  is missing; `KERNEXT_PACKAGE_MANAGER` was removed. `kernext doctor` now checks
  for pnpm and `pnpm-lock.yaml` instead of npm and `package-lock.json`.
- A single release workflow, `.github/workflows/release.yaml`, builds the CLI
  archives and the desktop bundles and publishes one non-draft GitHub Release
  marked Latest. `.github/workflows/desktop-release.yaml` was removed.
- Desktop releases now cover Linux arm64 and produce RPM packages; canonical
  installer names embed the architecture.
- The server exposes `GET /api/v1/releases/latest` (newest stable release plus
  classified assets) and `GET /api/v1/releases` (release history for the
  previous-releases list), both served from one cached GitHub lookup.

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
- **56 native built-ins**, covering filesystem/text operations, environment and
  scripting helpers, job control, and shell lifecycle commands. See `help` or
  the registry for the complete list.
- **External process execution**: direct `std::process::Command` execution
  without shell wrappers. Windows `.cmd`/`.bat` programs run through the system
  command interpreter with escaped arguments; `.exe`/`.com` launch natively.
- **Windows external command fix**: npm-installed tools such as `opencode` and
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
- **Linux (GNU and musl)**: canonical target; release automation builds both.
- **macOS (Apple Silicon and Intel)**: source-compatible; the release workflow
  builds both architectures on GitHub runners. Interactive macOS verification is
  still open.
- **Windows x64**: source-compatible and exercised by the test suite; the
  release workflow builds MSVC artifacts on GitHub runners and the GNU build
  is verified on the development host.

### Desktop distribution
- `.github/workflows/release.yaml` publishes desktop installers for Windows x64
  (`kernext-desktop-windows-x64.msi`,
  `kernext-desktop-windows-x64-setup.exe`), Linux x64 and arm64
  (`kernext-desktop-linux-x64.AppImage`, `kernext-desktop-linux-x64.deb`,
  `kernext-desktop-linux-x64.rpm`, and the `arm64` equivalents), and macOS
  (`kernext-desktop-macos-arm64.dmg`, `kernext-desktop-macos-x64.dmg`). It runs
  on a `v*` tag push or manual dispatch, verifies the workspace first, publishes
  a non-draft GitHub Release marked Latest, and attaches a `.sha256` checksum to
  every installer (plus a minisign `.sig` beside a signed installer).
- A release tag must match the workspace `Cargo.toml`, `tauri.conf.json`, and
  `package.json` versions; `scripts/check-release-version.mjs` enforces this
  before anything is built.
- Installers are unsigned by default and run without any secret. Windows
  SmartScreen and macOS Gatekeeper may warn on first launch; see
  `docs/RELEASING.md` for the optional signing setup.
- The website's downloads page reads the real release metadata from
  `GET /api/v1/releases/latest` and only shows files that exist; the
  previous-releases list reads `GET /api/v1/releases`. Both are served from one
  cached GitHub lookup.
- pnpm is the only supported package manager; `pnpm-lock.yaml` is the single
  lockfile.

### Security
- No command injection through a shell wrapper.
- No `eval` or dynamic code execution.
- Arguments are passed as separate `Command::arg` values.
- No `sh -c`, `bash -c`, or `cmd /C` is used.
- Windows `.cmd`/`.bat` scripts launch through the system command interpreter
  (`%COMSPEC%`) with a dedicated, pre-escaped command line; arguments containing
  `"`, `%`, CR, LF, or NUL are refused rather than rewritten.
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
- The release workflow covers every platform but has not yet been executed on
  GitHub. `release.yaml` is the single workflow: it publishes a real, non-draft
  release for a version tag. No hosted run has been observed, and no installer
  has been produced by a hosted runner yet.

---

## Future roadmap

### CLI hardening
- Script sourcing and redirections on function calls.
- Asynchronous job notifications and full foreground process-group hand-off.
- Shell-wide signal handling and terminal ownership.
- Globbing, pattern matching, and script comments.
- Richer diagnostics and completion for command flags.

### KERNEXT Web — implemented
- Local Axum server (`apps/shell-server`) with engine sessions and a real
  pseudoterminal relay (ConPTY on Windows, `openpty` on Unix).
- Loopback bind by default, optional bearer token, fixed browser-origin
  allowlist, session caps, and idle cleanup — see `docs/SECURITY.md`.
- React front end (`apps/web`) with the graphical splash and
  mountain-wallpaper identity, the full public website (home, features,
  documentation, downloads, about, contact), and an interactive browser
  terminal. The site's version badge comes from the workspace version at build
  time.

### KERNEXT Desktop — implemented
- Native desktop application (`apps/desktop`) built on Tauri 2, reusing the
  web front end with a local Rust runtime: multi-tab terminal (engine or PTY),
  dashboard, command explorer, file explorer, sessions, settings, and about
  screens.
- Desktop commands execute through the local Rust runtime only — never the
  public web server, never a JavaScript shell simulation.
- Windows release bundles (MSI and NSIS installers) build successfully through
  `pnpm run desktop:build`; the release binary passes a launch smoke test.
- Desktop installers for Windows, Linux (x64 and arm64), and macOS are built and
  published by `.github/workflows/release.yaml`; see `docs/RELEASING.md`.
- Verified on Windows; interactive Linux/macOS verification, a first hosted
  release run, and macOS bundle verification remain open. See
  `docs/PROJECT_STATUS.md`.

### Product integration (Phase 6)
- **New built-ins**: `host` (system-resolver DNS lookup) and `sysinfo`
  (platform/runtime report), plus the previously missing `[` registry entry
  so the documented bracket alias of `test` actually dispatches as a built-in
  on every platform.
- **Cross-interface consistency suite**: HTTP sessions and the CLI engine are
  checked for identical help, version, `pwd`/`cd`, `ls`, `mkdir`, `echo`,
  pipeline, redirection, and environment behavior; HTTP sessions now persist
  directory changes the same way the CLI does.
- **External-tool compatibility tests**: `git`, `cargo`, `rustc`, `node`,
  `npm`, `python`, `curl`, `code`, and `opencode` run through the full engine
  (absent tools are skipped, never assumed).
- **CI**: lint, web, test, and desktop-bundle matrices across Windows x64,
  Linux x64, and macOS (Apple Silicon + Intel) on Node 24 action versions.
- **Releases**: a single tag-triggered workflow covering CLI archives (GNU,
  musl, Windows, macOS both architectures, each including `kernext-server`),
  desktop bundles, and SHA-256 checksums for every artifact.
- **Documentation**: command reference (`docs/COMMANDS.md`), security model
  (`docs/SECURITY.md`), deployment guide (`docs/DEPLOYMENT.md`),
  troubleshooting (`docs/TROUBLESHOOTING.md`), and contribution guide
  (`CONTRIBUTING.md`).
- **Fixed**: the root `pnpm run desktop:dev` / `pnpm run desktop:build` scripts, which
  previously failed with "None of the selected packages has a 'tauri' script".
