# KERNEXT Project Status

Status snapshot after Phase 13 (responsive Web Terminal and site layouts). The CLI, Web,
and Desktop surfaces share one Rust engine. MariaDB-backed accounts,
authentication, and per-account web-terminal ownership are implemented. This
document separates what is **implemented**, **tested**, **partially tested**,
and still **unverified**. It is the authoritative completion snapshot;
`README.md` describes usage and `docs/ARCHITECTURE.md`,
`docs/BACKEND_ARCHITECTURE.md`, `docs/DATABASE.md`, and
`docs/AUTHENTICATION.md` describe the system.

Verification host: Windows 11, Rust GNU toolchain (WinLibs MinGW-w64),
Node.js + npm. MariaDB is available on `127.0.0.1:3307`.

## Implemented

### Shell engine (CLI)

- Own lexer, parser, AST, evaluator; 56 built-in commands (including `host`,
  `sysinfo`, and the registered `[` alias of `test`); direct external process
  execution without `sh -c`/`cmd /C` for KERNEXT syntax.
- Pipelines over real OS pipes; `<`, `>`, `>>`, `2>`, `2>>`; `;`, groups,
  `&&`/`||`.
- Variables, expansion (`$name`, `${name}`, `$?`, `$`), `export`/`unset`/`env`/
  `set`, functions, `if`/`else`, `while`, `for`, `return`/`break`/`continue`.
- Background jobs with a job table and `jobs`/`wait`/`kill`/`fg`/`bg`.
- Startup presentation, genuine `clear`/Ctrl+L, history, completion, TOML
  configuration.

### Web and Desktop

- Local Axum server with REST sessions, engine WebSocket, and real PTY
  WebSocket (ConPTY on Windows, `openpty` on Unix); static SPA serving with
  route metadata and SPA fallback.
- React 19 + Vite + Tailwind front end: public site, web terminal, account
  pages, and account dashboard.
- Tauri 2 desktop app reusing the web front end, executing through the local
  Rust runtime only; engine and PTY sessions, dashboard, command/file
  explorers, sessions, account, settings.
- Desktop account bridge keeps the session cookie in the OS credential store;
  local shell works offline without sign-in.

### Accounts, MariaDB, and authentication

- SQLx `MySqlPool` (max 10, min 1, 5-second acquire timeout), startup
  connectivity check, embedded idempotent migrations, 60-second session purge.
- Schema: `users`, `user_sessions`, `password_reset_tokens`,
  `email_verification_tokens`, `user_preferences`; FKs with cascade, unique
  email/token constraints, indexes, UTC `DATETIME(6)`.
- Argon2id password hashing with equalized unknown-account timing.
- Registration, login, logout, current user, profile update, preferences,
  change password, session listing/revocation, forgot/reset password, email
  verification, resend verification.
- HttpOnly `kernext_session` cookie, readable `kernext_csrf` cookie, CSRF
  double-submit on state-changing requests, `SameSite=Strict`, `Secure` and HSTS
  in production.
- Fixed-window rate limits on register/login/recovery/verification; generic
  invalid-credential and forgot-password responses.
- SMTP delivery via `lettre`; `KERNEXT_DEV_MAIL=1` development token echo
  outside production; delivery failures are reported, never pretended.
- Terminal authorization: account required in account mode; every terminal
  session bound to its owner; cross-user access returns `404`; per-account caps
  (4 engine, 2 PTY); origin and cookie validation on WebSocket upgrades.
- `GET /api/v1/health/database` reports connectivity and latency.
- Security headers on every response (`nosniff`, `DENY` framing,
  `no-referrer`, permissions policy, CSP, production HSTS).
- Configurable origin allowlist (`KERNEXT_ALLOWED_ORIGINS`) shared by CORS and
  request validation; `APP_PUBLIC_URL` for emailed links.
- `.env.example` placeholders only; `.env` is gitignored.

### Hero and administration (Phase 12)

- The public site uses a minimalist engineering direction: a centered editorial
  introduction, restrained surfaces, a subdued navigation bar, and a shared
  dark palette. Homepage sections cover the CLI, Web, Desktop, verified
  capabilities, supported platforms, the shared Rust architecture, and a
  clearly labelled static terminal example with a link to the real Web Terminal.
- The hero uses the existing KERNEXT mark on a CSS 3D metal-effect core with
  two independently rotating orbital planes. Perspective, depth separation,
  low-amplitude pointer parallax, automatic motion, and reduced-motion support
  are implemented without adding a 3D dependency.
- Official Windows, Linux, and Apple SVG marks are bundled locally for the
  homepage and downloads. Shared cards allow long documentation code samples to
  scroll within their panels instead of widening the mobile page.
