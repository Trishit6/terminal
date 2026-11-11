# KERNEXT Backend Architecture

KERNEXT has one Rust engine and three surfaces. The backend in
`apps/shell-server` is a local Axum service that owns real
`shell_runtime::Session` instances and routes every command through
`shell_builtins::Dispatcher`. Accounts and persistence live beside that engine;
they never become a dependency of it.

```text
browser / desktop webview
  -> apps/web (React, TanStack Router)
  -> HTTP + WebSocket (/api/...)
  -> apps/shell-server (Axum, Tokio)
       -> shell_api contracts
       -> shell_builtins Dispatcher + pipeline
       -> shell_runtime Session
       -> shell-server::db (SQLx -> MariaDB)
```

## Crate boundaries

| Component | Responsibility |
| --- | --- |
| `apps/shell-server` | HTTP/WebSocket surface, account routes, terminal registry |
| `crates/shell-api` | Serializable request/response contracts shared with the desktop |
| `crates/shell-builtins` | Lexer/parser evaluation, built-ins, pipelines, external processes |
| `crates/shell-runtime` | Session state: cwd, variables, functions, jobs |
| `apps/web` | Website, account pages, terminal renderer |
| `apps/desktop/src-tauri` | Tauri IPC commands and local runtime; never routes through the public server |

## Server modes

`apps/shell-server/src/lib.rs` selects one of two modes at startup:

- **Account mode** — `DATABASE_URL` is set. The server connects to MariaDB,
  applies migrations, and requires a valid account session cookie for every
  terminal endpoint. Each terminal session is bound to its owner.
- **Local mode** — `DATABASE_URL` is unset. The server keeps its loopback-only
  behavior with an optional `KERNEXT_SERVER_TOKEN` bearer token. Account
  routes answer `503` with a clear message. The CLI and desktop shell are not
  affected.

The CLI never reads `DATABASE_URL`; local shell execution stays independent of
database availability.

## Module map

| File | Role |
| --- | --- |
| `src/main.rs` | Loads `.env` via `dotenvy`, initializes tracing, calls `run()` |
| `src/lib.rs` | Startup, DB connect + migrate, idle sweeper, graceful shutdown |
| `src/routes.rs` | REST sessions, health, CORS, security headers |
| `src/auth.rs` | Origin allowlist, bearer-token checks, constant-time compare |
| `src/auth_routes.rs` | Register/login/logout/me/recovery/verification/sessions |
| `src/db.rs` | SQLx pool, Argon2id, token hashing, rate limiter, SMTP mailer |
| `src/state.rs` | Session registry, ownership, PTY slots, account context |
| `src/ws.rs` | Engine and PTY WebSocket transports |
| `src/spa.rs` | Static SPA serving with per-route metadata |
| `src/error.rs` | Stable JSON error shape |

## API surface

Sessions (account-scoped when accounts are enabled):

```text
GET    /api/v1/health
GET    /api/v1/health/database
GET    /api/v1/sessions
POST   /api/v1/sessions
GET    /api/v1/sessions/{id}
DELETE /api/v1/sessions/{id}
POST   /api/v1/sessions/{id}/execute
GET    /api/v1/sessions/{id}/ws        (engine WebSocket)
GET    /api/v1/pty                     (real pseudoterminal WebSocket)
```

Accounts:

```text
GET    /api/auth/status
POST   /api/auth/register
POST   /api/auth/login
POST   /api/auth/logout
GET    /api/auth/me
PATCH  /api/auth/profile
PUT    /api/auth/preferences
POST   /api/auth/forgot-password
POST   /api/auth/reset-password
POST   /api/auth/verify-email
POST   /api/auth/resend-verification
POST   /api/auth/change-password
GET    /api/auth/sessions
DELETE /api/auth/sessions/{id}
```

Errors use one shape (`ApiErrorBody`):

```json
{ "code": "unauthorized", "message": "sign in to access this resource" }
```

Status codes: `400` validation, `401` unauthenticated, `403` CSRF failure,
`404` missing or not-owned resource, `409` duplicate email/session limit,
`429` rate limited, `503` database unavailable.

## Request security

- Origin allowlist (`KERNEXT_ALLOWED_ORIGINS`, default local Vite/Tauri origins).
- Bearer token only in local mode (`KERNEXT_SERVER_TOKEN`, constant-time compare).
- Account cookies are `HttpOnly`, `SameSite=Strict`, `Secure` in production.
- CSRF double-submit: the `kernext_csrf` cookie value must match the
  `x-csrf-token` header and the session's stored hash for state-changing requests.
- Fixed-window rate limits on register, login, forgot, reset, and verify.
- Security headers on every response: `X-Content-Type-Options: nosniff`,
  `X-Frame-Options: DENY`, `Referrer-Policy: no-referrer`, a `Permissions-Policy`,
  a restrictive `Content-Security-Policy`, and HSTS when
  `KERNEXT_ENV=production`.
- CORS is restricted to the allowlist and enables credentials only for it.

## Database health

`GET /api/v1/health/database` runs `SELECT 1` through the pool and returns
`{ "status": "ok", "database": "mariadb", "latency_ms": N }`. Without
`DATABASE_URL` it returns `503` with the same JSON error shape; the terminal
and CLI keep working in local mode.

## Verification

- Unit and API tests: `cargo test -p shell-server`.
- MariaDB integration tests: `cargo test -p shell-server --test auth_db`
  with `KERNEXT_TEST_DATABASE_URL` pointing at a disposable database.
- CI runs the MariaDB suite against a `mariadb:11.4` service container.
- Web: `npm run typecheck`, `npm test`, `npm run build`.
