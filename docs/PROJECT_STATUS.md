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
- The hero core was, in Phase 15, a layered CSS-3D metallic body with a glass
  highlight, three independently rotating orbits, labeled annotations, and
  pointer-responsive depth. Phase 16 replaces it with a real Three.js scene (see
  below). The CSS `.core-scene` rules remain in `art-direction.css` for any
  non-WebGL surface that still uses them.
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

### Three.js core scene and typography (Phase 16)

- The homepage hero core is now a real Three.js scene
  (`apps/web/src/components/site/orbital-hero.tsx`) instead of a CSS-3D
  composition. It renders a dark metal sphere with meridian seam grooves (so
  rotation is legible), the authentic KERNEXT K emblem as a billboarded plane
  that copies the camera quaternion (so the mark is never distorted or randomly
  rotated), independently rotating elliptical rings — each a faint full torus
  plus a brighter leading arc — travelling particles with additive glow
  sprites, and sprite glows behind the sphere. Phase 17 raises this to four
  orbits and adds the fresnel atmosphere described below.
- Nothing is fetched at runtime: the logo, glow, and equirectangular environment
  textures are generated procedurally on a canvas, and the environment map is
  built from the procedural equirect through `PMREMGenerator`.
- Pointer parallax uses `THREE.MathUtils.damp` for smooth, eased motion. A GSAP
  tween scales the inner group on entrance, and an outer group damps its scale to
  the measured container so the composition stays centred and uncropped at every
  width.
- Lifecycle and cost: `frameloop` is `always` only when motion is allowed, the
  stage is in view, and the tab is visible; otherwise it is `demand`. The PMREM
  environment map is created and disposed inside an effect (StrictMode-safe), and
  React Three Fiber disposes the JSX geometries and materials. The module-level
  logo and glow textures are cached and intentionally never disposed.
- Accessibility: `prefers-reduced-motion: reduce` renders the same composition
  with all animation stopped. When WebGL is unavailable or the scene throws, a
  `Canvas` `fallback` plus a class error boundary render `CoreSceneFallback`
  (`apps/web/src/components/site/core-fallback.tsx`), which imports no 3D code.
- Bundle discipline: the 3D chunk is lazy-loaded with `React.lazy` + `Suspense`,
  so the hero copy paints before the 3D stack arrives. The home route chunk is
  22.2 kB (5.35 kB gzip) and the lazy 3D chunk is 996 kB (272 kB gzip).
- Typography is self-hosted: Manrope (display, variable 200–800), Inter (body,
  variable 100–900), and IBM Plex Mono (400/500 technical labels) live in
  `apps/web/public/fonts` and are wired through `@font-face` in
  `art-direction.css` and the Tailwind theme tokens. Self-hosting is required
  because the server sends `font-src 'self' data:` in its CSP, so no remote font
  host can be used at runtime.
- The homepage gained a documentation band (`DocumentationSection`) that
  links to the real `/docs` sections — quick start, HTTP session API, WebSocket
  transports, security model, and project status. Phase 17 renumbers it to
  `07 / Documentation` after the platform section moved below the hero.
- Two broken in-page anchors were fixed: the footer's `/docs#transports` now
  points at `#websocket`, and the terminal setup notes' `/docs#quick-start` now
  points at `#quickstart`.

### Premium redesign and restored core (Phase 17)

- The core scene was rebuilt around a single dark metal sphere carrying the K
  emblem at its centre, four thin elliptical orbits at different inclinations
  and independent speeds, two to three travelling cyan/blue/violet particles
  per orbit, a restrained fresnel atmosphere shader, and two soft glows behind
  the sphere. The steepest orbit no longer presents as a stray straight line:
  all four read as ellipses at every rotation.
- **Emblem depth fix:** the billboarded emblem now sits at
  `SPHERE_RADIUS + 0.012` rather than inside the sphere volume. At the old
  depth the metal cap occluded the middle of the K mark, so only the outer
  edges could be sampled. The mark is drawn with the same 48-unit geometry as
  the SVG logo and remains upright while the sphere turns.
- The environment map is a procedural equirectangular canvas (cool key, blue
  fill, weakened violet rim) passed through `PMREMGenerator`; no texture or HDR
  asset is fetched. The violet reflection and point light were reduced after
  visual review to remove a smeared highlight on the lower-left of the sphere.
- Reduced motion now applies the container fit immediately instead of easing
  over frames that are never drawn, so the static composition is never
  oversized. Rendering remains `demand` when reduced, off-screen, or the tab is
  hidden; the PMREM texture is disposed on unmount and React Three Fiber owns
  geometry/material disposal. Module-level emblem/glow textures stay cached for
  the document lifetime.
