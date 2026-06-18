//! PostgreSQL + Redis connection configuration.

use crate::internal::config::{env, error::ConfigError};

#[derive(Debug, Clone)]
pub struct DatabaseConfig {
    /// PostgreSQL connection URL (`DATABASE_URL`).
    pub url: String,
    /// Maximum pool connections (default: `10`).
    pub max_connections: u32,
}

impl DatabaseConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            url: env::required("DATABASE_URL")?,
            max_connections: env::u32("DB_MAX_CONNECTIONS", "10")?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct RedisConfig {
    /// Redis connection URL (`REDIS_URL`).
    pub url: String,
}

impl RedisConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            url: env::required("REDIS_URL")?,
        })
    }
}
