-- Administrator role and forced password change.
--
-- Additive and idempotent: existing accounts become `user`, keep working, and
-- are never modified by this migration. MariaDB supports IF NOT EXISTS for
-- ADD COLUMN, so re-running is safe.

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS role VARCHAR(16) NOT NULL DEFAULT 'user';

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS must_change_password TINYINT(1) NOT NULL DEFAULT 0;
