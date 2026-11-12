# KERNEXT Project Status

Status snapshot after the Phase 15 visual redesign, which builds on
Phase 14 (responsive hardening) and Phase 13 (responsive Web Terminal and site
layouts). The CLI, Web, and Desktop
surfaces share one Rust engine. MariaDB-backed accounts,
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

### Responsive hardening (Phase 14)

- Terminal viewport limits are now render-only. Narrowing the window or
  rotating a phone no longer rewrites the saved width, so the terminal grows
  back to its requested size when space returns. Saved preferences keep the
  size the user chose; the frame is clamped at render time.
- Fullscreen is available from the size preset selector as well as the
  toolbar. Session tabs use separate select/close buttons, the close control is
  always visible and touch-sized on small screens, and the tab-add, settings,
  font stepper, mode, and apply-size controls meet a 40–44px touch target.
- The Desktop route's navigation rail collapses to icons below 640px and its
  content column may shrink, removing the clipped heading at 360px.
- The development stack no longer produces proxy stack-trace spam. `npm run
  dev` preflights ports 5173/7878, reuses a healthy backend, and waits for
  `/api/v1/health` before starting Vite. While the backend is down, the Vite
  proxy answers `503` with `{"error":"backend_unavailable"}` and logs one
  warning per outage; expected `ECONNREFUSED`/`ECONNRESET` proxy errors are
  filtered from Vite's logger.
- A persistent “Backend offline” pill is rendered outside the router, so every
  route shows the same API state. It stays while requests fail, does not cause
  horizontal overflow at 390×844, and its Retry button clears it once the
  backend answers. Two Vitest integration tests boot Vite's proxy against
  closed and fake backend ports to keep both the `503`/single-warning behavior
  and the no-stack-trace WebSocket reset behavior from regressing.
- Documentation and About tables scroll horizontally inside their own rounded
  containers instead of clipping columns. The 3D hero object no longer bleeds
  past the mobile viewport, and the site menu button is 44px on touch layouts.

### Visual redesign (Phase 15)

- A new site-wide art direction replaces the generic centered hero and
  three-equal-card section pattern: graphite/midnight surfaces, signal cyan and
  electric blue with restrained violet, a tighter display type scale, hairline
  rules, and a composed light theme. Tokens and the new visual system live in
  `apps/web/src/art-direction.css`; existing Tailwind theme variables are
  overridden so every route follows the same palette.
- The homepage is rebuilt as an editorial product page: an asymmetric hero
  with a live status line and CSS-3D core, a single bordered surface index for
  CLI/Web/Desktop previews, an execution-pipeline figure, a cross-platform
  band with original platform marks, a shared-engine flow diagram, a large
  static terminal example, and a closing wordmark band.
- The hero core now has a layered metallic body with a glass highlight, three
  independently rotating orbits, labeled Rust/PTY/cross-platform annotations,
  and pointer-responsive depth. It remains CSS 3D, requires no WebGL, and
  stops all motion under `prefers-reduced-motion`.
- The navbar is rebuilt around a centered navigation cluster with an active
  indicator, a persistent terminal action at every width, and a full-height
  mobile index with focus containment. The footer is rebuilt with a status
  masthead, four link columns, and a compact legal strip.
- Every public page now uses the shared `PageShell` header (kicker, display
  heading, lede, actions, optional spec aside). Features sections no longer use
  repeated cards for each capability. The 404 route renders inside the normal
  site frame instead of outside the layout.
- Terminal customization now closes the settings panel after Apply or a size
  preset, and the appearance controls use an auto-fitting grid so they remain
  readable inside a narrow panel. A persisted “Show output toolbar” option can
  hide the Clear/Copy/Interrupt/font toolbar without touching the session. The
  underlying session is not remounted.
- Vite pre-bundles `@radix-ui/react-tabs` and forces dependency optimization on
  dev-server start. Vite 8’s Windows cache on this checkout otherwise leaves
  only `deps_temp_*` directories, and the Docs/Account tabs chunk returns a
  stale 504 until the server is restarted.

## Tested

