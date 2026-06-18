//! On-ramp: price quotes and checkout URL generation.

use reqwest::Client;
use rust_decimal::Decimal;

use crate::internal::providers::models::ProviderQuote;
use crate::internal::providers::transak::api::TransakApiClient;
use crate::internal::providers::transak::config::TransakProviderConfig;
use crate::internal::providers::transak::error::{TransakError, TransakResult};
use crate::internal::providers::transak::models::OnRampQuote;

#[derive(Clone)]
pub struct OnRampService {
    api: TransakApiClient,
}

impl OnRampService {
    pub fn new(http: Client, config: TransakProviderConfig) -> Self {
        Self {
            api: TransakApiClient::new(http, config),
        }
    }

    pub fn uses_mock(&self) -> bool {
        self.api.uses_mock()
    }

    pub async fn quote(
        &self,
        fiat_currency: &str,
        fiat_amount: Option<Decimal>,
        crypto_asset: &str,
        crypto_chain: &str,
        crypto_amount: Option<Decimal>,
        wallet_address: Option<&str>,
    ) -> TransakResult<ProviderQuote> {
        let fiat_currency = fiat_currency.trim().to_uppercase();
        let crypto_asset = crypto_asset.trim().to_uppercase();
        let crypto_chain = crypto_chain.trim().to_lowercase();

        let (fiat_amount, crypto_amount) = resolve_amounts(fiat_amount, crypto_amount)?;
        let mock = self.uses_mock();

        let on_ramp = if mock {
            mock_quote(
                &fiat_currency,
                fiat_amount,
                &crypto_asset,
                &crypto_chain,
                crypto_amount,
                wallet_address,
            )
        } else {
            let network = TransakApiClient::map_chain(&crypto_chain);
            let mut q = self
                .api
                .price_quote(&fiat_currency, &crypto_asset, &network, fiat_amount)
                .await?;
            q.crypto_chain = crypto_chain;
            q
        };

        Ok(to_provider_quote(on_ramp, mock))
    }
}

fn resolve_amounts(
    fiat_amount: Option<Decimal>,
    crypto_amount: Option<Decimal>,
) -> TransakResult<(Decimal, Decimal)> {
    match (fiat_amount, crypto_amount) {
        (Some(f), _) if f > Decimal::ZERO => {
            let c = crypto_amount
                .filter(|v| *v > Decimal::ZERO)
                .unwrap_or_else(|| f / Decimal::new(96, 2));
            Ok((f, c))
        }
        (_, Some(c)) if c > Decimal::ZERO => {
            let f = fiat_amount
                .filter(|v| *v > Decimal::ZERO)
                .unwrap_or_else(|| c * Decimal::new(104, 2));
            Ok((f, c))
        }
        _ => Err(TransakError::Validation(
            "fiat_amount or crypto_amount required".into(),
        )),
    }
}

fn mock_quote(
    fiat_currency: &str,
    fiat_amount: Decimal,
    crypto_asset: &str,
    crypto_chain: &str,
    crypto_amount: Decimal,
    wallet_address: Option<&str>,
) -> OnRampQuote {
    let rate = if crypto_amount > Decimal::ZERO {
        fiat_amount / crypto_amount
    } else {
        Decimal::ONE
    };
    let network = TransakApiClient::map_chain(crypto_chain);
    let mut checkout = format!(
        "https://global.transak.com/?apiKey=mock&cryptoCurrencyCode={crypto_asset}&fiatCurrency={fiat_currency}&fiatAmount={fiat_amount}&network={network}"
    );
    if let Some(addr) = wallet_address {
        checkout.push_str(&format!("&walletAddress={addr}"));
    }

    OnRampQuote {
        fiat_currency: fiat_currency.to_string(),
        fiat_amount,
        crypto_asset: crypto_asset.to_string(),
        crypto_chain: crypto_chain.to_string(),
        crypto_amount,
        exchange_rate: rate,
        total_fee: Some(fiat_amount * Decimal::new(3, 2) / Decimal::from(100)),
        checkout_url: Some(checkout),
    }
}

fn to_provider_quote(q: OnRampQuote, mock: bool) -> ProviderQuote {
    ProviderQuote {
        fiat_currency: q.fiat_currency,
        fiat_amount: q.fiat_amount,
        crypto_asset: q.crypto_asset,
        crypto_chain: q.crypto_chain,
        crypto_amount: q.crypto_amount,
        exchange_rate: q.exchange_rate,
        fee_fiat: q.total_fee,
        checkout_url: q.checkout_url,
        external_order_id: None,
        mock,
        raw: serde_json::json!({ "onramp": "transak", "mock": mock }),
    }
}
