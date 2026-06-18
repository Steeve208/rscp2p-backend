use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    AwaitingPayment,
    Processing,
    Completed,
    Cancelled,
    Failed,
    Expired,
    Unknown,
}

impl OrderStatus {
    pub fn parse(value: &str) -> Self {
        match value.to_uppercase().as_str() {
            "AWAITING_PAYMENT_FROM_USER" | "PAYMENT_DONE" | "PROCESSING" => Self::Processing,
            "COMPLETED" | "COMPLETE" | "SUCCESS" => Self::Completed,
            "CANCELLED" | "CANCELED" => Self::Cancelled,
            "FAILED" | "DECLINED" => Self::Failed,
            "EXPIRED" => Self::Expired,
            _ => Self::Unknown,
        }
    }

    pub fn is_success(self) -> bool {
        matches!(self, Self::Completed)
    }

    pub fn is_failure(self) -> bool {
        matches!(self, Self::Failed | Self::Cancelled | Self::Expired)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::AwaitingPayment => "AWAITING_PAYMENT_FROM_USER",
            Self::Processing => "PROCESSING",
            Self::Completed => "COMPLETED",
            Self::Cancelled => "CANCELLED",
            Self::Failed => "FAILED",
            Self::Expired => "EXPIRED",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransakOrder {
    pub id: String,
    pub status: OrderStatus,
    pub fiat_currency: Option<String>,
    pub fiat_amount: Option<Decimal>,
    pub crypto_asset: Option<String>,
    pub crypto_amount: Option<Decimal>,
    pub wallet_address: Option<String>,
    pub partner_order_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnRampQuote {
    pub fiat_currency: String,
    pub fiat_amount: Decimal,
    pub crypto_asset: String,
    pub crypto_chain: String,
    pub crypto_amount: Decimal,
    pub exchange_rate: Decimal,
    pub total_fee: Option<Decimal>,
    pub checkout_url: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KycStatus {
    NotStarted,
    Pending,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KycUserStatus {
    pub user_id: String,
    pub status: KycStatus,
    pub level: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMethodOption {
    pub id: String,
    pub label: String,
    pub payment_method_type: String,
    pub enabled: bool,
}
