# KERNEXT troubleshooting

Solutions to the problems that come up most often. Status of every claim here
matches the current workspace; see [`PROJECT_STATUS.md`](PROJECT_STATUS.md) for
verification state.

## Build problems

### `Permission denied (os error 13)` from cargo on Linux

The checkout is on a VFAT mount with `showexec`, which only executes
`.exe/.com/.bat/.btm/.cmd` names. Cargo's build scripts and test binaries are
extensionless, so they fail.

The fix already lives in `.cargo/config.toml`
(`build.target-dir = "/home/trishit/.cache/terminal-target"`), which routes
build artifacts to an ext4 location. Run cargo from the workspace root without
overriding the target directory, and never create a repo-local `target/`.

### `link.exe not found` / `ld: cannot find` on Windows

Rust needs the MSVC Build Tools (or a full MinGW-w64 for the GNU target).
Install "Visual Studio Build Tools" with the C++ workload and re-run
`cargo check`. For the desktop application see [`DESKTOP.md`](DESKTOP.md).

### Tauri build fails on Linux: missing `webkit2gtk`

The desktop application needs system libraries:

```sh
sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file \
  libxdo-dev libayatana-appindicator3-dev librsvg2-dev
```

The CI workflow installs exactly this set.

### `pnpm` says "None of the selected packages has a tauri script"

Older workspace scripts called `pnpm --filter @kernext/desktop tauri …`, which
runs *package scripts*, not binaries. The scripts are now
`pnpm desktop:dev` and `pnpm desktop:build`, which delegate to the package's
own `dev`/`build` scripts. Update any notes you kept.

### Node/pnpm version problems

The workspace pins `pnpm@11.25.0` via `packageManager`. Use Corepack
(`corepack enable`) or install that exact version; a different major of pnpm
may fail the frozen lockfile check.

## Running the shell

### "standard input is not a terminal" when piping into kernext

KERNEXT is interactive by design. Piping (`echo ls | cargo run -p shell-cli`)
is rejected. Use a real terminal, or `screen`/`tmux`. Scripts run through
`run script.rsh` or the engine API instead.

### A program works in bash but not in KERNEXT

- KERNEXT has no globbing yet: `ls *.rs` passes the literal `*.rs`. Expand
  names yourself or use `find . -name "*.rs"`.
- `grep` matches **literal** patterns, not regular expressions.
- Redirections exist (`<`, `>`, `>>`, `2>`, `2>>`) but there is no `2>&1`
  merge form yet.
- Windows `.cmd`/`.bat` arguments containing `"`, `%`, CR, LF, or NUL are
  refused — `cmd.exe` has no escaping for them.

### `npm`/`pnpm`-installed tools fail with `os error 193`

Older builds selected the extensionless POSIX `sh` shim that npm installs
next to the `.cmd` shim. Extensionless files are never executed on Windows;
current builds resolve the `.cmd`. Rebuild (`cargo build --workspace`) if you
still see it.

### `cd` inside a pipeline does not stick

Built-ins in a multi-stage pipeline run against a cloned session, so state
changes do not leak. This is deliberate: `cd dir | cat` prints the listing
without moving your shell. A single `cd dir` command still moves the real
session.

### Background job seems to vanish

There is no asynchronous completion notice. Run `jobs` to see it
(`running`/`done`/`failed`/`killed`), or `wait`/`fg <id>` to block on it.
`kill <id>` uses shell job numbers, never OS PIDs.

## Web and desktop

### The web terminal cannot connect

- Is `cargo run -p shell-server` running? The front end needs
  `http://127.0.0.1:7878` (or the port in `KERNEXT_SERVER_BIND`).
- With `KERNEXT_SERVER_TOKEN` set, the browser terminal sends the token via
  the WebSocket query parameter; the header form is used by the HTTP API.
- The dev server origins (`:5173`, `:4173`) and the bundled origin (`:7878`)
  are the only browser origins accepted — a page served from anywhere else is
  refused with "origin … is not allowed".
- PTY sessions need the `kernext` binary discoverable: a sibling of the
  server executable or `KERNEXT_BIN`.

### Sessions disappear after half an hour

Idle sessions are cleaned up by a background sweeper (60-second cadence,
30-minute idle threshold). Create a new session; state was in-memory anyway.

### Desktop window shows "engine" errors

Desktop commands run through the local Rust runtime, never the web server —
a server not running does not matter for the desktop app. If the PTY terminal
fails, the desktop spawns its own executable as the worker
(`KERNEXT_DESKTOP_WORKER=1`); run `pnpm desktop:build` again so the binary and
front end match.

### Desktop build aborts with a `@tauri-apps/api` version mismatch

The Rust `tauri` crate and the npm `@tauri-apps/api` package must stay on the
same major/minor. `pnpm install --frozen-lockfile` after changing either.

### STATUS_ENTRYPOINT_NOT_FOUND at desktop startup on MinGW

The MinGW default manifest lacks the Common Controls v6 dependency.
`apps/desktop/src-tauri/build.rs` shadows it with a replacement manifest on
`*-pc-windows-gnu` targets — do not "simplify" that build script. See
[`DESKTOP.md`](DESKTOP.md).

## Configuration

### Config changes have no effect

The CLI reads the config at startup only. Restart the shell. The file lives at
`%APPDATA%\kernext\config.toml` (Windows),
`~/Library/Application Support/kernext/config.toml` (macOS), or
`~/.config/kernext/config.toml` (Linux). A damaged file logs a warning and
falls back to defaults.

### Unknown config field errors

Unknown fields are rejected (`deny_unknown_fields`). Remove the stale field;
the accepted set is listed in [`README.md`](../README.md#configuration).
