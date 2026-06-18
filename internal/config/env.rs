//! Environment variable reading primitives.
//!
//! This is the **only** module in `internal/config/` that calls `std::env::var`.
//! All other config sub-modules call these helpers instead.

use std::env;

use crate::internal::config::error::ConfigError;

/// Read a required env var. Fails with [`ConfigError::MissingVar`] if absent or empty.
pub fn required(key: &'static str) -> Result<String, ConfigError> {
    env::var(key)
        .ok()
        .filter(|v| !v.is_empty())
        .ok_or(ConfigError::MissingVar(key))
}

/// Read an optional env var. Returns `None` if absent or empty.
pub fn optional(key: &str) -> Option<String> {
    env::var(key).ok().filter(|v| !v.is_empty())
}

/// Read with a static fallback default.
pub fn with_default(key: &str, default: &str) -> String {
    env::var(key)
        .unwrap_or_else(|_| default.into())
}

/// Read + parse as `u16`. Returns [`ConfigError::Invalid`] on bad value.
pub fn u16(key: &'static str, default: &str) -> Result<::std::primitive::u16, ConfigError> {
    with_default(key, default)
        .parse::<::std::primitive::u16>()
        .map_err(|_| ConfigError::Invalid { field: key, message: "must be a valid port (u16)".into() })
}

/// Read + parse as `u32` with a default.
pub fn u32(key: &'static str, default: &str) -> Result<::std::primitive::u32, ConfigError> {
    with_default(key, default)
        .parse::<::std::primitive::u32>()
        .map_err(|_| ConfigError::Invalid { field: key, message: "must be a positive integer".into() })
}

/// Read + parse as `u64` with a default.
pub fn u64(key: &'static str, default: &str) -> Result<::std::primitive::u64, ConfigError> {
    with_default(key, default)
        .parse::<::std::primitive::u64>()
        .map_err(|_| ConfigError::Invalid { field: key, message: "must be a positive integer".into() })
}

/// Read + parse as `bool` with an explicit default.
pub fn bool(key: &'static str, default: bool) -> Result<::std::primitive::bool, ConfigError> {
    match env::var(key) {
        Ok(v) => v.parse::<::std::primitive::bool>().map_err(|_| ConfigError::Invalid {
            field: key,
            message: "must be true or false".into(),
        }),
        Err(_) => Ok(default),
    }
}

/// Read a comma-separated list, trimming whitespace, discarding empty segments.
pub fn list(key: &str, default: &str) -> Vec<String> {
    with_default(key, default)
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Read an optional `u64` — `None` if absent or empty, error if present but unparseable.
pub fn optional_u64(key: &'static str) -> Result<Option<::std::primitive::u64>, ConfigError> {
    match optional(key) {
        None => Ok(None),
        Some(v) => v.parse::<::std::primitive::u64>()
            .map(Some)
            .map_err(|_| ConfigError::Invalid {
                field: key,
                message: "must be a valid u64".into(),
            }),
    }
}

/// Read a required var with a fallback to a second key (e.g. `MFA_ENCRYPTION_KEY || JWT_SECRET`).
pub fn required_or_fallback(key: &'static str, fallback: &'static str) -> Result<String, ConfigError> {
    env::var(key)
        .ok()
        .filter(|v| !v.is_empty())
        .or_else(|| env::var(fallback).ok().filter(|v| !v.is_empty()))
        .ok_or(ConfigError::MissingVar(key))
}