- `cargo fmt --all -- --check` — clean.
- `cargo check --workspace --all-targets` — clean.
- `cargo test --workspace` — 470 workspace tests passed before Phase 11
  hardening; the shell-server suite now also contains 7 MariaDB integration
  tests (see below).
- `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- `npm run typecheck` — ok.
- `npm test` (Vitest) — 61 passed in 9 files.
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
- **Phase 14 terminal acceptance (headless Chrome, real PTY):** 23/23 checks
  passed against `shell-server` with `pty_available: true`. A PTY session
  reported `13 × 144` rows × columns at 1280×900 and a child `node` process
  printed the same `144x13`; after resizing the viewport it reported `8 × 108`
  and the child printed `108x8`. Compact preset height, theme/font persistence
  across refresh, reset, custom 700×420 size, pointer drag, fullscreen entry
  and Escape exit, mobile 390×844 fit (no page overflow, 8 × 34 PTY size), and
  a 390×844 → 844×390 orientation change that kept the session connected and
  changed the reported PTY dimensions all passed. No uncaught browser
  exceptions were observed.
- **Phase 14 site audit (headless Chrome):** 25 routes (home, features, docs,
  downloads, about, contact, auth screens, terminal and render fixture,
  desktop routes, and admin routes) at 1920×1080, 1440×900, 1366×768,
  1024×768, 768×1024, 430×932, 390×844, and 360×800 produced no page-level
  horizontal overflow and no uncaught JavaScript exceptions. Admin routes were
  visited in the unauthenticated/`Accounts are not configured` state; signed-in
  admin tables and dialogs were not re-exercised in this pass.
- **Phase 15 build checks:** `npm run typecheck`, `npm test` (61 passed in 9
  files), and `npm run build` all passed after the redesign and terminal
  preference changes.
- **Phase 14 dev-stack checks:** 20 offline `/api` requests through Vite each
  returned `503` JSON with exactly one backend-down warning and zero `proxy
  error` lines; a successful backend request reset the warning, and a second
  outage produced one new warning. A proxied WebSocket reset produced no proxy
  stack trace. `npm run dev` refused to start with a clear message while port
  5173 was occupied, then started the backend first, waited for
  `/api/v1/health`, and only then started Vite (HTTP 200, no proxy errors).
- **Phase 14 offline indicator:** headless Chrome at 390×844 showed the
  persistent “Backend offline” pill with no horizontal overflow, kept it while
  Retry failed, and removed it after a real `kernext-server` came up and Retry
  succeeded.
- **Phase 15 responsive route audit:** nine public routes (`/`, `/features`,
  `/docs`, `/downloads`, `/about`, `/contact`, `/sign-in`, `/create-account`,
  and an unknown route) at 1920×1080, 1440×900, 1280×800, 768×1024, 390×844,
  and 360×800 — 54 loads — produced no page-level horizontal overflow and no
  console errors. The mobile menu opened at 390×844 with all eight links and no
  overflow. Screenshots were inspected at 1440 and 390 in dark mode and at
  1440 in light mode. Code blocks report wider intrinsic widths inside their
  own `overflow-x-auto` containers; the page itself does not scroll
  horizontally.
- **Phase 15 terminal verification:** against a local-mode `kernext-server`
  (accounts disabled only for this test), the PTY transport reported
  `Connected · interactive PTY`, rendered the `kernext ~ $` prompt, and
  executed a typed `echo kernext-live-ok`, which appeared in the buffer. A
  custom 900×420 size changed the frame from 674px / `11 × 81` to 874px /
  `8 × 106`; the settings panel closed after Apply, the output toolbar toggle
  hid and restored the Clear control, and there were no console errors or
  horizontal overflow.
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
- **Signed-in administration UI.** Phase 14 used a server without account
  configuration, so admin routes rendered their configuration notice. Earlier
  phase checks covered signed-in admin pages, but the responsive drawer, data
  tables, and dialogs were not re-exercised with a live administrator session
  in this pass.
- **Account preference sync for the terminal.** Local-storage persistence was
  exercised; the signed-in `user_preferences` sync path was left unchanged but
  not re-tested because no account database was configured in this run.
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
