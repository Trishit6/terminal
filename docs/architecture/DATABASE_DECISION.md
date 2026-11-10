# KERNECT Database Architecture Decision

## Summary

No database is currently required for the core shell functionality. The shell's state is ephemeral (in-memory session, job table, history) and intentionally does not persist full command output or active process handles. A database is only justified for web-platform user-facing features (preferences, saved workspaces, session records) while the desktop must remain fully offline-capable.

## Evaluation of Options

### PostgreSQL — Web Platform

**Justification:** Required if the web surface needs durable user data: saved workspaces, persistent terminal preferences, user profiles, or long-term session records. The web API already has endpoints for session management, and adding PostgreSQL would enable `CREATE TABLE users`, `user_preferences`, `saved_workspaces`, and `terminal_session_records` tables.

**Potential entities:**

**users**
- id
- email
- password_hash or external identity reference
- display_name
- created_at
- updated_at

**user_preferences**
- id
- user_id
- theme
- terminal_preferences
- default_working_directory
- created_at
- updated_at

**saved_workspaces**
- id
- user_id
- name
- configuration
- created_at
- updated_at

**terminal_session_records**
- id
- user_id
- session_reference
- created_at
- closed_at
- status

**Only persist the fields that are genuinely needed.** Do not store plaintext passwords, authentication tokens, or unnecessary sensitive terminal data. Do not persist full command output by default.

**Decision:** Consider PostgreSQL if and when the web surface requires user accounts, saved workspaces, or persistent preferences. Do not implement prematurely.

### Active Terminal Sessions

**Active PTY processes must be managed by a runtime session manager.**

They are not database records that should be executed from PostgreSQL. Keep active process handles, streams, cancellation tokens, and runtime state in the appropriate process/session layer. Persist only necessary durable metadata.

### SQLite — Desktop Application

**Evaluation:** SQLite is appropriate for the desktop application to store:

- Local settings (theme, terminal preferences, font size/family, cursor style)
- Local command history
- Recently used directories
- Local workspace metadata

**The desktop must continue to work without a remote database connection.** If existing configuration files (TOML in the app config directory) are sufficient, do not introduce SQLite merely for architectural appearance.

### Redis — Optional Future Infrastructure

**Do not add Redis unless there is a concrete requirement for:**
- Multiple Axum instances needing shared session coordination
- Distributed event delivery
- Distributed rate limiting

**Keep the first working deployment as simple as possible.**

## Decision

**Do not introduce a database for Phase 8.** The core KERNECT shell operates without persistent storage. Sessions are managed in-memory by the Rust runtime. The web surface can optionally add PostgreSQL later when user accounts, saved workspaces, or persistent preferences are genuinely required. The desktop uses TOML configuration files in the app config directory and remains fully offline-capable.

**Rationale:**
- The shell's primary mode of operation is ephemeral command execution
- Active PTY processes and engine sessions are managed by runtime process handles, not SQL queries
- The web terminal already works without a database (e2e tests pass)
- The desktop uses TOML config files and works without any database
- Adding a database would add complexity, build dependencies, and migration requirements without delivering immediate user value
- If/when the web surface needs user accounts or saved workspaces, PostgreSQL with SQLx migrations can be added at that time

## Implementation Path (if PostgreSQL is later justified)

If the web surface later requires PostgreSQL:

- Implement migrations using `sqlx` or `diesel`
- Use environment-based connection configuration (`DATABASE_URL`)
- Use connection pooling (`r2d2` or `sqlx`'s built-in pool)
- Use parameterized queries throughout (never string interpolation)
- Add indexes and foreign keys where query patterns warrant
- Implement transaction handling with proper error rollback
- Add migration and rollback documentation
- Provide a `.env.example` without real secrets
- Provide a local development setup (Docker or local PostgreSQL)
- Add integration tests for session persistence and user operations

## Current State

- **No database tables exist** in the project
- **No migration files exist**
- **No database connection configuration exists**
- **All shell state is in-memory** (session, job table, history)
- **Configuration files** use TOML in the platform config directory
- **E2E tests pass** without any database dependency