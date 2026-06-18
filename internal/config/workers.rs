//! Background worker configuration — retry, backoff, polling intervals.

use crate::internal::config::{env, error::ConfigError};

#[derive(Debug, Clone)]
pub struct WorkerConfig {
    /// Max retry attempts before dead-lettering (default: `5`).
    pub retry_max_attempts: u32,
    /// Base delay for exponential backoff in ms (default: `1000`).
    pub retry_base_ms: u64,
    /// Cap on backoff delay in ms (default: `300000` = 5 min).
    pub retry_max_ms: u64,
    /// How often the retry-queue processor polls Redis (default: `5` s).
    pub queue_poll_interval_secs: u64,
    /// Withdrawal confirmation poll interval (default: `15` s).
    pub withdrawal_poll_interval_secs: u64,
    /// Enable withdrawal confirmation worker (default: `true`).
    pub withdrawal_worker_enabled: bool,
    /// Persist dead-lettered jobs to PostgreSQL (default: `true`).
    pub dlq_audit_enabled: bool,
}

impl WorkerConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            retry_max_attempts: env::u32("WORKER_RETRY_MAX_ATTEMPTS", "5")?,
            retry_base_ms: env::u64("WORKER_RETRY_BASE_MS", "1000")?,
            retry_max_ms: env::u64("WORKER_RETRY_MAX_MS", "300000")?,
            queue_poll_interval_secs: env::u64("WORKER_QUEUE_POLL_INTERVAL_SECS", "5")?,
            withdrawal_poll_interval_secs: env::u64("WORKER_WITHDRAWAL_POLL_INTERVAL_SECS", "15")?,
            withdrawal_worker_enabled: env::bool("WORKER_WITHDRAWAL_ENABLED", true)?,
            dlq_audit_enabled: env::bool("WORKER_DLQ_AUDIT", true)?,
        })
    }
}
