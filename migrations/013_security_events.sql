-- Migration 013: security_events
--
-- Audit table for fraud assessments, IP blocks, and security decisions.
-- Separate from auth_audit_events (which covers login/session lifecycle).

CREATE TABLE IF NOT EXISTS security_events (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID        REFERENCES users(id) ON DELETE SET NULL,
    event_type      VARCHAR(64) NOT NULL,          -- e.g. 'fraud_assessment', 'ip_blocked'
    ip              VARCHAR(64),
    user_agent      TEXT,
    fraud_score     SMALLINT    CHECK (fraud_score BETWEEN 0 AND 100),
    fraud_decision  VARCHAR(32),                   -- allow | monitor | challenge_mfa | block
    fraud_signals   JSONB       NOT NULL DEFAULT '[]',
    metadata        JSONB       NOT NULL DEFAULT '{}',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_security_events_user_id
    ON security_events (user_id)
    WHERE user_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_security_events_ip
    ON security_events (ip)
    WHERE ip IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_security_events_created_at
    ON security_events (created_at DESC);

CREATE INDEX IF NOT EXISTS idx_security_events_decision
    ON security_events (fraud_decision)
    WHERE fraud_decision IS NOT NULL;

-- For SIEM/alerting: quickly find high-score events
CREATE INDEX IF NOT EXISTS idx_security_events_high_score
    ON security_events (fraud_score DESC, created_at DESC)
    WHERE fraud_score >= 60;