- Hero copy retains the official tagline, brand statement, and the concise
  product description; primary actions open the Web Terminal and Features.
- `users.role` (`user`/`admin`) and `users.must_change_password` migration.
- Idempotent development administrator seed (`KERNEXT_SEED_ADMIN`), ignored in
  production, Argon2id-only, never logging credentials, never overwriting an
  existing email.
- Admin-only `GET /api/admin/users`, backend role checks, and forced password
  change (`403 password_change_required`) on account and terminal features.
- Deterministic env discovery (`KERNEXT_ENV_FILE`, `./.env`, workspace `.env`),
  explicit production `DATABASE_URL` requirement, URL scheme validation, and
  post-migration schema verification.
- Sign-in distinguishes an unreachable server from a server without accounts.

### Responsive terminal and site layouts (Phase 13)

- The browser Web Terminal has Compact, Standard, Large, and custom dimensions;
  pointer/touch drag and keyboard resizing; fit-to-window; reset; and fullscreen
  with a viewport-sized Escape-dismissible fallback when the browser denies
  its fullscreen request. Resizing does not remount the terminal panel.
- Terminal rows and columns continue to come from measured glyph dimensions.
  Engine buffers resize immediately; PTY resize messages are coalesced to at
  most one every 80 ms while dragging and send the latest measured size.
- Font family and size, line height, padding, cursor style, background opacity,
  and four terminal color themes are configurable. Versioned preferences are
  validated and saved in local storage; signed-in web users also sync them
  through the existing account preference endpoint. Tauri terminal defaults
  and execution remain unchanged.
- Public mobile navigation and the admin drawer provide Escape handling,
  outside/backdrop dismissal, focus containment, and body scroll locking.
- Responsive route checks covered 30 public, account, terminal, admin, and
  desktop routes at all eight requested viewport widths/heights used in this
  pass (1920×1080, 1440×900, 1366×768, 1024×768, 768×1024, 430×932,
  390×844, and 360×800); no page-level horizontal overflow or uncaught
  browser exceptions were observed.

## Tested

- `cargo fmt --all -- --check` — clean.
- `cargo check --workspace --all-targets` — clean.
- `cargo test --workspace` — 470 workspace tests passed before Phase 11
  hardening; the shell-server suite now also contains 7 MariaDB integration
  tests (see below).
- `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- `npm run typecheck` — ok.
- `npm test` (Vitest) — 58 passed in 8 files.
- `npm run build` — ok.
- **Live MariaDB integration** (`cargo test -p shell-server --test auth_db`
  against the configured database on port 3307): **9/9 passed**.
  - migrations apply (and are re-runnable)
  - registration, duplicate-email conflict, login, logout
  - revoked session cookie cannot be reused
  - CSRF rejection and acceptance on profile updates
  - single-use password reset; reset revokes sessions; old password rejected
  - email verification updates the public profile
  - password hash is Argon2id and only token hashes are stored
  - terminal sessions are owner-scoped (cross-user read/execute returns 404)
  - repeated failed logins are rate limited
  - admin seed is idempotent, stores Argon2id, forces password change, and
    admin endpoints reject non-admins and un-changed passwords
- **Redesign browser verification:** Chrome headless against Vite, with
  `prefers-reduced-motion` emulated both ways. At 1920×1080, 1440×900,
  1366×768, 768×1024, and 390×844, the home page had no horizontal overflow;
  the core and both orbital transforms changed over time without hover. Pointer
  movement produced a 3D matrix transform, and reduced motion disabled all
  three continuous animations. Navigating to Features and back retained the
  animation. The local platform SVGs loaded successfully.
- **Responsive route rendering:** Chrome headless visited Home, Features,
  Documentation, Downloads, About, Contact, sign-in, account creation,
  Terminal, and Admin at 1440px and 390px. No console errors or horizontal
  overflow were observed; Admin redirected to sign-in while unauthenticated.
  Documentation's long API examples initially exposed a mobile width issue;
  constraining shared cards fixed it, and the route scan passed afterward.
- **Core motion:** headless Edge with `prefers-reduced-motion: no-preference`
  confirmed the KERNEXT core's Y-axis transform changes over time and the two
  orbital tracks animate independently. Emulating reduced motion disabled the
  continuous animations as expected.
- **Responsive terminal controls:** headless Edge verified the compact preset
  height, appearance controls, local preference storage and reload restoration,
  pointer resizing, and Escape restoration from the window-sized fullscreen
  fallback. The headless browser denied the native Fullscreen API request, so
  native fullscreen was not visually verified in a headed browser.
- **Real browser login:** Chrome headless against Vite on 5173, proxied to
  Axum on 7878. `/sign-in` showed no configuration or unreachable warning;
  submitting `admin@admin.com` / `password` navigated to `/account` with the
  Administrator badge and first-login password banner; reloading the page kept
  the session signed in. Through the same proxy, an invalid password returned
  `401`, valid login `200`, logout `204`, and `/api/auth/me` returned `401`
  after logout.
- **PTY authorization and handshake:** the seeded admin's PTY upgrade returned
  `403 password_change_required` as designed. The terminal page now detects
  that flag and renders the change-password gate without opening a socket. A
  freshly registered account reached `/terminal`, opened
  `ws://127.0.0.1:5173/api/v1/pty`, reported PTY mode and dimensions, and ran
  `echo PTY_SMOKE_OK` successfully; the temporary account was removed after the
  check.
