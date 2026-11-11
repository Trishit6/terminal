-- KERNEXT administration console schema.
--
-- Additive and idempotent: existing accounts, sessions, and tokens are left
-- untouched. The audit table deliberately has no foreign keys to `users`, so
-- a deleted account cannot cascade away its administrative history and an
-- audit row never keeps a deleted account alive.

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS last_login_at DATETIME(6) NULL;

CREATE TABLE IF NOT EXISTS admin_audit_log (
    id CHAR(36) NOT NULL,
    event_type VARCHAR(64) NOT NULL,
    actor_id CHAR(36) NULL,
    actor_email VARCHAR(320) NULL,
    target_user_id CHAR(36) NULL,
    target_label VARCHAR(320) NULL,
    metadata JSON NULL,
    ip_address VARCHAR(64) NULL,
    created_at DATETIME(6) NOT NULL,
    PRIMARY KEY (id),
    KEY idx_audit_created (created_at),
    KEY idx_audit_event (event_type),
    KEY idx_audit_event_created (event_type, created_at),
    KEY idx_audit_actor (actor_id),
    KEY idx_audit_target (target_user_id)
) ENGINE = InnoDB
  DEFAULT CHARSET = utf8mb4
  COLLATE = utf8mb4_unicode_ci;

-- Search, filtering, and dashboard indexes. `idx_users_status` already exists
-- from the authentication migration; the rest cover the admin queries.
CREATE INDEX IF NOT EXISTS idx_users_role ON users (role);
CREATE INDEX IF NOT EXISTS idx_users_created ON users (created_at);
CREATE INDEX IF NOT EXISTS idx_users_last_login ON users (last_login_at);
CREATE INDEX IF NOT EXISTS idx_user_sessions_activity ON user_sessions (user_id, last_used_at);
CREATE INDEX IF NOT EXISTS idx_user_sessions_last_used ON user_sessions (last_used_at);
