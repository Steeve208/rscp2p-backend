use crate::internal::config::{SwapProviderSettings, SwapVenueKind};
use crate::internal::swaps::models::LiquidityVenueKind;

#[derive(Debug, Clone)]
pub struct ProviderRuntimeConfig {
    pub settings: SwapProviderSettings,
}

impl ProviderRuntimeConfig {
    pub fn id(&self) -> &str {
        &self.settings.id
    }

    pub fn venue_kind(&self) -> LiquidityVenueKind {
        match self.settings.venue_kind {
            SwapVenueKind::Dex => LiquidityVenueKind::Dex,
            SwapVenueKind::Cex => LiquidityVenueKind::Cex,
        }
    }

    pub fn uses_mock(&self) -> bool {
        self.settings.mock_mode
    }

    pub fn is_configured(&self) -> bool {
        self.settings.api_key.is_some() || self.settings.mock_mode
    }
}