- **New art direction:** the near-black surface palette is replaced by midnight
  navy (`#070c15`) with graphite surfaces, soft-white text, muted blue-gray
  secondary text, a refined electric-cyan primary, a restrained blue-to-violet
  gradient, and low-contrast graphite hairlines. The root type scale is raised
  to 17px, the 10–11px utility floor is lifted, section kickers use larger type
  with less tracking, and footer/nav labels were enlarged. Manrope, Inter, and
  IBM Plex Mono remain the only three faces.
- **Homepage rebuilt:** a full-bleed two-part hero with no bordered stage or
  card; the core floats beside the copy with hairline annotations for
  `shell-core`, `shell-runtime`, and the PTY layer. The headline is the
  requested “The shell, re-engineered from the core.” with a non-breaking
  hyphen so it cannot split at the hyphen. Sections are now: surfaces,
  platforms, engine, capability matrix (six implemented features), architecture
  flow, terminal workspace, documentation index, honest downloads band, about,
  contact, and the closing wordmark band. No fabricated release files are
  shown; downloads link only to the repository's releases and build docs.
- The navigation cluster lost its bordered pill and active pill background in
  favour of spacing and a thin cyan active rule. Sticky blur remains.
- Secondary routes inherit the token, type, and footer changes through the same
  `art-direction.css` layer. Auth forms retain their existing labels, inline
  validation, error states, and password visibility control; they were checked
  visually at desktop and mobile widths rather than restructured.
- The homepage now renders 12 sections; no previous section was removed to
  fit the new layout.

### Typographic reset, larger core, and navigation systems (Phase 18)

- **Display type** is now Space Grotesk, self-hosted as a 22.3 kB variable woff2
  at `apps/web/public/fonts/space-grotesk-latin-wght-normal.woff2` and declared
  in `art-direction.css`. Inter stays the body face and IBM Plex Mono stays
  restricted to terminal output, technical labels, and code. No remote font host
  was added. `--font-display` and the Tailwind display token both point at the
  new face, so headings on every route changed together.
- The palette was lifted without becoming light: midnight blue `#08111b`
  background, `#0d1826` surfaces, `#13202f` raised surfaces, soft-white
  `#eaf2fb` text, muted `#9aacc1`, `#5edcec` primary, `#6f9df7` blue, and
  `#a28ff8` violet. Annotation text gained a background-matching text-shadow so
  orbit lines cannot pass through the labels.
- **The core is larger and rounder.** `SPHERE_RADIUS = 1.42`, with the camera at
  6.8 units and 32° FOV, an emblem plane of 1.5 at `SPHERE_RADIUS + 0.014`, and a
  fixed axial tilt of `[0.14, 0, 0.12]`. `FIT_EXTENT = 4.9` includes the orbit
  radius, so the whole assembly still fits the measured stage at every width.
  Orbit radii are 1.66 / 1.88 / 2.06 / 2.22 with raised opacities, so all four
  rings stay visible around the larger body.
- The metal surface now uses a procedural 256px brushed-roughness texture
  (generated on canvas, cached for the document lifetime) with metalness 1,
  roughness 0.46, clearcoat 1, and `envMapIntensity` 1.45. The variation keeps
  the sphere from reading as flat black plastic under the procedural
  environment map while adding no network asset.
- **Homepage:** a new benefits section (`#benefits`, index `06 / Why KERNEXT`)
  sits between the architecture and terminal sections; the later sections were
  renumbered to 07–11. The page remains 12 sections and the core stays in the
  hero rather than being repeated further down.
- **Documentation:** a real sidebar (`DOC_NAV` / `DocsNav`) lists the six page
  sections with an `IntersectionObserver` scroll-spy. `PageShell` gained an
  optional `sidebar` slot so the grid is shared rather than reimplemented; below
  900px the sidebar becomes a horizontally scrollable chip row above the
  content.
- **Features:** a sticky category index links to each feature group. The index
  scrolls horizontally on narrow screens instead of wrapping into the content.
- **Route transitions:** `SiteLayout` and `TerminalLayout` wrap the outlet in an
  opacity-only Motion transition keyed by pathname. There is deliberately no
  exit animation, so router scroll restoration and immediate navigation are not
  delayed.
- **Contact:** the page explains that KERNEXT has no contact-form endpoint and
  provides the repository links instead of a form that would discard messages;
  no fake success state exists.
- **Auth and admin shells:** the sign-in shell dropped its backdrop blur for a
  solid raised surface, replaced the legacy cyan glow with the current primary,
  and reduced the 10px/0.28em account kicker to 11px/0.14em. The admin modal
  scrim moved from near-black `#02040a` to `#040a12`. Existing form labels,
  validation, error/loading states, and admin logic were not changed.
