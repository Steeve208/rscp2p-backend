//! Redis connection for cache, sessions, and rate limiting.

use redis::aio::ConnectionManager;
use redis::Client;
use thiserror::Error;
use tracing::info;

#[derive(Debug, Error)]
pub enum RedisError {
    #[error("failed to connect to Redis: {0}")]
    Connection(#[from] redis::RedisError),
}

pub async fn connect(redis_url: &str) -> Result<ConnectionManager, RedisError> {
    let client = Client::open(redis_url)?;
    let manager = ConnectionManager::new(client).await?;

    let mut conn = manager.clone();
    let _: String = redis::cmd("PING").query_async(&mut conn).await?;

    info!("Redis connected successfully");

    Ok(manager)
}

pub async fn ping(manager: &ConnectionManager) -> bool {
    let mut conn = manager.clone();
    let result: Result<String, _> = redis::cmd("PING").query_async(&mut conn).await;
    result.is_ok()
}
