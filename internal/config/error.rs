//! Configuration error types.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("missing required environment variable: {0}")]
    MissingVar(&'static str),
    #[error("invalid {field}: {message}")]
    Invalid { field: &'static str, message: String },
}