- **Mobile overflow repair:** the docs sidebar initially let long code samples
  widen the page because grid children default to `min-width: auto`. The content
  column, sidebar column, and `.docs-nav` now set `min-width: 0`, so code blocks
  and the chip row scroll inside their own containers.

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
- **Phase 16 build checks:** `npm run typecheck`, `npm test` (61 passed in 9
  files), and `npm run build` all passed after the Three.js hero, the
  self-hosted fonts, and the documentation band. The extracted stylesheet is
  145.8 kB (26.3 kB gzip); `home-*.js` is 22.2 kB (5.35 kB gzip) and
  `orbital-hero-*.js` is 996 kB (272 kB gzip).
- **Phase 16 core-scene verification (headless Chrome, production build via
  `vite preview`, WebGL via SwiftShader, `prefers-reduced-motion` emulated both
  ways):**
  - WebGL context creation succeeded and a `<canvas>` of 559×531 px filled the
    hero stage.
  - Hiding the canvas changed 38.8% of the hero region's pixels, so the scene —
    not the CSS stage background — paints the composition. Its bounding box was
    456×428 px inside the 608×579 px stage: centred (bright centroid 284,281 vs
    a stage centre of 304,289) and not cropped.
  - Two frames 800 ms apart differed across 1.05% of pixels, proving the sphere,
    ring arcs, and travelling particles animate.
  - With `prefers-reduced-motion: reduce`, the same 800 ms interval produced
    **0** changed pixels while the static scene still rendered (43.9% of the
    region), confirming motion stops without losing the composition.
  - No console errors, no uncaught exceptions, and no failed requests. The only
    console output was a WebGL performance notice caused by the test's own
    `drawImage` pixel readback.
  - Computed fonts resolved to `Manrope` for headings, `Inter` for body text,
    and `IBM Plex Mono` for the mono readout; the four font files were served
    with HTTP 200 from the app origin.
- **Phase 16 route and anchor audit (headless Chrome, production build):**
  - 28 routes (home, features, docs, downloads, about, contact, sign-in,
    account creation, forgot/reset password, email verification, account,
    terminal, terminal render fixture, five admin routes, eight desktop routes,
    and an unknown route) at 1440×1000 and 390×844 — 56 loads — produced no
    console errors, no failed non-API requests, no horizontal overflow, and
    rendered content on every route.
  - The homepage documentation band renders five entries whose `href`s match the
    real `/docs` section ids, and the anchor targets on `/docs`, `/downloads`,
    and `/` all resolve.
- **Phase 16 terminal E2E against the real backend:** a local-mode
  `kernext-server` (no `DATABASE_URL`; accounts disabled only for this check)
  serving the production bundle directly, with a genuine PTY:
  `node scripts/e2e-browser.mjs http://127.0.0.1:7999/terminal` reported
  **12/12** checks — no failed requests, no uncaught errors, terminal surface
  rendered, workspace connected, typed command output visible, PTY mode and
  dimensions in the footer, engine mode connected with visible output, and a
  real running session with its session directory. Note for future runs: the
  server rejects WebSocket upgrades whose `Origin` is not in `ALLOWED_ORIGINS`
  (or `KERNEXT_ALLOWED_ORIGINS`) with `401`, so a direct-serve check must use
  port 5173, 4173, or 7878, or add its own origin.
- **Phase 16 account-mode check:** against the already-running account-mode
  server on 7878, `/terminal` correctly rendered the sign-in gate
  (“Terminal sessions run against this machine, so they are bound to your
  KERNEXT account”) with no HTTP 500 and no console errors — the designed
  behaviour when `DATABASE_URL` is set.
- **Phase 17 build checks:** `npx tsc --noEmit`, `npm test` (61 passed in 9
  files), and `npm run build` all passed after the hero, homepage, and design
  system changes. The extracted stylesheet is 157.46 kB (27.86 kB gzip);
  `home-*.js` is 28.27 kB (7.12 kB gzip) and the lazy `orbital-hero-*.js` is
  997.27 kB (272.74 kB gzip).
- **Phase 17 visual review:** Chrome headless against the production build
  (`vite preview` on 4173, WebGL through SwiftShader). Desktop screenshots were
  inspected at 1440px for the hero, surfaces, platforms, capabilities, engine,
  architecture, documentation, downloads, about, contact, CTA, and footer;
  mobile screenshots were inspected at 390×844 for the hero, rail, and platform
  band. The hero renders the sphere, upright K emblem, four orbital rings, and
  particles without cropping; the K mark was confirmed visible after the depth
  fix. Section kickers, footer labels, and platform cards were corrected after
  that review (narrow-screen state stacking, larger label type, reduced
  tracking).
