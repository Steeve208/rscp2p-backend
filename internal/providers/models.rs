use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FiatProvider {
    /// Crypto on-ramp / off-ramp (invoice fiat pay).
    Transak,
}

impl FiatProvider {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Transak => "transak",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "transak" => Some(Self::Transak),
            // Legacy DB rows from removed Ramp integration.
            "ramp" => Some(Self::Transak),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FiatOrderStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize)]
pub struct FiatProviderInfo {
    pub provider: FiatProvider,
    pub configured: bool,
    pub mock_mode: bool,
}

#[derive(Debug, Deserialize, Validate)]
pub struct FiatQuoteRequest {
    pub provider: Option<String>,
    #[validate(length(min = 3, max = 8))]
    pub fiat_currency: String,
    pub fiat_amount: Option<Decimal>,
    #[validate(length(min = 2, max = 32))]
    pub crypto_asset: String,
    #[validate(length(min = 3, max = 32))]
    pub crypto_chain: String,
    pub crypto_amount: Option<Decimal>,
}

#[derive(Debug, Serialize)]
pub struct FiatQuoteResponse {
    pub provider: FiatProvider,
    pub fiat_currency: String,
    pub fiat_amount: Decimal,
    pub crypto_asset: String,
    pub crypto_chain: String,
    pub crypto_amount: Decimal,
    pub exchange_rate: Decimal,
    pub fee_fiat: Option<Decimal>,
    pub mock: bool,
}

#[derive(Debug, Deserialize, Validate)]
pub struct StartFiatInvoicePayRequest {
    #[validate(length(min = 8, max = 128))]
    pub idempotency_key: String,
    #[validate(length(min = 3, max = 8))]
    pub fiat_currency: String,
    pub provider: String,
    /// Optional redirect URL passed to provider widget (when supported).
    pub redirect_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FiatConversionOrder {
    pub id: Uuid,
    pub user_id: Uuid,
    pub invoice_id: Option<Uuid>,
    pub payment_id: Option<Uuid>,
    pub provider: FiatProvider,
    pub external_order_id: Option<String>,
    pub status: FiatOrderStatus,
    pub fiat_currency: String,
    pub fiat_amount: Decimal,
    pub crypto_asset: String,
    pub crypto_chain: String,
    pub crypto_amount: Decimal,
    pub exchange_rate: Option<Decimal>,
    pub checkout_url: Option<String>,
    pub wallet_address: Option<String>,
    pub idempotency_key: String,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct StartFiatInvoicePayResponse {
    pub order: FiatConversionOrder,
    pub quote: FiatQuoteResponse,
    pub idempotent_replay: bool,
}

#[derive(Debug, Clone)]
pub struct ProviderQuote {
    pub fiat_currency: String,
    pub fiat_amount: Decimal,
    pub crypto_asset: String,
    pub crypto_chain: String,
    pub crypto_amount: Decimal,
    pub exchange_rate: Decimal,
    pub fee_fiat: Option<Decimal>,
    pub checkout_url: Option<String>,
    pub external_order_id: Option<String>,
    pub mock: bool,
    pub raw: serde_json::Value,
}
