use serde::{Deserialize, Serialize};

/// RSC Bank KYC status — mapped from Striga, never exposing provider names to clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum KycStatus {
    Pending,
    InReview,
    Approved,
    Rejected,
}

impl KycStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::InReview => "IN_REVIEW",
            Self::Approved => "APPROVED",
            Self::Rejected => "REJECTED",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value.to_uppercase().as_str() {
            "APPROVED" | "VERIFIED" | "ACTIVE" => Self::Approved,
            "REJECTED" | "DECLINED" | "FAILED" => Self::Rejected,
            "IN_REVIEW" | "INREVIEW" | "SUBMITTED" | "PENDING_REVIEW" => Self::InReview,
            _ => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CardType {
    Virtual,
    Physical,
}

impl CardType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Virtual => "VIRTUAL",
            Self::Physical => "PHYSICAL",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CardStatus {
    Pending,
    Active,
    Frozen,
    Terminated,
    Dispatched,
}

impl CardStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Active => "ACTIVE",
            Self::Frozen => "FROZEN",
            Self::Terminated => "TERMINATED",
            Self::Dispatched => "DISPATCHED",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value.to_uppercase().as_str() {
            "ACTIVE" => Self::Active,
            "BLOCKED" | "FROZEN" => Self::Frozen,
            "CLOSED" | "TERMINATED" | "BURNED" | "LOST" => Self::Terminated,
            "DISPATCHED" => Self::Dispatched,
            _ => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrigaUser {
    pub id: String,
    pub email: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateStrigaUserRequest {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartKycResponse {
    pub status: KycStatus,
    pub verification_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KycStatusResponse {
    pub status: KycStatus,
    pub tier: i16,
    pub rejection_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardResponse {
    pub id: String,
    pub card_type: CardType,
    pub status: CardStatus,
    pub last_four: Option<String>,
    pub expiry_month: Option<i16>,
    pub expiry_year: Option<i16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardTransaction {
    pub external_id: String,
    pub amount: rust_decimal::Decimal,
    pub currency: String,
    pub direction: String,
    pub merchant_name: Option<String>,
    pub status: String,
    pub transacted_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct StrigaWebhookEvent {
    pub event_type: String,
    pub external_id: Option<String>,
    pub user_id: Option<String>,
    pub card_id: Option<String>,
    pub raw: serde_json::Value,
}
