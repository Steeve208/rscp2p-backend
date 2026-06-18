//! Transak provider facade — implements gateway `FiatOnRampProvider` trait.

use async_trait::async_trait;
use reqwest::Client;
use rust_decimal::Decimal;

use crate::internal::providers::error::ProviderResult;
use crate::internal::providers::models::{FiatProvider, ProviderQuote};
use crate::internal::providers::traits::FiatOnRampProvider;
use crate::internal::providers::transak::config::TransakProviderConfig;
use crate::internal::providers::transak::onramp::OnRampService;

pub struct TransakProvider {
    onramp: OnRampService,
    config: TransakProviderConfig,
}

impl TransakProvider {
    pub fn new(http: Client, config: TransakProviderConfig) -> Self {
        let onramp = OnRampService::new(http, config.clone());
        Self { onramp, config }
    }

    pub fn config(&self) -> &TransakProviderConfig {
        &self.config
    }
}

#[async_trait]
impl FiatOnRampProvider for TransakProvider {
    fn provider(&self) -> FiatProvider {
        FiatProvider::Transak
    }

    fn is_configured(&self) -> bool {
        self.config.api_key.is_some() || self.config.mock_mode
    }

    fn uses_mock(&self) -> bool {
        self.onramp.uses_mock()
    }

    async fn quote(
        &self,
        fiat_currency: &str,
        fiat_amount: Option<Decimal>,
        crypto_asset: &str,
        crypto_chain: &str,
        crypto_amount: Option<Decimal>,
        wallet_address: Option<&str>,
    ) -> ProviderResult<ProviderQuote> {
        self.onramp
            .quote(
                fiat_currency,
                fiat_amount,
                crypto_asset,
                crypto_chain,
                crypto_amount,
                wallet_address,
            )
            .await
            .map_err(Into::into)
    }
}
