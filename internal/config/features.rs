//! Runtime feature flags.
//!
//! Flip features without redeploying by setting env vars:
//!
//! ```env
//! FEATURE_FIAT_ON_RAMP=true
//! FEATURE_SWAPS=true
//! FEATURE_FRAUD_DETECTION=true
//! FEATURE_MAINTENANCE_MODE=false
//! FEATURE_MFA_REQUIRED=false
//! ```
//!
//! Check flags at the handler/service level:
//!
//! ```rust,ignore
//! if !state.config.features.swaps_enabled {
//!     return Err(AppError::FeatureDisabled("swaps"));
//! }
//! ```

use crate::internal::config::{env, error::ConfigError};

#[derive(Debug, Clone)]
pub struct FeatureFlags {
    /// Enable fiat on-ramp flows (Transak). Default: `true`.
    pub fiat_on_ramp_enabled: bool,
    /// Enable the swaps engine. Default: `true`.
    pub swaps_enabled: bool,
    /// Enable fraud detection engine on sensitive operations. Default: `true`.
    pub fraud_detection_enabled: bool,
    /// Maintenance mode — returns 503 on all non-health routes. Default: `false`.
    pub maintenance_mode: bool,
    /// Require MFA for all users before any payment action. Default: `false`.
    pub mfa_required: bool,
}

impl FeatureFlags {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            fiat_on_ramp_enabled: env::bool("FEATURE_FIAT_ON_RAMP", true)?,
            swaps_enabled: env::bool("FEATURE_SWAPS", true)?,
            fraud_detection_enabled: env::bool("FEATURE_FRAUD_DETECTION", true)?,
            maintenance_mode: env::bool("FEATURE_MAINTENANCE_MODE", false)?,
            mfa_required: env::bool("FEATURE_MFA_REQUIRED", false)?,
        })
    }

    /// Returns `true` if the app is in a state that should serve traffic normally.
    pub fn is_operational(&self) -> bool {
        !self.maintenance_mode
    }
}
