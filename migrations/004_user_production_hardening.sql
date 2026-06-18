-- Production hardening for users module
-- 1. Optimistic locking
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS version BIGINT NOT NULL DEFAULT 1;

-- 2. Account deletion lifecycle (GDPR / right to be forgotten)
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS deletion_requested_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS deletion_scheduled_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS anonymized_at TIMESTAMPTZ;

-- 3. User audit trail (separate from auth audit for clarity)
CREATE TABLE IF NOT EXISTS user_audit_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users (id) ON DELETE SET NULL,
    actor_user_id UUID REFERENCES users (id) ON DELETE SET NULL, -- who performed the action (self or admin)
    event_type VARCHAR(64) NOT NULL,
    success BOOLEAN NOT NULL DEFAULT TRUE,
    ip VARCHAR(45),
    user_agent TEXT,
    old_values JSONB,
    new_values JSONB,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_user_audit_user_created
    ON user_audit_events (user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_user_audit_type_created
    ON user_audit_events (event_type, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_user_audit_actor
    ON user_audit_events (actor_user_id, created_at DESC);

-- Helpful index for scheduled deletion jobs
CREATE INDEX IF NOT EXISTS idx_users_deletion_scheduled
    ON users (deletion_scheduled_at)
    WHERE deletion_scheduled_at IS NOT NULL;

-- Bump version on every update (simple trigger)
CREATE OR REPLACE FUNCTION bump_user_version()
RETURNS TRIGGER AS $$
BEGIN
    NEW.version = OLD.version + 1;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_users_version ON users;
CREATE TRIGGER trg_users_version
BEFORE UPDATE ON users
FOR EACH ROW
EXECUTE FUNCTION bump_user_version();
