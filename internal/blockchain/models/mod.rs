use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

/// Gateway-facing view of an on-chain balance (wei as decimal string).
#[derive(Debug, Clone, Serialize)]
pub struct OnChainBalance {
    pub address: String,
    pub balance_wei: String,
    pub block_number: u64,
}

/// Normalized transaction status from the node / mempool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OnChainTxState {
    Pending,
    Success,
    Failed,
    NotFound,
}

#[derive(Debug, Clone, Serialize)]
pub struct OnChainTransaction {
    pub hash: String,
    pub state: OnChainTxState,
    pub block_number: Option<u64>,
    pub confirmations: u64,
    pub from: Option<String>,
    pub to: Option<String>,
    pub value_wei: Option<String>,
    pub gas_used: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct OnChainBlock {
    pub number: u64,
    pub hash: String,
    pub parent_hash: String,
    pub timestamp: u64,
    pub transaction_count: usize,
}

/// Broadcast a signed raw transaction (hex, with or without 0x prefix).
#[derive(Debug, Deserialize, Validate)]
pub struct BroadcastTxRequest {
    #[validate(length(min = 2, max = 512_000))]
    pub raw_tx_hex: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BroadcastTxResponse {
    pub tx_hash: String,
}

/// Events pushed from WebSocket subscriptions (translated for gateway consumers).
#[derive(Debug, Clone, Serialize)]
pub struct BlockchainEvent {
    pub network: String,
    pub event_type: BlockchainEventType,
    pub payload: serde_json::Value,
    pub received_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockchainEventType {
    NewHead,
    PendingTx,
    Log,
    Raw,
}

/// Incoming native transfer detected in a block (before wallet matching).
#[derive(Debug, Clone)]
pub struct BlockTransfer {
    pub tx_hash: String,
    pub from: Option<String>,
    pub to: String,
    pub value_wei: String,
    pub block_number: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct NodeHealth {
    pub rpc_url: String,
    pub ws_configured: bool,
    pub chain_id: Option<u64>,
    pub latest_block: Option<u64>,
    pub syncing: bool,
}