- **Phase 17 reduced motion:** with `prefers-reduced-motion: reduce` emulated
  over CDP, the hero copy and stage computed to `opacity: 1` and the static
  core rendered; the same build animated with `no-preference`. Reduced motion
  therefore does not hide content or leave the composition unfitted.
- **Phase 17 WebGL fallback:** launching Chrome with `--disable-webgl
  --disable-webgl2` against the production build rendered
  `CoreSceneFallback` (`orbital-fallback`) in the hero instead of an empty
  canvas. The Three.js scene was not requested from a browser without WebGL.
- **Phase 18 build checks:** `npm run build` (which runs `tsc --noEmit`,
  `vite build`, the index build, and SEO generation) exited 0, and `npm test`
  reported **61 passed** in 9 files. The extracted stylesheet is 163.37 kB
  (28.81 kB gzip), `home-*.js` is 31.22 kB (7.70 kB gzip), and the lazy
  `orbital-hero-*.js` is 997.99 kB (272.94 kB gzip).
- **Phase 18 console check:** loading `/` and scrolling the page produced one
  warning — React Three Fiber's internal `THREE.Clock` deprecation — with no
  uncaught exceptions, React key warnings, GSAP errors, or failed requests.
- **Phase 18 client-side navigation:** clicking the real nav anchors moved the
  router through `/features`, `/docs`, `/downloads`, `/about`, `/contact`, and
  `/`; each destination's pathname and `h1` matched the expected page, with zero
  runtime exceptions during the transitions.
- **Phase 18 responsive scan:** 13 routes (`/`, `/features`, `/docs`,
  `/downloads`, `/about`, `/contact`, `/sign-in`, `/create-account`,
  `/forgot-password`, `/verify-email`, `/terminal`, `/admin`, and an unknown
  route) at 320, 375, 425, 768, 1024, 1440, and 1920px — **91 loads** — produced
  no page-level horizontal overflow at a fixed layout viewport. The docs route
  was rechecked at 390px with mobile emulation both on and off: scroll width,
  client width, and visual viewport all measured 390.
- **Phase 18 reduced motion:** with `prefers-reduced-motion: reduce` emulated,
  two hero captures 900 ms apart were byte-identical while the static sphere,
  emblem, and rings still rendered. The scene therefore stops without losing
  the composition.
- **Phase 18 WebGL fallback:** Chrome launched with `--disable-webgl
  --disable-webgl2` rendered `orbital-fallback` in the hero instead of an empty
  canvas.
- **Phase 18 admin gate:** `/admin` redirected to `/sign-in`, which rendered the
  `Welcome back.` heading and a form. No signed-in admin table or authenticated
  account form was exercised in this pass.
- **Phase 18 auth visual check:** `/sign-in` was inspected at 1440×1000 and
  390×844 after the shell change. The card, fields, password visibility control,
  primary action, and footer link all fit with no horizontal overflow and no
  runtime console errors.
- **Phase 17 console check:** loading `/` and scrolling the full page produced
  one console warning — React Three Fiber's internal `THREE.Clock`
  deprecation notice — with no React key warnings, uncaught exceptions, GSAP
  lifecycle errors, or failed asset requests. The warning is emitted inside
  `@react-three/fiber`, not by KERNEXT code.
- **Phase 17 responsive route scan:** 12 routes (`/`, `/features`, `/docs`,
  `/downloads`, `/about`, `/contact`, `/sign-in`, `/create-account`,
  `/forgot-password`, `/verify-email`, `/terminal`, and an unknown route) at
  320, 390, 768, 1024, and 1440px — 60 loads — produced no page-level
  horizontal overflow. Auth screens were inspected at 1440 and 390px; with no
  backend in preview they show the expected unreachable-server notice rather
  than a broken form.
- **Not re-run in this pass:** the Rust workspace suite, the real-PTY terminal
  E2E, and signed-in account/admin flows. The changes are confined to
  `apps/web` and no Rust source, API contract, or authentication handler was
  modified. The existing Phase 16 backend results above still describe the
  engine, and should be re-run when a backend and database are available.
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
- The hero core needs WebGL. Without it the static `CoreSceneFallback` renders
  instead, so the homepage never depends on a GPU. The 3D chunk is ~272 kB gzip
  and is loaded lazily after the hero copy, so it does not block first paint.
- The three typefaces are self-hosted from `apps/web/public/fonts` because the
  server CSP forbids a remote font host. Adding a fourth family means adding the
  files and an `@font-face` block; it cannot be pulled from Google Fonts at
  runtime.

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
