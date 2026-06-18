//! Transak-specific configuration — env via [`crate::internal::config::env`].

use crate::internal::config::env;

pub const WIDGET_URL_PRODUCTION: &str = "https://global.transak.com";
pub const WIDGET_URL_STAGING: &str = "https://global-stg.transak.com";
pub const API_URL_PRODUCTION: &str = "https://api.transak.com";
pub const API_URL_STAGING: &str = "https://api-stg.transak.com";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransakEnvironment {
    Staging,
    Production,
}

impl TransakEnvironment {
    pub fn parse(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "production" | "prod" => Self::Production,
            _ => Self::Staging,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TransakProviderConfig {
    pub api_key: Option<String>,
    pub secret: Option<String>,
    pub environment: TransakEnvironment,
    pub api_base_url: Option<String>,
    pub mock_mode: bool,
    pub webhook_secret: Option<String>,
}

impl TransakProviderConfig {
    pub fn from_env(fiat_mock_default: bool) -> Self {
        let api_key = env::optional("TRANSAK_API_KEY");
        let mock_mode = env::optional("TRANSAK_MOCK_MODE")
            .and_then(|v| v.parse().ok())
            .unwrap_or(fiat_mock_default || api_key.is_none());

        let environment = env::optional("TRANSAK_ENVIRONMENT")
            .map(|v| TransakEnvironment::parse(&v))
            .unwrap_or(TransakEnvironment::Staging);

        Self {
            api_key,
            secret: env::optional("TRANSAK_SECRET"),
            environment,
            api_base_url: env::optional("TRANSAK_API_BASE_URL"),
            mock_mode,
            webhook_secret: env::optional("TRANSAK_WEBHOOK_SECRET"),
        }
    }

    pub fn widget_base_url(&self) -> &str {
        if self.mock_mode {
            return WIDGET_URL_STAGING;
        }
        match self.environment {
            TransakEnvironment::Production => WIDGET_URL_PRODUCTION,
            TransakEnvironment::Staging => WIDGET_URL_STAGING,
        }
    }

    pub fn default_api_base_url(&self) -> &str {
        match self.environment {
            TransakEnvironment::Production => API_URL_PRODUCTION,
            TransakEnvironment::Staging => API_URL_STAGING,
        }
    }

    pub fn is_configured(&self) -> bool {
        !self.mock_mode && self.api_key.is_some()
    }
}
