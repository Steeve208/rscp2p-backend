-- Migration 014: user roles
--
-- Adds a single `role` column to users.
-- Multi-role support (many-to-many) can be layered later via a `user_roles` table.
-- Allowed values mirror the `UserRole` enum in `internal/users/models/mod.rs`.

ALTER TABLE users
    ADD COLUMN IF NOT EXISTS role VARCHAR(32) NOT NULL DEFAULT 'user'
        CHECK (role IN ('user', 'support', 'fraud_analyst', 'admin', 'system'));

-- Index for admin/support dashboards that filter by role
CREATE INDEX IF NOT EXISTS idx_users_role
    ON users (role)
    WHERE role <> 'user';

-- Explicit bootstrap: ensure the clearing system user gets the system role.
-- (The clearing wallet user is created at startup by wallets::ensure_clearing_wallet.)
UPDATE users
SET role = 'system'
WHERE email = 'settlement-clearing@system.rsc.internal'
  AND role = 'user';
