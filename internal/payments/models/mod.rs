use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

// ==================== Enums ====================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MerchantStatus {
    Pending,
    Active,
    Suspended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InvoiceStatus {
    Draft,
    Pending,
    Paid,
    Expired,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentStatus {
    Pending,
    Completed,
    Failed,
    Refunded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethod {
    Qr,
    Instant,
    Invoice,
    FiatRamp,
    FiatTransak,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettlementStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

// ==================== Domain models ====================

#[derive(Debug, Clone, Serialize)]
pub struct Merchant {
    pub id: Uuid,
    pub owner_user_id: Uuid,
    pub wallet_id: Option<Uuid>,
    pub display_name: String,
    pub legal_name: Option<String>,
    pub status: MerchantStatus,
    pub settlement_asset: String,
    pub settlement_chain: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaymentInvoice {
    pub id: Uuid,
    pub merchant_id: Uuid,
    pub reference_code: String,
    pub amount: Decimal,
    pub asset: String,
    pub chain: String,
    pub description: Option<String>,
    pub status: InvoiceStatus,
    pub expires_at: Option<DateTime<Utc>>,
    pub paid_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Payment {
    pub id: Uuid,
    pub invoice_id: Uuid,
    pub payer_user_id: Uuid,
    pub amount: Decimal,
    pub fee: Decimal,
    pub method: PaymentMethod,
    pub status: PaymentStatus,
    pub idempotency_key: String,
    pub wallet_journal_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Settlement {
    pub id: Uuid,
    pub merchant_id: Uuid,
    pub amount: Decimal,
    pub asset: String,
    pub chain: String,
    pub status: SettlementStatus,
    pub wallet_journal_id: Option<Uuid>,
    pub destination_wallet_id: Option<Uuid>,
    pub period_start: Option<DateTime<Utc>>,
    pub period_end: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ==================== Requests / responses ====================

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterMerchantRequest {
    #[validate(length(min = 2, max = 120))]
    pub display_name: String,
    pub legal_name: Option<String>,
    pub wallet_id: Option<Uuid>,
    #[validate(length(min = 2, max = 32))]
    pub settlement_asset: Option<String>,
    #[validate(length(min = 3, max = 32))]
    pub settlement_chain: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MerchantResponse {
    pub merchant: Merchant,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateInvoiceRequest {
    pub amount: Decimal,
    #[validate(length(min = 2, max = 32))]
    pub asset: String,
    #[validate(length(min = 3, max = 32))]
    pub chain: String,
    pub description: Option<String>,
    pub expires_in_minutes: Option<u64>,
    #[validate(length(min = 8, max = 128))]
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateInvoiceResponse {
    pub invoice: PaymentInvoice,
    pub qr: QrPaymentPayload,
}

/// Payload encoded in QR (RSC Pay deep link / reference).
#[derive(Debug, Clone, Serialize)]
pub struct QrPaymentPayload {
    pub scheme: String,
    pub reference_code: String,
    pub merchant_id: Uuid,
    pub amount: Decimal,
    pub asset: String,
    pub chain: String,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct InvoicePublicView {
    pub reference_code: String,
    pub merchant_display_name: String,
    pub amount: Decimal,
    pub asset: String,
    pub chain: String,
    pub description: Option<String>,
    pub status: InvoiceStatus,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct PayInvoiceRequest {
    #[validate(length(min = 8, max = 128))]
    pub idempotency_key: String,
    pub method: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PayInvoiceResponse {
    pub payment: Payment,
    pub invoice: PaymentInvoice,
    pub idempotent_replay: bool,
    pub wallet_transfer_idempotent_replay: bool,
}

#[derive(Debug, Deserialize, Validate)]
pub struct RequestSettlementRequest {
    #[validate(length(min = 8, max = 128))]
    pub idempotency_key: String,
    pub period_start: Option<DateTime<Utc>>,
    pub period_end: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct SettlementResponse {
    pub settlement: Settlement,
    pub payment_count: usize,
    pub wallet_transfer_idempotent_replay: bool,
}
