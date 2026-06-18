//! Striga-specific configuration — env via [`crate::internal::config::env`].

use crate::internal::config::env;

pub const DEFAULT_SANDBOX_BASE_URL: &str = "https://www.sandbox.striga.com/api/v1";

#[derive(Debug, Clone)]
pub struct StrigaProviderConfig {
    pub app_id: Option<String>,
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
    pub ui_secret: Option<String>,
    pub base_url: String,
    pub mock_mode: bool,
    pub webhook_secret: Option<String>,
}

impl StrigaProviderConfig {
    pub fn from_env() -> Self {
        let api_key = env::optional("STRIGA_API_KEY");
        let api_secret = env::optional("STRIGA_API_SECRET");
        let mock_mode = env::optional("STRIGA_MOCK_MODE")
            .and_then(|v| v.parse().ok())
            .unwrap_or(api_key.is_none() || api_secret.is_none());

        Self {
            app_id: env::optional("STRIGA_APP_ID"),
            api_key,
            api_secret,
            ui_secret: env::optional("STRIGA_UI_SECRET"),
            base_url: env::optional("STRIGA_BASE_URL")
                .unwrap_or_else(|| DEFAULT_SANDBOX_BASE_URL.to_string()),
            mock_mode,
            webhook_secret: env::optional("STRIGA_WEBHOOK_SECRET"),
        }
    }

    pub fn is_configured(&self) -> bool {
        !self.mock_mode && self.api_key.is_some() && self.api_secret.is_some()
    }
}
