use async_trait::async_trait;
use reqwest::Client;
use rust_decimal::Decimal;

use crate::internal::swaps::error::{SwapError, SwapResult};
use crate::internal::swaps::models::{LiquidityVenueKind, SwapPair};
use crate::internal::swaps::models::supported_pairs;
use crate::internal::swaps::providers::settings::ProviderRuntimeConfig;
use crate::internal::swaps::traits::{
    ExecuteRequest, ExecuteResult, LiquidityQuote, QuoteRequest, SwapLiquidityProvider,
};

/// Config-driven DEX or CEX adapter — no provider brand names in business logic.
pub struct ConfiguredLiquidityAdapter {
    config: ProviderRuntimeConfig,
    #[allow(dead_code)]
    http: Client,
    pairs: Vec<SwapPair>,
}

impl ConfiguredLiquidityAdapter {
    pub fn new(config: ProviderRuntimeConfig, http: Client) -> Self {
        Self {
            config,
            http,
            pairs: supported_pairs(),
        }
    }

    fn mock_rate(&self, from: &str, to: &str) -> Decimal {
        let base = match (from, to) {
            ("BTC", "USDT") | ("USDT", "BTC") => Decimal::new(65_000, 0),
            ("RSC", "BTC") => Decimal::new(25, 6),
            ("BTC", "RSC") => Decimal::new(40_000, 0),
            ("ETH", "BRL") => Decimal::new(15_000, 0),
            ("BRL", "ETH") => Decimal::new(67, 5),
            _ => Decimal::ONE,
        };

        let spread_bps = match self.config.venue_kind() {
            LiquidityVenueKind::Dex => 30,
            LiquidityVenueKind::Cex => 15,
        };

        let adjustment = Decimal::ONE
            - Decimal::from(spread_bps) / Decimal::from(10_000);
        base * adjustment
    }

    fn quote_mock(&self, request: &QuoteRequest) -> SwapResult<LiquidityQuote> {
        let from = request.pair.from_asset.to_uppercase();
        let to = request.pair.to_asset.to_uppercase();
        let rate = self.mock_rate(&from, &to);

        let (from_amount, to_amount) = match (request.from_amount, request.to_amount) {
            (Some(f), _) if f > Decimal::ZERO => {
                let gross = f * rate;
                let fee = gross * Decimal::new(2, 3);
                (f, gross - fee)
            }
            (_, Some(t)) if t > Decimal::ZERO => {
                let fee = t * Decimal::new(2, 3) / (Decimal::ONE - Decimal::new(2, 3));
                let from = (t + fee) / rate;
                (from, t)
            }
            _ => return Err(SwapError::InvalidAmount),
        };

        let network_fee = match self.config.venue_kind() {
            LiquidityVenueKind::Dex => Decimal::new(5, 0),
            LiquidityVenueKind::Cex => Decimal::ZERO,
        };

        Ok(LiquidityQuote {
            from_asset: from,
            to_asset: to,
            from_amount,
            to_amount,
            exchange_rate: rate,
            fee_provider: to_amount * Decimal::new(1, 3),
            fee_network: network_fee,
            mock: true,
            raw: serde_json::json!({
                "adapter": self.config.id(),
                "venue": self.config.venue_kind().as_str(),
                "mock": true
            }),
        })
    }

    async fn quote_upstream(&self, request: &QuoteRequest) -> SwapResult<LiquidityQuote> {
        let base = self
            .config
            .settings
            .api_base_url
            .as_deref()
            .ok_or_else(|| SwapError::ProviderUnavailable)?;

        let url = format!("{base}/quote");
        let response = self
            .http
            .post(&url)
            .json(&serde_json::json!({
                "from": request.pair.from_asset,
                "to": request.pair.to_asset,
                "fromAmount": request.from_amount,
                "toAmount": request.to_amount,
                "slippageBps": request.slippage_bps,
            }))
            .send()
            .await
            .map_err(|e| SwapError::Upstream(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(SwapError::Upstream(format!("HTTP {status}: {body}")));
        }

        let parsed: UpstreamQuoteResponse = response
            .json()
            .await
            .map_err(|e| SwapError::Upstream(e.to_string()))?;

        Ok(LiquidityQuote {
            from_asset: request.pair.from_asset.clone(),
            to_asset: request.pair.to_asset.clone(),
            from_amount: parsed.from_amount,
            to_amount: parsed.to_amount,
            exchange_rate: parsed.exchange_rate,
            fee_provider: parsed.fee_provider.unwrap_or_default(),
            fee_network: parsed.fee_network.unwrap_or_default(),
            mock: false,
            raw: serde_json::to_value(&parsed).unwrap_or_default(),
        })
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct UpstreamQuoteResponse {
    from_amount: Decimal,
    to_amount: Decimal,
    exchange_rate: Decimal,
    #[serde(default)]
    fee_provider: Option<Decimal>,
    #[serde(default)]
    fee_network: Option<Decimal>,
}

#[async_trait]
impl SwapLiquidityProvider for ConfiguredLiquidityAdapter {
    fn id(&self) -> &str {
        self.config.id()
    }

    fn venue_kind(&self) -> LiquidityVenueKind {
        self.config.venue_kind()
    }

    fn is_configured(&self) -> bool {
        self.config.is_configured()
    }

    fn uses_mock(&self) -> bool {
        self.config.uses_mock()
    }

    fn supported_pairs(&self) -> &[SwapPair] {
        &self.pairs
    }

    async fn quote(&self, request: &QuoteRequest) -> SwapResult<LiquidityQuote> {
        if !self.supports_pair(&request.pair.from_asset, &request.pair.to_asset) {
            return Err(SwapError::UnsupportedPair);
        }

        if self.uses_mock() {
            return self.quote_mock(request);
        }

        self.quote_upstream(request).await
    }

    async fn execute(&self, request: &ExecuteRequest) -> SwapResult<ExecuteResult> {
        if self.uses_mock() {
            let external_id = format!("mock-{}-{}", self.id(), uuid::Uuid::new_v4());
            return Ok(ExecuteResult {
                external_order_id: external_id,
                from_amount: request.from_amount,
                to_amount: request.min_to_amount,
                status: "completed".into(),
                raw: serde_json::json!({
                    "provider": self.id(),
                    "mock": true
                }),
            });
        }

        let base = self
            .config
            .settings
            .api_base_url
            .as_deref()
            .ok_or_else(|| SwapError::ProviderUnavailable)?;

        let url = format!("{base}/swap");
        let response = self
            .http
            .post(&url)
            .json(&serde_json::json!({
                "from": request.pair.from_asset,
                "to": request.pair.to_asset,
                "fromAmount": request.from_amount,
                "minToAmount": request.min_to_amount,
                "slippageBps": request.slippage_bps,
                "idempotencyKey": request.idempotency_key,
            }))
            .send()
            .await
            .map_err(|e| SwapError::Upstream(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(SwapError::Upstream(format!("HTTP {status}: {body}")));
        }

        let parsed: UpstreamExecuteResponse = response
            .json()
            .await
            .map_err(|e| SwapError::Upstream(e.to_string()))?;

        let raw = serde_json::to_value(&parsed).unwrap_or_default();
        Ok(ExecuteResult {
            external_order_id: parsed.order_id,
            from_amount: parsed.from_amount,
            to_amount: parsed.to_amount,
            status: parsed.status,
            raw,
        })
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct UpstreamExecuteResponse {
    order_id: String,
    from_amount: Decimal,
    to_amount: Decimal,
    status: String,
}
