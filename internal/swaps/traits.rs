use async_trait::async_trait;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::internal::swaps::error::SwapResult;
use crate::internal::swaps::models::{LiquidityVenueKind, SwapPair};

/// Provider-agnostic quote returned by any DEX/CEX adapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityQuote {
    pub from_asset: String,
    pub to_asset: String,
    pub from_amount: Decimal,
    pub to_amount: Decimal,
    pub exchange_rate: Decimal,
    pub fee_provider: Decimal,
    pub fee_network: Decimal,
    pub mock: bool,
    pub raw: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct QuoteRequest {
    pub pair: SwapPair,
    pub from_amount: Option<Decimal>,
    pub to_amount: Option<Decimal>,
    pub slippage_bps: u32,
}

#[derive(Debug, Clone)]
pub struct ExecuteRequest {
    pub pair: SwapPair,
    pub from_amount: Decimal,
    pub min_to_amount: Decimal,
    pub slippage_bps: u32,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecuteResult {
    pub external_order_id: String,
    pub from_amount: Decimal,
    pub to_amount: Decimal,
    pub status: String,
    pub raw: serde_json::Value,
}

/// Any liquidity source (DEX aggregator, CEX API, etc.) implements this trait.
#[async_trait]
pub trait SwapLiquidityProvider: Send + Sync {
    fn id(&self) -> &str;
    fn venue_kind(&self) -> LiquidityVenueKind;
    fn is_configured(&self) -> bool;
    fn uses_mock(&self) -> bool;
    fn supported_pairs(&self) -> &[SwapPair];

    fn supports_pair(&self, from: &str, to: &str) -> bool {
        self.supported_pairs()
            .iter()
            .any(|p| {
                p.from_asset.eq_ignore_ascii_case(from) && p.to_asset.eq_ignore_ascii_case(to)
            })
    }

    async fn quote(&self, request: &QuoteRequest) -> SwapResult<LiquidityQuote>;

    async fn execute(&self, request: &ExecuteRequest) -> SwapResult<ExecuteResult>;
}
