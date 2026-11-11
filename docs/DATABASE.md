# KERNEXT Database

KERNEXT accounts are stored in MariaDB (MySQL protocol) through SQLx. The
database is optional: without `DATABASE_URL` the CLI, desktop shell, and local
web terminal keep working, and account endpoints report `503`.

## Configuration

```env
DATABASE_URL=mysql://kernext:CHANGE_ME@127.0.0.1:3307/kernext
```

- Development uses MariaDB on port **3307** with a dedicated `kernext` database
  and `kernext` user. Do not use a shared `root` account for the server.
- `.env` lives at the workspace root (gitignored). Real environment variables
  always win over the file. `.env.example` contains placeholders only.
- The backend loads configuration deterministically: `KERNEXT_ENV_FILE` when
  set, then `./.env`, then the workspace-root `../../.env` relative to the
  server crate. Launching from another directory therefore still finds the
  development database configuration.
- `KERNEXT_ENV=production` requires `DATABASE_URL`; startup fails with a clear
  error instead of silently running without accounts. Non-`mysql://` URLs are
  rejected before connecting.
- Never hardcode credentials or commit `.env`.

## Development startup

The workspace-root `.env` is already expected at this point. From the
workspace root, the exact sequence is:

```sh
npm ci          # install the web and desktop workspaces from package-lock.json
npm run dev     # Axum on 127.0.0.1:7878 + Vite on 127.0.0.1:5173
```

Open <http://127.0.0.1:5173/sign-in>. Vite proxies `/api` and WebSocket
upgrades to the Axum server, so the browser never needs a database address.
To run the services separately, use `cargo run -p shell-server` and
`npm run web:dev`. If port 5173 is already occupied, stop the existing dev
server first; `npm run dev` intentionally stops the stack when either half
fails.

For a fresh local database, the dedicated development account (never `root`)
can be created once with:

```sql
CREATE DATABASE IF NOT EXISTS kernext
  CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;
CREATE USER IF NOT EXISTS 'kernext'@'127.0.0.1'
  IDENTIFIED BY 'CHANGE_ME';
GRANT ALL PRIVILEGES ON kernext.* TO 'kernext'@'127.0.0.1';
FLUSH PRIVILEGES;
```

Migrations run automatically when the server starts; do not create or alter
tables by hand.

## Pool and startup

`apps/shell-server/src/db.rs` opens a `MySqlPool`:

- max 10 connections, min 1
- 5-second acquire timeout
- connectivity is validated at startup; a failed connection aborts server
  startup with an `unavailable` error
- migrations run at startup via `sqlx::migrate!("./migrations")`
- `verify_schema()` checks that all five account tables exist after migrating;
  a missing table is a startup error
- a 60-second background task purges expired/revoked sessions

If the database becomes unreachable after startup, account requests return
`503` and local shell features continue.

## Migrations

Migrations live in `apps/shell-server/migrations/` and are embedded in the
binary. They are applied automatically on startup and are idempotent
(`CREATE TABLE IF NOT EXISTS`, additive only — no table is dropped).

Current schema (`20261111000001_auth.sql`, `20261111000002_admin_roles.sql`):

| Table | Purpose |
| --- | --- |
| `users` | id, full_name, email (unique), password_hash, email_verified_at, account_status, role (`user`/`admin`), must_change_password, timestamps |
| `user_sessions` | id, user_id FK, session_token_hash (unique), csrf_token_hash, user_agent, ip_address, expires_at, last_used_at, revoked_at |
| `password_reset_tokens` | id, user_id FK, token_hash (unique), expires_at, used_at, created_at |
| `email_verification_tokens` | id, user_id FK, token_hash (unique), expires_at, used_at, created_at |
| `user_preferences` | id, user_id FK (unique), theme, terminal_preferences JSON, timestamps |

Foreign keys use `ON DELETE CASCADE`; all timestamps are UTC `DATETIME(6)`.
Only hashes are stored for passwords (Argon2id) and tokens (SHA-256).

### Manual application

Migrations run automatically. To apply them manually:

```sh
cargo run -p shell-server   # connects, validates, and migrates
```

or execute the SQL file with a MariaDB client against the configured database.
There is no separate migration binary.

### Recovery

Migrations are additive and idempotent, so re-running is safe. To recover a
development database, drop the `kernext` **development** database and restart
the server; migrations recreate the schema. Never drop a production database
without a backup. The schema intentionally stores no terminal output or command
history.

## Development administrator seed

Set in the workspace `.env`:

```env
KERNEXT_SEED_ADMIN=true
KERNEXT_ADMIN_USERNAME=admin
KERNEXT_ADMIN_EMAIL=admin@admin.com
KERNEXT_ADMIN_PASSWORD=password
```

On startup the server inserts the account with an Argon2id hash, role `admin`,
and `must_change_password = 1`. The seed is idempotent: if the email already
exists, the password, role, and status are left untouched. It is ignored when
`KERNEXT_ENV=production`. Credentials are never logged.

## Test database

Run the account integration suite against a disposable database:

```sh
KERNEXT_TEST_DATABASE_URL=mysql://kernext:kernext-ci-password@127.0.0.1:3307/kernext_test \
  cargo test -p shell-server --test auth_db -- --nocapture
```

Safety rules enforced by the test harness:

- `KERNEXT_TEST_DATABASE_URL` takes precedence.
- A `DATABASE_URL` fallback is used only when the database name contains
  `test`.
- Tests never create or drop databases; they apply migrations and insert
  uniquely named test users.

CI (`.github/workflows/ci.yml`) starts a `mariadb:11.4` service container on
host port 3307 and runs this suite with temporary test-only credentials.

## Query safety

All SQL uses SQLx parameter binding (`?` placeholders). Dynamic table names are
limited to the two token tables selected by the `TokenTable` enum — never by
request input. Session and token lookups are by hash, not by raw secret.
