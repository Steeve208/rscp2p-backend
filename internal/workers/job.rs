use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

/// Redis queue names.
pub mod queues {
    pub const DEPOSIT: &str = "deposit";
    pub const WITHDRAWAL: &str = "withdrawal";
}

/// Job kind identifiers — used by the processor to dispatch handlers.
pub mod kinds {
    pub const DEPOSIT_PROCESS_HEAD: &str = "deposit.process_head";
    pub const DEPOSIT_RECORD: &str = "deposit.record";
    pub const WITHDRAWAL_CONFIRM: &str = "withdrawal.confirm";
}

/// A unit of work in the retry queue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerJob {
    pub id: String,
    pub queue: String,
    pub kind: String,
    pub payload: Value,
    pub attempts: u32,
    pub max_attempts: u32,
    pub created_at: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

impl WorkerJob {
    pub fn new(queue: &str, kind: &str, payload: Value, max_attempts: u32) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            queue: queue.to_string(),
            kind: kind.to_string(),
            payload,
            attempts: 0,
            max_attempts,
            created_at: chrono::Utc::now().timestamp(),
            last_error: None,
        }
    }
}
