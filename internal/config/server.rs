//! Server / HTTP configuration: bind address, timeouts, CORS, rate-limiting.

use crate::internal::config::{env, error::ConfigError};

#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Bind host (default: `0.0.0.0`).
    pub host: String,
    /// Bind port (default: `8080`).
    pub port: u16,
    /// Allowed CORS origins (default: `["*"]`).
    pub allowed_origins: Vec<String>,
    /// HTTP handler timeout in seconds (default: `30`).
    pub request_timeout_secs: u64,
    /// Per-IP requests/second limit — `0` disables rate limiting (default: `100`).
    pub rate_limit_per_second: u64,
}

impl ServerConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            host: env::with_default("HOST", "0.0.0.0"),
            port: env::u16("PORT", "8080")?,
            allowed_origins: env::list("ALLOWED_ORIGINS", "*"),
            request_timeout_secs: env::u64("REQUEST_TIMEOUT_SECS", "30")?,
            rate_limit_per_second: env::u64("RATE_LIMIT_PER_SECOND", "100")?,
        })
    }

    pub fn listen_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
