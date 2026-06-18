use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// Supported asset (can be extended)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Asset(pub String);

impl Asset {
    pub fn btc() -> Self {
        Self("BTC".into())
    }
    pub fn eth() -> Self {
        Self("ETH".into())
    }
    pub fn rsc() -> Self {
        Self("RSC".into())
    }
}

/// Blockchain / network
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Chain(pub String);

impl Chain {
    pub fn bitcoin() -> Self {
        Self("bitcoin".into())
    }
    pub fn ethereum() -> Self {
        Self("ethereum".into())
    }
    pub fn rsc_mainnet() -> Self {
        Self("rsc-mainnet".into())
    }
}

/// A user's wallet (can have multiple in the future)
#[derive(Debug, Clone, Serialize)]
pub struct Wallet {
    pub id: Uuid,
    pub user_id: Uuid,
    pub label: Option<String>,
    pub is_default: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Current balance for a specific asset on a specific chain
#[derive(Debug, Clone, Serialize)]
pub struct WalletBalance {
    pub wallet_id: Uuid,
    pub asset: Asset,
    pub chain: Chain,
    pub available: Decimal,
    pub total: Decimal,
    pub locked: Decimal,
    pub updated_at: DateTime<Utc>,
}

/// Deposit / receive address
#[derive(Debug, Clone, Serialize)]
pub struct WalletAddress {
    pub id: Uuid,
    pub wallet_id: Uuid,
    pub asset: Asset,
    pub chain: Chain,
    pub address: String,
    pub derivation_path: Option<String>,
    pub is_used: bool,
    pub created_at: DateTime<Utc>,
}

/// Type of ledger movement (double-entry)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LedgerEntryType {
    Deposit,
    Withdrawal,
    InternalTransfer,
    Fee,
    Adjustment,
}

/// A single line in the double-entry ledger
#[derive(Debug, Clone, Serialize)]
pub struct LedgerEntry {
    pub id: Uuid,
    pub journal_id: Uuid,
    pub wallet_id: Uuid,
    pub asset: Asset,
    pub chain: Chain,
    pub amount: Decimal, // positive = credit (increase balance)
    pub entry_type: LedgerEntryType,
    pub related_wallet_id: Option<Uuid>,
    pub transaction_id: Option<Uuid>,
    pub idempotency_key: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

/// High-level transaction for history / UI
#[derive(Debug, Clone, Serialize)]
pub struct WalletTransaction {
    pub id: Uuid,
    pub wallet_id: Uuid,
    pub r#type: TransactionType,
    pub asset: Asset,
    pub chain: Chain,
    pub amount: Decimal,
    pub fee: Decimal,
    pub status: TransactionStatus,
    pub tx_hash: Option<String>,
    pub from_address: Option<String>,
    pub to_address: Option<String>,
    pub confirmations: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionType {
    Deposit,
    Withdrawal,
    TransferIn,
    TransferOut,
    Fee,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionStatus {
    Pending,
    Confirming,
    Confirmed,
    Failed,
    Cancelled,
}

/// Request to create a new deposit address
#[derive(Debug, Deserialize, Validate)]
pub struct CreateAddressRequest {
    #[validate(length(min = 2, max = 32))]
    pub asset: String,
    #[validate(length(min = 3, max = 32))]
    pub chain: String,
}

#[derive(Debug, Serialize)]
pub struct DepositAddressResponse {
    pub wallet: Wallet,
    pub address: WalletAddress,
}

#[derive(Debug, Serialize)]
pub struct EnsureDefaultWalletResponse {
    pub wallet: Wallet,
}

/// Resolved deposit destination from `wallet_addresses`.
#[derive(Debug, Clone)]
pub struct DepositTarget {
    pub wallet_id: uuid::Uuid,
    pub asset: String,
    pub chain: String,
}

/// Internal worker request used after an on-chain deposit reaches enough confirmations.
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct RecordDepositRequest {
    pub wallet_id: Uuid,
    #[validate(length(min = 2, max = 32))]
    pub asset: String,
    #[validate(length(min = 3, max = 32))]
    pub chain: String,
    #[validate(length(min = 1, max = 128))]
    pub tx_hash: String,
    #[validate(range(min = 1))]
    pub confirmations: i32,
    #[validate(length(min = 8, max = 128))]
    pub idempotency_key: String,
    pub amount: Decimal,
    pub from_address: Option<String>,
    pub to_address: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct RecordDepositResponse {
    pub transaction: WalletTransaction,
    pub balance: WalletBalance,
    pub ledger_entry: LedgerEntry,
    pub idempotent_replay: bool,
}

/// Internal transfer between two wallets (same asset/chain). Used by RSC Pay.
#[derive(Debug, Clone)]
pub struct InternalTransferRequest {
    pub from_wallet_id: Uuid,
    pub to_wallet_id: Uuid,
    pub asset: String,
    pub chain: String,
    pub amount: Decimal,
    pub idempotency_key: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct InternalTransferResponse {
    pub journal_id: Uuid,
    pub from_balance: WalletBalance,
    pub to_balance: WalletBalance,
    pub idempotent_replay: bool,
}

/// User-initiated withdrawal (locks balance until broadcast + confirmation).
#[derive(Debug, Deserialize, Validate)]
pub struct RequestWithdrawalRequest {
    #[validate(length(min = 2, max = 32))]
    pub asset: String,
    #[validate(length(min = 3, max = 32))]
    pub chain: String,
    pub amount: Decimal,
    #[validate(length(min = 8, max = 128))]
    pub to_address: String,
    #[validate(length(min = 8, max = 128))]
    pub idempotency_key: String,
    pub wallet_id: Option<Uuid>,
    pub fee: Option<Decimal>,
}

#[derive(Debug, Serialize)]
pub struct RequestWithdrawalResponse {
    pub transaction: WalletTransaction,
    pub idempotent_replay: bool,
}

#[derive(Debug, Deserialize, Validate)]
pub struct BroadcastWithdrawalRequest {
    #[validate(length(min = 2, max = 512_000))]
    pub raw_tx_hex: String,
}

#[derive(Debug, Serialize)]
pub struct BroadcastWithdrawalResponse {
    pub transaction: WalletTransaction,
    pub tx_hash: String,
}

/// Response after requesting a withdrawal (legacy alias)
#[derive(Debug, Serialize)]
pub struct WithdrawalResponse {
    pub transaction_id: Uuid,
    pub status: TransactionStatus,
}
