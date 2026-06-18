use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorkerError {
    #[error("redis error: {0}")]
    Redis(#[from] redis::RedisError),
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("job processing failed: {0}")]
    Processing(String),
    #[error("unknown job kind: {0}")]
    UnknownKind(String),
}

pub type WorkerResult<T> = Result<T, WorkerError>;
