use reqwest::Client;
use rust_decimal::Decimal;
use serde::Deserialize;

use crate::internal::providers::transak::config::TransakProviderConfig;
use crate::internal::providers::transak::error::{TransakError, TransakResult};
use crate::internal::providers::transak::models::{OnRampQuote, OrderStatus, TransakOrder};

#[derive(Clone)]
pub struct TransakApiClient {
    http: Client,
    config: TransakProviderConfig,
}

impl TransakApiClient {
    pub fn new(http: Client, config: TransakProviderConfig) -> Self {
        Self { http, config }
    }

    pub fn uses_mock(&self) -> bool {
        self.config.mock_mode || self.config.api_key.is_none()
    }

    fn base_url(&self) -> &str {
        self.config
            .api_base_url
            .as_deref()
            .unwrap_or_else(|| self.config.default_api_base_url())
    }

    fn api_key(&self) -> TransakResult<&str> {
        self.config
            .api_key
            .as_deref()
            .ok_or(TransakError::NotConfigured)
    }

    pub fn map_chain(chain: &str) -> String {
        match chain {
            "rsc-mainnet" | "rsc" => "ethereum".into(),
            "ethereum" | "eth" => "ethereum".into(),
            "bitcoin" | "btc" => "bitcoin".into(),
            other => other.to_string(),
        }
    }

    /// `GET /api/v1/pricing/public/quotes` — buy (on-ramp).
    pub async fn price_quote_buy(
        &self,
        fiat_currency: &str,
        crypto_asset: &str,
        network: &str,
        fiat_amount: Decimal,
    ) -> TransakResult<OnRampQuote> {
        self.fetch_price_quote(fiat_currency, crypto_asset, network, fiat_amount, "BUY", None)
            .await
    }

    /// `GET /api/v1/pricing/public/quotes` — sell (off-ramp).
    pub async fn price_quote_sell(
        &self,
        fiat_currency: &str,
        crypto_asset: &str,
        network: &str,
        crypto_amount: Decimal,
    ) -> TransakResult<OnRampQuote> {
        self.fetch_price_quote(
            fiat_currency,
            crypto_asset,
            network,
            crypto_amount,
            "SELL",
            Some(true),
        )
        .await
    }

    /// Legacy buy-only alias.
    pub async fn price_quote(
        &self,
        fiat_currency: &str,
        crypto_asset: &str,
        network: &str,
        fiat_amount: Decimal,
    ) -> TransakResult<OnRampQuote> {
        self.price_quote_buy(fiat_currency, crypto_asset, network, fiat_amount)
            .await
    }

    async fn fetch_price_quote(
        &self,
        fiat_currency: &str,
        crypto_asset: &str,
        network: &str,
        amount: Decimal,
        side: &str,
        sell_by_crypto: Option<bool>,
    ) -> TransakResult<OnRampQuote> {
        if self.uses_mock() {
            return Err(TransakError::NotConfigured);
        }

        let url = format!("{}/api/v1/pricing/public/quotes", self.base_url());
        let mut query = vec![
            ("partnerApiKey", self.api_key()?),
            ("fiatCurrency", fiat_currency),
            ("cryptoCurrency", crypto_asset),
            ("network", network),
            ("isBuyOrSell", side),
            ("paymentMethod", "credit_debit_card"),
        ];

        let amount_str = amount.to_string();
        if sell_by_crypto == Some(true) {
            query.push(("cryptoAmount", &amount_str));
        } else {
            query.push(("fiatAmount", &amount_str));
        }

        let response = self
            .http
            .get(&url)
            .query(&query)
            .send()
            .await
            .map_err(|e| TransakError::Upstream(format!("price-quote failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(TransakError::Upstream(format!(
                "price-quote HTTP {status}: {text}"
            )));
        }

        let parsed: PriceQuoteWrapper = response
            .json()
            .await
            .map_err(|e| TransakError::Parse(e.to_string()))?;

        let data = parsed.response;
        let crypto_out = data.crypto_amount.unwrap_or(Decimal::ZERO);
        let fiat_out = data.fiat_amount.unwrap_or(amount);
        let rate = if crypto_out > Decimal::ZERO {
            fiat_out / crypto_out
        } else {
            Decimal::ONE
        };

        Ok(OnRampQuote {
            fiat_currency: fiat_currency.to_uppercase(),
            fiat_amount: fiat_out,
            crypto_asset: crypto_asset.to_uppercase(),
            crypto_chain: String::new(),
            crypto_amount: crypto_out,
            exchange_rate: rate,
            total_fee: data.total_fee,
            checkout_url: None,
        })
    }

    /// `GET /api/v2/order/{orderId}` (partner API).
    pub async fn get_order(&self, order_id: &str) -> TransakResult<TransakOrder> {
        if self.uses_mock() {
            return Err(TransakError::NotConfigured);
        }

        let url = format!("{}/api/v2/order/{order_id}", self.base_url());
        let response = self
            .http
            .get(&url)
            .query(&[("partnerApiKey", self.api_key()?)])
            .send()
            .await
            .map_err(|e| TransakError::Upstream(format!("get order failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(TransakError::Upstream(format!(
                "order HTTP {status}: {text}"
            )));
        }

        let parsed: OrderWrapper = response
            .json()
            .await
            .map_err(|e| TransakError::Parse(e.to_string()))?;

        Ok(TransakOrder {
            id: parsed.id,
            status: OrderStatus::parse(&parsed.status),
            fiat_currency: parsed.fiat_currency,
            fiat_amount: parsed.fiat_amount,
            crypto_asset: parsed.crypto_currency,
            crypto_amount: parsed.crypto_amount,
            wallet_address: parsed.wallet_address,
            partner_order_id: parsed.partner_order_id,
        })
    }
}

#[derive(Debug, Deserialize)]
struct PriceQuoteWrapper {
    response: PriceQuoteData,
}

#[derive(Debug, Deserialize)]
struct PriceQuoteData {
    #[serde(default)]
    fiat_amount: Option<Decimal>,
    #[serde(default)]
    crypto_amount: Option<Decimal>,
    #[serde(default)]
    total_fee: Option<Decimal>,
}

#[derive(Debug, Deserialize)]
struct OrderWrapper {
    id: String,
    status: String,
    #[serde(default)]
    fiat_currency: Option<String>,
    #[serde(default)]
    fiat_amount: Option<Decimal>,
    #[serde(default)]
    crypto_currency: Option<String>,
    #[serde(default)]
    crypto_amount: Option<Decimal>,
    #[serde(default)]
    wallet_address: Option<String>,
    #[serde(default)]
    partner_order_id: Option<String>,
}
