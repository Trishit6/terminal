# KERNEXT — Repository Knowledge

## What this is

**KERNEXT** ("The Next Core of Computing.") is an independent, cross-platform
shell written in Rust with its own lexer, parser, AST, evaluator, built-ins,
pipeline engine, and external process executor. It never wraps Bash, PowerShell,
or CMD. Three surfaces share one engine:

- **CLI** — `apps/shell-cli`, binary `kernext` (interactive REPL, startup, dev commands)
- **Web** — `apps/shell-server` (local Axum server) + `apps/web` (React + Vite + Tailwind)
- **Desktop** — `apps/desktop` (Tauri 2, crate `shell-desktop`, binary `kernext-desktop`)

Internal crate/package names stay `shell-*`; only user-facing names are KERNEXT.

### Where code lives

| Path | Responsibility |
| --- | --- |
| `crates/shell-core` | Branding constants, lexer, parser, AST, spans, `ShellError` |
| `crates/shell-runtime` | `Session` (cwd, config, variables, functions, lifecycle), jobs |
| `crates/shell-builtins` | Registry (56 commands), `eval.rs`, `commands.rs`, `pipeline.rs`, `process.rs`, `expand.rs` |
| `crates/shell-config` | TOML config model + validation |
| `crates/shell-line-editor` | Reedline integration, clear helpers |
| `crates/shell-api` | Serializable session/terminal API contracts |
| `apps/shell-cli` | Clap args, startup renderer (`startup.rs`), REPL (`repl.rs`), `dev.rs` project commands, `worker.rs` embedded worker used by desktop PTY sessions |
| `apps/shell-server` | Axum HTTP + engine/PTY WebSockets, MariaDB accounts, email, CSRF |
| `apps/web` | React front end; terminal renderer in `src/lib/terminal/` and `src/components/terminal/` |
| `apps/desktop` | Tauri 2 app; Rust runtime in `src-tauri/` |
| `scripts/dev.mjs` | Starts backend + Vite dev stack |

Execution flow: `input → shell-core lexer → parser → AST → shell-builtins evaluator → pipeline → registry OR process executor → CommandOutcome → REPL`. Never split input with `split_whitespace`; `Dispatcher` uses `lex_spanned` + `parse`.

Dependency direction: `shell-cli → shell-builtins → shell-runtime → shell-config → shell-core`; `shell-server → shell-api → shell-builtins`; `shell-desktop → shell-server` (engine sessions) + `shell-cli` (embedded worker). The lexer/parser are hand-written — no pest/nom/lalrpop.

## Commands

### Rust (KERNEXT project commands, or Cargo directly)

```sh
kernext fmt --check   # cargo fmt --all -- --check
kernext check         # cargo check --workspace --all-targets
kernext build         # cargo build --workspace
kernext test          # cargo test --workspace
kernext lint          # cargo clippy --workspace --all-targets -- -D warnings
kernext run           # cargo run -p shell-cli
kernext clean         # cargo clean
kernext doctor        # environment/project inspection
```

All accept `--dry-run` and return the underlying tool's exit code. Cargo equivalent commands work too.

Focused tests:

```sh
cargo test -p shell-builtins --test builtins       # also --test pipeline, external, external_tools
cargo test -p shell-core --test parser
cargo test -p shell-cli --test repl
cargo test -p shell-server
cargo test -p shell-desktop
```

### Front end / desktop (npm workspaces, Node 22+, npm 10+)

```sh
npm ci                  # install apps/web + apps/desktop from lockfile
npm run dev             # scripts/dev.mjs: shell-server (7878) + Vite (5173)
npm run typecheck       # web typecheck
npm test                # web Vitest suite
npm run build           # web production build
npm run web:dev         # Vite only
npm run desktop:dev     # tauri dev
npm run desktop:build   # tauri build
cargo run -p shell-server
```

Interactive shell needs a real TTY; piping stdin is rejected. Use a terminal, `screen`, or `tmux`.

## Conventions & gotchas

### Build environment (critical)
- Repo may live on a **vfat `showexec`** mount where extensionless binaries can't execute. `.cargo/config.toml` sets `build.target-dir = "/home/trishit/.cache/terminal-target"` (ext4). Never create a repo-local `target/`, never change mount options, never hardcode a target dir in `Cargo.toml`.
- **Windows GNU (MinGW):** `apps/desktop/src-tauri/build.rs` must keep shadowing the MinGW `default-manifest.o` on `*-pc-windows-gnu` (else `STATUS_ENTRYPOINT_NOT_FOUND`). GNU targets use `WindowsAttributes::new_without_app_manifest()`; MSVC/non-Windows keep Tauri's default.
- Keep Rust `tauri` crate and npm `@tauri-apps/api` on the same major/minor. `tauri.conf.json` `frontendDist` is `../../web/dist`.

