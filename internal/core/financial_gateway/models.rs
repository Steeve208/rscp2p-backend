use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::internal::providers::striga::models::KycStatus;
use crate::internal::providers::striga::repository::{CardRow, WebhookLogRow};

#[derive(Debug, Serialize)]
pub struct BankingUserResponse {
    pub user_id: Uuid,
    pub email: Option<String>,
    pub kyc_status: Option<String>,
    pub has_card: bool,
}

#[derive(Debug, Serialize)]
pub struct StartKycGatewayResponse {
    pub status: KycStatus,
    /// Token for embedded KYC SDK — never a Striga dashboard URL.
    pub verification_token: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct KycStatusGatewayResponse {
    pub status: KycStatus,
    pub tier: i16,
    pub rejection_reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CardGatewayResponse {
    pub id: Uuid,
    pub card_type: String,
    pub status: String,
    pub last_four: Option<String>,
    pub expiry_month: Option<i16>,
    pub expiry_year: Option<i16>,
    pub created_at: DateTime<Utc>,
}

impl From<CardRow> for CardGatewayResponse {
    fn from(row: CardRow) -> Self {
        Self {
            id: row.id,
            card_type: row.card_type,
            status: row.card_status,
            last_four: row.last_four,
            expiry_month: row.expiry_month,
            expiry_year: row.expiry_year,
            created_at: row.created_at,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CardTransactionResponse {
    pub external_id: String,
    pub amount: rust_decimal::Decimal,
    pub currency: String,
    pub direction: String,
    pub merchant_name: Option<String>,
    pub status: String,
    pub transacted_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CryptoQuoteRequest {
    pub fiat_currency: String,
    pub crypto_asset: String,
    pub fiat_amount: Option<rust_decimal::Decimal>,
    pub crypto_amount: Option<rust_decimal::Decimal>,
    pub crypto_chain: Option<String>,
    /// BUY or SELL — defaults to BUY.
    pub side: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CryptoQuoteResponse {
    pub fiat_currency: String,
    pub fiat_amount: rust_decimal::Decimal,
    pub crypto_asset: String,
    pub crypto_amount: rust_decimal::Decimal,
    pub rate: rust_decimal::Decimal,
    pub side: String,
}

#[derive(Debug, Deserialize)]
pub struct CryptoTradeRequest {
    pub fiat_currency: String,
    pub crypto_asset: String,
    pub fiat_amount: Option<rust_decimal::Decimal>,
    pub crypto_amount: Option<rust_decimal::Decimal>,
    pub crypto_chain: Option<String>,
    pub wallet_address: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CryptoWidgetResponse {
    pub order_id: Uuid,
    /// URL for embedded checkout widget inside RSC Bank.
    pub widget_url: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct CryptoOrderResponse {
    pub id: Uuid,
    pub order_type: String,
    pub status: String,
    pub fiat_currency: Option<String>,
    pub fiat_amount: Option<rust_decimal::Decimal>,
    pub crypto_asset: Option<String>,
    pub crypto_amount: Option<rust_decimal::Decimal>,
    pub widget_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ProviderHealth {
    pub configured: bool,
    pub mock_mode: bool,
    pub status: String,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ProvidersDashboardResponse {
    pub striga: ProviderHealth,
    pub transak: ProviderHealth,
    pub recent_webhooks: Vec<WebhookLogRow>,
}

#[derive(Debug, Deserialize)]
pub struct ActivateCardRequest {
    pub activation_code: String,
}