- **Production front end:** `npm run build` now writes a `dist/index.html`
  pointing at the hashed entry JS and extracted CSS (`scripts/build-index.mjs`)
  instead of the development `/src/main.tsx`, so the served site renders.
- **Launch-directory independence:** the server started from
  `apps/shell-server` loaded the workspace `.env`, enabled MariaDB accounts,
  and served the built front end. Admin login returned `200` with role `admin`
  and `must_change_password: true`; the admin list correctly returned `403`
  until the password changes.
- **CLI without MariaDB:** `kernext version` runs with no `DATABASE_URL`,
  confirming local execution remains independent of accounts.
- Web terminal acceptance scripts (`node scripts/e2e-terminal.mjs`,
  `node scripts/e2e-browser.mjs`) passed in earlier phases against the Vite
  proxy and the production bundle served by Axum.

## Partially tested

- **SMTP delivery.** The `lettre` path and delivery-status reporting are
  implemented, but no real SMTP credentials are configured on this host, so an
  actual email was not sent. Development token echo is verified. Production
  email delivery remains an owner-provided configuration.
- **Interactive desktop GUI.** Typing, tab switching, resizing, Ctrl+C, and
  settings were not driven interactively; the launch smoke test and shared web
  terminal tests ran.
- **Live PTY resize in this pass.** Browser layout and controls were checked
  with auth/health responses isolated for UI testing, but no authenticated PTY
  session was used. The existing authenticated PTY command flow and the
  process's response to a resize were therefore not re-verified; visual
  resizing still uses the established WebSocket resize path.
- **Hosted CI.** The workflow now includes a MariaDB service container and the
  `auth_db` job, but a hosted run after this change has not been observed.
- **macOS and Linux GUI.** No macOS host and no mounted Linux checkout were
  used for this verification.
- **Installer installation.** Bundles were built in earlier phases; installing
  them was not repeated.

## Known limitations

- Engine WebSocket sessions are line-oriented: an interrupt clears the client
  line but cannot cancel an already-running engine command. PTY mode forwards
  Ctrl+C to the real process.
- Engine sessions have no terminal size; resize is client-rendering only.
- CLI signal handling and process-group job control remain unimplemented
  (`fg` waits; `bg` only reports state).
- No asynchronous background-job completion notice.
- No `source`/`.` loading, function-call redirections, globbing, script
  comments, or `2>&1` merge form.
- Plain `cargo build --workspace --release` compiles the desktop crate in dev
  mode; `tauri build` produces distributable binaries.
- Engine-session caps and IP-based rate limiting are in-process. A multi-replica
  deployment would need shared limits.

## How to re-verify

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
npm ci
npm run typecheck
npm test
npm run build
```

MariaDB account integration (disposable database; the harness refuses a
`DATABASE_URL` fallback unless its database name contains `test`):

```sh
KERNEXT_TEST_DATABASE_URL=mysql://kernext:CHANGE_ME@127.0.0.1:3307/kernext_test \
  cargo test -p shell-server --test auth_db -- --nocapture
```

Start the server with `.env` present to apply migrations and exercise the API:

```sh
cargo run -p shell-server
curl -s http://127.0.0.1:7878/api/v1/health
curl -s http://127.0.0.1:7878/api/v1/health/database
curl -s http://127.0.0.1:7878/api/auth/status
```

## Remaining owner input

- SMTP credentials (`SMTP_HOST`, `SMTP_PORT`, `SMTP_USERNAME`,
  `SMTP_PASSWORD`, `SMTP_FROM`) for real verification/reset email.
- A production `APP_PUBLIC_URL` and `KERNEXT_ALLOWED_ORIGINS` for deployment.
- A disposable test database or CI service container (CI provides one).