### Engine rules
- Built-ins always win over external programs; functions win over external but lose to built-ins.
- External execution (`process.rs`) never uses `sh -c`/`cmd /C`/PowerShell. One exception: Windows `.cmd`/`.bat` go through `%COMSPEC%` with `/d /v:off /s /c` (launcher, not parser). Args containing `"`, `%`, CR, LF, NUL are refused.
- Windows resolution honors `PATHEXT` only for `.com/.exe/.bat/.cmd`; **extensionless files are never executables** (npm POSIX shims must resolve to `.cmd`).
- Pipelines (`pipeline.rs`): external stages use real OS pipes; built-ins exchange buffered bytes; built-ins in a pipeline run on a cloned `Session` (state changes don't leak). Exit code/success come from the final stage.
- Redirections: `<`, `>`, `>>`, `2>`, `2>>`; resolve via `commands::resolve_path`; `2>` recognized only when `2` starts a token. Grouped pipeline stages unsupported ("Grouped pipelines are not supported yet.").
- `clear`/`exit` actions propagate only from a single command, never a pipeline stage.
- Variables: assignment `name = value`; expansion `$name`, `${name}`, `$?`, `$$`; no field splitting; one word → one argument.
- Startup renderer is presentation-only; never fake progress. Animated/Plain/Quiet modes; `NO_COLOR`/`TERM=dumb`/non-TTY disable ANSI. Emblem row widths are a test invariant. No artificial delays.
- `clear` uses crossterm CSI sequences; never shell out to `clear`/`cls`; suppressed when stdout isn't a terminal. Ctrl+L is Reedline's default binding — verify by test, don't rebind.

### Web terminal
- DOM renderer (rows of `div.terminal-row`), not canvas; no xterm.js or terminal dependencies. `apps/web/src/lib/terminal/` is unit-tested in Node.
- **Never set a single-font `font-family`** — always append the monospace fallback stack (`withMonospaceFallback`).
- `useTerminalConnection.resize` must call `engine.resize(rows, cols)` in both PTY and engine mode, then send `resize` to the PTY (guard with previous-size check).
- `/terminal/render-test` is a backend-free visual regression fixture; keep it working.
- Motion config in `main.tsx` respects Reduced Motion in prod; don't remove it.

### Web server / accounts
- **Account mode** when `DATABASE_URL` is set (MariaDB, Argon2id, HttpOnly cookie sessions, CSRF double-submit, per-account terminal sessions); **local mode** (loopback-only, optional `KERNEXT_SERVER_TOKEN`) otherwise.
- `.env` at workspace root loaded via `dotenvy`; real env vars win. Never commit `.env`. Local dev DB: `mysql://...@127.0.0.1:3307/...`.
- Migrations in `apps/shell-server/migrations/` must be idempotent and must not drop tables.
- Tokens use OS CSPRNG, only SHA-256 hashes persisted; register/login/verify/reset rate-limited. `KERNEXT_DEV_MAIL=1` returns tokens instead of sending email. SMTP via `SMTP_*` vars.
- Web `apps/web/src/lib/auth.ts`; desktop keeps session cookie in OS credential store (`src-tauri/src/account.rs`).

### Config
- Model: `crates/shell-config/src/lib.rs`; loaded from `Config::default_path()` via `load_or_default`. Missing → defaults; invalid → warning + defaults. Unknown fields rejected (`deny_unknown_fields`); `history_size > 0`.
- Key defaults: prompt `kernext ~ $ `, `color = true`, `startup_animation = true`, `animation_speed = "normal"`, `accent_color = "bright-cyan"`.
- Colors accept ANSI names, `colorN`/`0–255`, `#RGB`/`#RRGGBB`. `apply_terminal_font = true` requires non-empty `terminal_font`.

### Branding
- Constants in `crates/shell-core/src/lib.rs`: `SHELL_NAME = "kernext"`, `PRODUCT_NAME`, `TAGLINE`, `BRAND_STATEMENT`, `VALUES`, `MOTTO`, `VERSION`. Use `VERSION`, never hardcode versions. See `docs/BRANDING.md`.

### Known gaps (don't describe these as implemented)
- No asynchronous job-completion notice (user must run `jobs`/`wait`); `fg` waits instead of taking the terminal; `bg` only reports state; detached shell-expression jobs can't be interrupted from another thread.
- No `source`/`.` script loading, no redirections on function calls, no closures or arrays, no globbing, no comments in scripts.
- Grouped expressions are not valid pipeline stages (`(a; b) | c`), and signals/process groups aren't handled (Ctrl+C during startup animation is platform behavior).

### Tests
- Filesystem tests use isolated temp dirs + `Session::set_working_directory`; never mutate process cwd in tests.
- External/pipeline tests use the test binary itself as a helper (env `CUSTOM_SHELL_TEST_HELPER`, `CUSTOM_SHELL_PIPELINE_ARGS`); don't require specific external programs.
- Never weaken or delete tests to make a build pass.

### Git
- Never run `git reset --hard`, `git checkout -- .`, `git clean`, or anything that discards working-tree changes without explicit permission. Don't commit/amend/rebase/push unless asked. Snapshot risky work outside the repo first.
- Canonical checkout: `/run/media/trishit/UBUNTU 26_0/Projects/terminal` (Linux) / `D:\projects\React-Ts\terminal` (current Windows workstation). Don't edit duplicate checkouts or create new ones.

### Docs to keep in sync
`README.md`, `AGENTS.md`, `CLAUDE.md`, `CONTRIBUTING.md`, `RELEASE_NOTES.md`, and `docs/*` (`BRANDING`, `ARCHITECTURE`, `COMMANDS`, `DESKTOP`, `SECURITY`, `DEPLOYMENT`, `TROUBLESHOOTING`, `PROJECT_STATUS`), plus the accounts/backend docs (`AUTHENTICATION`, `BACKEND_ARCHITECTURE`, `DATABASE`) and `docs/architecture/`. Built-in count in prose must match `DEFAULT_BUILTINS` in `registry.rs` (56). Trust source and tests when docs disagree.
