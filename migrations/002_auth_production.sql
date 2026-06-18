-- MFA
ALTER TABLE users
    ADD COLUMN IF NOT EXISTS mfa_enabled BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS mfa_secret TEXT,
    ADD COLUMN IF NOT EXISTS mfa_pending_secret TEXT;

-- Auth audit trail (compliance / SIEM)
CREATE TABLE IF NOT EXISTS auth_audit_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users (id) ON DELETE SET NULL,
    event_type VARCHAR(64) NOT NULL,
    success BOOLEAN NOT NULL,
    ip VARCHAR(45),
    user_agent TEXT,
    metadata JSONB NOT NULL DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_auth_audit_user_created
    ON auth_audit_events (user_id, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_auth_audit_type_created
    ON auth_audit_events (event_type, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_auth_audit_created
    ON auth_audit_events (created_at DESC);
