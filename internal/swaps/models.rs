use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LiquidityVenueKind {
    Dex,
    Cex,
}

impl LiquidityVenueKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Dex => "dex",
            Self::Cex => "cex",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_uppercase().as_str() {
            "DEX" => Some(Self::Dex),
            "CEX" => Some(Self::Cex),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AssetSymbol(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapPair {
    pub from_asset: String,
    pub to_asset: String,
    pub from_chain: Option<String>,
    pub to_chain: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SwapOrderStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize)]
pub struct SwapProviderInfo {
    pub id: String,
    pub venue_kind: LiquidityVenueKind,
    pub configured: bool,
    pub mock_mode: bool,
    pub supported_pairs: Vec<SwapPair>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct SwapQuoteRequest {
    #[validate(length(min = 2, max = 32))]
    pub from_asset: String,
    #[validate(length(min = 2, max = 32))]
    pub to_asset: String,
    pub from_chain: Option<String>,
    pub to_chain: Option<String>,
    pub from_amount: Option<Decimal>,
    pub to_amount: Option<Decimal>,
    /// Optional provider filter (must match a registered provider id).
    pub provider_id: Option<String>,
    #[serde(default = "default_slippage_bps")]
    pub slippage_bps: u32,
}

fn default_slippage_bps() -> u32 {
    50
}

#[derive(Debug, Clone, Serialize)]
pub struct FeeBreakdown {
    pub platform_bps: u32,
    pub platform_amount: Decimal,
    pub provider_amount: Decimal,
    pub network_amount: Decimal,
    pub total_amount: Decimal,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderQuoteView {
    pub provider_id: String,
    pub venue_kind: LiquidityVenueKind,
    pub from_asset: String,
    pub to_asset: String,
    pub from_amount: Decimal,
    pub to_amount: Decimal,
    pub exchange_rate: Decimal,
    pub fee_provider: Decimal,
    pub fee_network: Decimal,
    pub mock: bool,
}

#[derive(Debug, Serialize)]
pub struct SwapQuoteResponse {
    pub pair: SwapPair,
    pub best: ProviderQuoteView,
    pub alternatives: Vec<ProviderQuoteView>,
    pub fees: FeeBreakdown,
    pub slippage_bps: u32,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateSwapOrderRequest {
    #[validate(length(min = 8, max = 128))]
    pub idempotency_key: String,
    #[validate(length(min = 2, max = 32))]
    pub from_asset: String,
    #[validate(length(min = 2, max = 32))]
    pub to_asset: String,
    pub from_chain: Option<String>,
    pub to_chain: Option<String>,
    pub from_amount: Decimal,
    pub provider_id: Option<String>,
    #[serde(default = "default_slippage_bps")]
    pub slippage_bps: u32,
}

#[derive(Debug, Serialize)]
pub struct SwapOrder {
    pub id: Uuid,
    pub user_id: Uuid,
    pub provider_id: String,
    pub venue_kind: LiquidityVenueKind,
    pub from_asset: String,
    pub to_asset: String,
    pub from_chain: Option<String>,
    pub to_chain: Option<String>,
    pub from_amount: Decimal,
    pub to_amount: Decimal,
    pub fee_platform: Decimal,
    pub fee_provider: Decimal,
    pub fee_network: Decimal,
    pub exchange_rate: Option<Decimal>,
    pub slippage_bps: u32,
    pub status: SwapOrderStatus,
    pub idempotency_key: String,
    pub external_order_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct CreateSwapOrderResponse {
    pub order: SwapOrder,
    pub quote: ProviderQuoteView,
    pub idempotent_replay: bool,
}

/// Catalog of gateway-supported swap pairs (not provider-specific).
pub fn supported_pairs() -> Vec<SwapPair> {
    vec![
        pair("BTC", "USDT", None, None),
        pair("USDT", "BTC", None, None),
        pair("RSC", "BTC", Some("rsc-mainnet"), None),
        pair("BTC", "RSC", None, Some("rsc-mainnet")),
        pair("ETH", "BRL", Some("ethereum"), None),
        pair("BRL", "ETH", None, Some("ethereum")),
    ]
}

fn pair(
    from: &str,
    to: &str,
    from_chain: Option<&str>,
    to_chain: Option<&str>,
) -> SwapPair {
    SwapPair {
        from_asset: from.into(),
        to_asset: to.into(),
        from_chain: from_chain.map(str::to_string),
        to_chain: to_chain.map(str::to_string),
    }
}

pub fn pairs_match(a: &SwapPair, from: &str, to: &str) -> bool {
    a.from_asset.eq_ignore_ascii_case(from) && a.to_asset.eq_ignore_ascii_case(to)
}
