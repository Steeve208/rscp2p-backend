-- Dead-letter audit trail for background worker jobs that exhausted retries.

CREATE TABLE IF NOT EXISTS worker_dead_letters (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_id VARCHAR(128) NOT NULL,
    queue_name VARCHAR(64) NOT NULL,
    job_kind VARCHAR(128) NOT NULL,
    payload JSONB NOT NULL,
    attempts INT NOT NULL,
    last_error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_worker_dlq_queue_created
    ON worker_dead_letters (queue_name, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_worker_dlq_kind_created
    ON worker_dead_letters (job_kind, created_at DESC);
