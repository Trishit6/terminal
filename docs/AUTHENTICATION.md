# KERNEXT Authentication

Accounts are optional. When `DATABASE_URL` is set, the web server runs in
account mode and protects account data and remote terminal sessions. The CLI
and the desktop shell keep working offline without an account.

## Registration and login

`POST /api/auth/register` accepts `full_name`, `email`, `password`, and
`accept_terms`. Server-side validation:

- name 2–120 characters, no control characters
- normalized email (trimmed, lowercase, 5–320 characters, one `@`, dotted domain)
- password 10–128 characters with at least one letter and one digit
- duplicate email returns `409`

Passwords are hashed with **Argon2id** (`argon2` defaults). Unknown emails still
run a dummy Argon2 verification so login timing does not reveal account
existence. `POST /api/auth/login` returns a generic
`the email address or password is incorrect` for any failure and is rate
limited per IP + email (10 attempts / 15 minutes).

Successful registration and login create a server-side session and set two
cookies:

| Cookie | Attributes |
| --- | --- |
| `kernext_session` | `HttpOnly`, `SameSite=Strict`, `Path=/`, `Max-Age`, `Secure` in production |
| `kernext_csrf` | readable by the SPA, `SameSite=Strict`, `Path=/`, `Max-Age`, `Secure` in production |

Session lifetimes: 12 hours by default, 30 days with “remember me”.

## Logout and session management

`POST /api/auth/logout` revokes the current session and clears both cookies.
Revoked sessions cannot resolve again. `GET /api/auth/me` returns the safe
public profile (`id`, `full_name`, `email`, `email_verified`, `created_at`,
`account_status`), preferences, the session expiry, and the CSRF token; password
hashes and raw tokens are never returned.

`GET /api/auth/sessions` lists only the caller’s live sessions with a `current`
flag. `DELETE /api/auth/sessions/{id}` revokes an owned session; another user’s
session id returns `404`. `POST /api/auth/change-password` verifies the current
password and, by default, keeps the current session while revoking the others.

## CSRF

State-changing account requests (`PATCH /api/auth/profile`,
`PUT /api/auth/preferences`, `POST /api/auth/change-password`,
`POST /api/auth/resend-verification`, `DELETE /api/auth/sessions/{id}`) require
the `x-csrf-token` header. Its SHA-256 hash must match the hash stored with the
session. Missing or mismatched tokens return `403 csrf_failed`. Login,
registration, logout, and the public recovery endpoints are exempt; logout is
idempotent and only ends a session.

## Password recovery

`POST /api/auth/forgot-password` always returns the same generic message. When
the account exists, a 256-bit random token is created, only its SHA-256 hash is
stored, and it expires after one hour and works once. Creating a new token marks
older unused tokens used.

`POST /api/auth/reset-password` consumes the token, replaces the Argon2id hash,
and revokes **every** session for the account. Reusing the token returns `400`.

Delivery:

- With SMTP configured (`SMTP_HOST`, `SMTP_PORT`, `SMTP_USERNAME`,
  `SMTP_PASSWORD`, `SMTP_FROM`) the link is emailed via `lettre`. A delivery
  failure is reported as `email_delivery: "failed"`; the API never claims an
  email was sent when it was not.
- With `KERNEXT_DEV_MAIL=1` and `KERNEXT_ENV` not `production`, the response
  includes `dev_reset_token` for development only.
- Otherwise the response says delivery is not configured.

## Email verification

Registration creates a 24-hour single-use verification token (hash stored).
`POST /api/auth/verify-email` consumes it and sets `email_verified_at`.
`POST /api/auth/resend-verification` issues a new token for the signed-in
account and reports `already_verified` when nothing is needed. Changing the
email clears verification and issues a new token. Accounts are **never**
auto-verified in production; without SMTP and without `KERNEXT_DEV_MAIL`, the
verification link cannot be delivered.

Links use `APP_PUBLIC_URL` (default `http://127.0.0.1:7878`):
`{APP_PUBLIC_URL}/reset-password?token=…` and
`{APP_PUBLIC_URL}/verify-email?token=…`.

## Roles and administrator accounts

