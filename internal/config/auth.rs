//! Authentication and JWT configuration.

use crate::internal::config::{env, error::ConfigError};

#[derive(Debug, Clone)]
pub struct JwtConfig {
    /// Primary signing secret (`JWT_SECRET`).
    pub secret: String,
    /// Previous secret for zero-downtime rotation (`JWT_SECRET_PREVIOUS`).
    pub secret_previous: Option<String>,
    /// Key ID for the current secret (`JWT_KID_CURRENT`, default: `"v1"`).
    pub kid_current: String,
    /// Key ID for the previous secret (`JWT_KID_PREVIOUS`).
    pub kid_previous: Option<String>,
    /// Access token TTL in hours (default: `24`).
    pub expiry_hours: u64,
    /// Refresh token TTL in days (default: `7`).
    pub refresh_expiry_days: u64,
    /// MFA challenge token TTL in minutes (default: `5`).
    pub mfa_challenge_minutes: u64,
}

impl JwtConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            secret: env::required("JWT_SECRET")?,
            secret_previous: env::optional("JWT_SECRET_PREVIOUS"),
            kid_current: env::with_default("JWT_KID_CURRENT", "v1"),
            kid_previous: env::optional("JWT_KID_PREVIOUS"),
            expiry_hours: env::u64("JWT_EXPIRY_HOURS", "24")?,
            refresh_expiry_days: env::u64("JWT_REFRESH_EXPIRY_DAYS", "7")?,
            mfa_challenge_minutes: env::u64("MFA_CHALLENGE_MINUTES", "5")?,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// Max failed login attempts before lockout (default: `5`).
    pub login_max_attempts: u32,
    /// Lockout window in seconds (default: `900` = 15 min).
    pub login_window_secs: u64,
    /// Max concurrent sessions per user (default: `10`).
    pub max_sessions_per_user: u32,
    /// AES-256 key for MFA secrets at rest — falls back to `JWT_SECRET`.
    pub mfa_encryption_key: String,
    /// TOTP issuer name shown in authenticator apps (default: `"RSC Gateway"`).
    pub mfa_issuer: String,
    /// IP addresses trusted as reverse proxies for `X-Forwarded-For`.
    pub trusted_proxies: Vec<String>,
}

impl AuthConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            login_max_attempts: env::u32("AUTH_LOGIN_MAX_ATTEMPTS", "5")?,
            login_window_secs: env::u64("AUTH_LOGIN_WINDOW_SECS", "900")?,
            max_sessions_per_user: env::u32("AUTH_MAX_SESSIONS_PER_USER", "10")?,
            mfa_encryption_key: env::required_or_fallback("MFA_ENCRYPTION_KEY", "JWT_SECRET")?,
            mfa_issuer: env::with_default("MFA_ISSUER", "RSC Gateway"),
            trusted_proxies: env::list("TRUSTED_PROXIES", ""),
        })
    }
}
