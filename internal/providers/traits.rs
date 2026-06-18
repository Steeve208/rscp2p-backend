use async_trait::async_trait;

use crate::internal::providers::error::ProviderResult;
use crate::internal::providers::models::{FiatProvider, ProviderQuote};

#[async_trait]
pub trait FiatOnRampProvider: Send + Sync {
    fn provider(&self) -> FiatProvider;
    fn is_configured(&self) -> bool;
    fn uses_mock(&self) -> bool;

    async fn quote(
        &self,
        fiat_currency: &str,
        fiat_amount: Option<rust_decimal::Decimal>,
        crypto_asset: &str,
        crypto_chain: &str,
        crypto_amount: Option<rust_decimal::Decimal>,
        wallet_address: Option<&str>,
    ) -> ProviderResult<ProviderQuote>;
}