`users.role` is explicit (`user` or `admin`) and is only assigned by the seed or
the database — never by registration or profile updates. Profile updates ignore
any `role` or `account_status` fields, so a signed-in user cannot promote
themselves.

`GET /api/admin/users` requires an authenticated account whose role is `admin`
and returns safe account summaries (no hashes, no tokens). Normal users receive
`403 forbidden`.

### Development seed

```env
KERNEXT_SEED_ADMIN=true
KERNEXT_ADMIN_USERNAME=admin
KERNEXT_ADMIN_EMAIL=admin@admin.com
KERNEXT_ADMIN_PASSWORD=password
```

Development credentials: `admin@admin.com` / `password`, role `admin`.

The seed runs once at startup, is idempotent, hashes the password with Argon2id,
and sets `must_change_password = 1`. An existing email is never overwritten.
It is ignored when `KERNEXT_ENV=production`, so these credentials can never be
created automatically in production. Change the password on first sign-in.

### Forced password change

While `must_change_password` is true, account and terminal features return
`403 password_change_required`. `GET /api/auth/me`, logout, and
`POST /api/auth/change-password` remain available. Changing the password clears
the flag; the account page shows an explicit banner and links to the Security
section.

The web terminal also reads `must_change_password` from the session profile.
It renders a "Change your initial password first" gate with a link to the
account page instead of opening a PTY or engine WebSocket that the server would
reject with `403`. This is the expected resolution for a browser console
message such as `WebSocket connection to '/api/v1/pty' failed: 403`; the
endpoint is protected, not broken.

## Terminal authorization

In account mode every terminal endpoint resolves the session cookie:

- Creating, listing, reading, executing, deleting, and WebSocket upgrades all
  require a valid account session.
- Sessions carry an owner. `GET`/`DELETE`/`execute`/WebSocket on another user’s
  session return `404` (not `403`) so session presence is not disclosed.
- Per-account caps: 4 engine sessions and 2 pseudoterminals.
- Session sweeper removes engine sessions idle for 30 minutes; the database
  sweeper purges expired/revoked login sessions every 60 seconds.
- The PTY relay runs the real `kernext` binary and tears it down when the
  socket closes.

In local mode (`DATABASE_URL` unset) the terminal keeps the original loopback
behavior and optional bearer token.

## Frontend

- `apps/web/src/lib/auth.ts` owns `AuthProvider`, cookie-based requests
  (`credentials: 'include'`), the CSRF header, and session refresh.
- Pages: sign in, create account, forgot password, reset password, verify
  email, sign out, account dashboard (profile, sessions, password, preferences).
- The web terminal route is account-gated only when the server reports accounts
  enabled.
- Tokens are never stored in `localStorage`; the session lives in an HttpOnly
  cookie and the CSRF token in its readable companion cookie.

## Desktop

`apps/desktop/src-tauri/src/account.rs` proxies account calls through Rust and
keeps the session cookie in the OS credential store (Keychain, Secret Service,
or Windows Credential Manager). The webview only sees public profile data.
Local terminals remain available offline and do not require sign-in.

## Configuration summary

| Variable | Effect |
| --- | --- |
| `DATABASE_URL` | Enables account mode |
| `KERNEXT_ENV=production` | Enables secure cookies and HSTS; disables dev mail |
| `KERNEXT_SECURE_COOKIES=1` | Forces `Secure` cookies outside production |
| `KERNEXT_DEV_MAIL=1` | Returns development-only recovery/verification tokens |
| `APP_PUBLIC_URL` | Base URL for emailed links |
| `KERNEXT_ALLOWED_ORIGINS` | Comma-separated browser origin allowlist |
| `KERNEXT_ENV_FILE` | Explicit env-file path (defaults to `./.env`, then the workspace `.env`) |
| `KERNEXT_SEED_ADMIN` | Opt-in development administrator seed |
| `KERNEXT_ADMIN_USERNAME` / `_EMAIL` / `_PASSWORD` | Development seed values |
| `SMTP_*` | SMTP delivery settings |

See `docs/DATABASE.md` for schema and migrations and
`docs/BACKEND_ARCHITECTURE.md` for routes and middleware.
