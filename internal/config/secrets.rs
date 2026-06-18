//! Secret environment variables — classification and production guards.
//!
//! Values are read from the process environment (`.env` in dev, K8s/DO secrets in prod).
//! They flow: `cmd/main.rs` → [`super::Config::load`] → [`super::AppState`] → services.
//!
//! **Never** log these keys' values. **Never** commit `.env` to git.

use super::{Config, ConfigError};

/// Env var names that must be treated as secrets (redaction, audits, secret managers).
pub const SECRET_ENV_KEYS: &[&str] = &[
    "DATABASE_URL",
    "REDIS_URL",
    "JWT_SECRET",
    "JWT_SECRET_PREVIOUS",
    "MFA_ENCRYPTION_KEY",
    "TRANSAK_API_KEY",
    "TRANSAK_SECRET",
    "TRANSAK_WEBHOOK_SECRET",
    "STRIGA_API_KEY",
    "STRIGA_API_SECRET",
    "STRIGA_UI_SECRET",
    "STRIGA_WEBHOOK_SECRET",
];

const PLACEHOLDER_MARKERS: &[&str] = &[
    "change-me",
    "changeme",
    "dev-secret",
    "replace-me",
    "your-",
    "example",
];

/// Extra production checks for secret strength and provider readiness.
pub fn validate_production(config: &Config) -> Result<(), ConfigError> {
    if !config.environment.is_production() {
        return Ok(());
    }

    reject_placeholder(&config.jwt.secret, "JWT_SECRET")?;
    reject_placeholder(&config.auth.mfa_encryption_key, "MFA_ENCRYPTION_KEY")?;

    if config.features.fiat_on_ramp_enabled && !config.providers.fiat_mock_mode {
        if config.providers.transak.api_key.is_none() {
            return Err(ConfigError::Invalid {
                field: "TRANSAK_API_KEY",
                message: "required when FIAT_MOCK_MODE is false".into(),
            });
        }
        if config.providers.transak.webhook_secret.is_none() {
            return Err(ConfigError::Invalid {
                field: "TRANSAK_WEBHOOK_SECRET",
                message: "required in production when TRANSAK_API_KEY is set".into(),
            });
        }
        if config.providers.striga.is_configured()
            && config.providers.striga.webhook_secret.is_none()
        {
            return Err(ConfigError::Invalid {
                field: "STRIGA_WEBHOOK_SECRET",
                message: "required in production when Striga API credentials are set".into(),
            });
        }
    }

    Ok(())
}

/// Returns true if a log field name should be redacted.
pub fn is_secret_env_key(key: &str) -> bool {
    if SECRET_ENV_KEYS.contains(&key) {
        return true;
    }
    key.ends_with("_API_KEY") || key.ends_with("_WEBHOOK_SECRET") || key.contains("SECRET")
}

fn reject_placeholder(value: &str, field: &'static str) -> Result<(), ConfigError> {
    let lower = value.to_lowercase();
    if PLACEHOLDER_MARKERS.iter().any(|m| lower.contains(m)) {
        return Err(ConfigError::Invalid {
            field,
            message: "placeholder value not allowed in production".into(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_secret_keys() {
        assert!(is_secret_env_key("JWT_SECRET"));
        assert!(is_secret_env_key("SWAP_PROVIDER_FOO_API_KEY"));
        assert!(!is_secret_env_key("PORT"));
    }
}
