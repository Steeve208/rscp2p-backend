//! Centralized configuration — the **only** entry point for environment variables.
//!
//! ```text
//! Frontend → routes/ → internal/services → DB | Redis | Blockchain | Providers
//!              ↑ no env access
//! ```
//!
//! Bootstrap: `cmd/main.rs` loads `.env` (dev only) → [`Config::load`] → [`AppState`].
//!
//! `std::env::var` is called from [`env`] and provider `from_env()` helpers invoked
//! only inside [`Config::load`]. Secret policy: [`secrets`].
//!
//! | Sub-module | Owns |
//! |------------|------|
//! | [`env`] | Raw env-reading primitives |
//! | [`error`] | `ConfigError` |
//! | [`server`] | Host, port, timeouts, CORS, rate-limit |
//! | [`database`] | PostgreSQL URL, pool size; Redis URL |
//! | [`auth`] | JWT signing keys, MFA, session limits |
//! | [`blockchain`] | RSC node, confirmations, deposit worker |
//! | [`features`] | Runtime feature flags |

pub mod auth;
pub mod blockchain;
pub mod database;
pub mod env;
pub mod error;
pub mod features;
pub mod observability;
pub mod secrets;
pub mod server;
pub mod threat_intel;
pub mod workers;

pub use auth::{AuthConfig, JwtConfig};
pub use blockchain::BlockchainConfig;
pub use database::{DatabaseConfig, RedisConfig};
pub use error::ConfigError;
pub use features::FeatureFlags;
pub use observability::ObservabilityConfig;
pub use server::ServerConfig;
pub use threat_intel::ThreatIntelConfig;
pub use workers::WorkerConfig;

// Provider configs re-exported for code that imports via `internal::config::*`
pub use crate::internal::providers::striga::StrigaProviderConfig;
pub use crate::internal::providers::transak::TransakProviderConfig;

// ─────────────────────────────────────────────────────────────────────────────
// Types that don't yet have their own sub-module (shared with swaps)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Development,
    Staging,
    Production,
}

impl Environment {
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_lowercase().as_str() {
            "development" | "dev" => Some(Self::Development),
            "staging" | "stage" => Some(Self::Staging),
            "production" | "prod" => Some(Self::Production),
            _ => None,
        }
    }

    pub fn is_production(self) -> bool {
        matches!(self, Self::Production)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Development => "development",
            Self::Staging => "staging",
            Self::Production => "production",
        }
    }
}

/// Venue type for swap liquidity sources (kept in config to avoid circular deps with swaps).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapVenueKind {
    Dex,
    Cex,
}

impl SwapVenueKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dex => "dex",
            Self::Cex => "cex",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SwapProviderSettings {
    pub id: String,
    pub venue_kind: SwapVenueKind,
    pub api_key: Option<String>,
    pub api_base_url: Option<String>,
    pub mock_mode: bool,
}

#[derive(Debug, Clone)]
pub struct ProvidersConfig {
    pub striga: StrigaProviderConfig,
    pub transak: TransakProviderConfig,
    /// When `true` (default when no API keys), use deterministic mock quotes.
    pub fiat_mock_mode: bool,
}

// ─────────────────────────────────────────────────────────────────────────────
// Root config
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Config {
    pub environment: Environment,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub jwt: JwtConfig,
    pub auth: AuthConfig,
    pub blockchain: BlockchainConfig,
    pub providers: ProvidersConfig,
    pub swaps: crate::internal::swaps::config::SwapsConfig,
    pub features: FeatureFlags,
    pub observability: ObservabilityConfig,
    pub workers: WorkerConfig,
    pub threat_intel: ThreatIntelConfig,
}

impl Config {
    pub fn load() -> Result<Self, ConfigError> {
        let environment = env::optional("ENVIRONMENT")
            .and_then(|v| Environment::parse(&v))
            .unwrap_or(Environment::Development);

        let server = ServerConfig::from_env()?;
        let database = DatabaseConfig::from_env()?;
        let redis = RedisConfig::from_env()?;
        let jwt = JwtConfig::from_env()?;
        let auth = AuthConfig::from_env()?;
        let blockchain = BlockchainConfig::from_env()?;
        let features = FeatureFlags::from_env()?;
        let observability = ObservabilityConfig::from_env()?;
        let workers = WorkerConfig::from_env()?;
        let threat_intel = ThreatIntelConfig::from_env()?;

        let transak_key = env::optional("TRANSAK_API_KEY");
        let fiat_mock_mode = env::bool("FIAT_MOCK_MODE", transak_key.is_none())?;

        let providers = ProvidersConfig {
            striga: StrigaProviderConfig::from_env(),
            transak: TransakProviderConfig::from_env(fiat_mock_mode),
            fiat_mock_mode,
        };

        let swaps = crate::internal::swaps::config::SwapsConfig::from_env()?;

        Ok(Self {
            environment,
            server,
            database,
            redis,
            jwt,
            auth,
            blockchain,
            providers,
            swaps,
            features,
            observability,
            workers,
            threat_intel,
        })
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.jwt.secret.is_empty() {
            return Err(ConfigError::Invalid {
                field: "JWT_SECRET",
                message: "cannot be empty".into(),
            });
        }
        if self.environment.is_production() {
            if self.jwt.secret.len() < 32 {
                return Err(ConfigError::Invalid {
                    field: "JWT_SECRET",
                    message: "must be at least 32 characters in production".into(),
                });
            }
            if self.auth.mfa_encryption_key.len() < 32 {
                return Err(ConfigError::Invalid {
                    field: "MFA_ENCRYPTION_KEY",
                    message: "must be at least 32 characters in production".into(),
                });
            }
        }
        if self.blockchain.rsc_rpc_url.is_empty() {
            return Err(ConfigError::Invalid {
                field: "RSC_RPC_URL",
                message: "cannot be empty".into(),
            });
        }
        secrets::validate_production(self)?;
        Ok(())
    }

    // ── Convenience delegators (backwards-compat for existing callsites) ─────

    pub fn listen_addr(&self) -> String {
        self.server.listen_addr()
    }

    /// Shortcut used widely — avoids `config.database.url`.
    pub fn database_url(&self) -> &str {
        &self.database.url
    }

    pub fn db_max_connections(&self) -> u32 {
        self.database.max_connections
    }

    pub fn redis_url(&self) -> &str {
        &self.redis.url
    }

    pub fn request_timeout_secs(&self) -> u64 {
        self.server.request_timeout_secs
    }

    pub fn rate_limit_per_second(&self) -> u64 {
        self.server.rate_limit_per_second
    }

    pub fn allowed_origins(&self) -> &[String] {
        &self.server.allowed_origins
    }
}
