//! Blockchain service — translates node/indexer responses into gateway models.
//!
//! Wallets and workers call this layer; they should not talk to RPC/WS directly.

use reqwest::Client;
use std::sync::Arc;
use tokio::sync::mpsc;
use validator::Validate;

use crate::internal::blockchain::error::{BlockchainError, BlockchainResult};
use crate::internal::blockchain::models::{
    BlockTransfer, BlockchainEvent, BlockchainEventType, BroadcastTxRequest, BroadcastTxResponse,
    NodeHealth, OnChainBalance, OnChainBlock, OnChainTransaction,
};
use crate::internal::blockchain::rpc::hex_to_u64;
use crate::internal::blockchain::rpc::JsonRpcClient;
use crate::internal::blockchain::rsc::RscRpcClient;
use crate::internal::blockchain::ws;
use crate::internal::config::BlockchainConfig;

const NETWORK_RSC: &str = "rsc-mainnet";

#[derive(Clone)]
pub struct BlockchainService {
    rsc: RscRpcClient,
    config: BlockchainConfig,
    ws_events: Option<mpsc::Sender<BlockchainEvent>>,
}

impl BlockchainService {
    pub fn new(http: Client, config: BlockchainConfig) -> Self {
        let rpc = JsonRpcClient::new(http, config.rsc_rpc_url.clone());
        let rsc = RscRpcClient::new(rpc);
        Self {
            rsc,
            config,
            ws_events: None,
        }
    }

    /// Optional channel receives translated WebSocket events (for deposit workers, etc.).
    pub fn with_event_channel(mut self, tx: mpsc::Sender<BlockchainEvent>) -> Self {
        self.ws_events = Some(tx);
        self
    }

    /// Start background WebSocket listener if `RSC_WS_URL` is configured.
    pub fn spawn_event_listener(self: &Arc<Self>) -> Option<tokio::task::JoinHandle<()>> {
        let ws_url = self.config.rsc_ws_url.clone()?;
        let tx = self.ws_events.clone()?;
        Some(ws::spawn_ws_listener(ws_url, NETWORK_RSC.into(), tx))
    }

    pub async fn health(&self) -> BlockchainResult<NodeHealth> {
        let latest_block = self.rsc.latest_block_number().await.ok();
        let syncing = self.rsc.syncing().await.unwrap_or(false);
        let chain_id = match self.config.rsc_chain_id {
            Some(id) => Some(id),
            None => self.rsc.chain_id().await.ok(),
        };

        Ok(NodeHealth {
            rpc_url: self.rsc.endpoint().to_string(),
            ws_configured: self.config.rsc_ws_url.is_some(),
            chain_id,
            latest_block,
            syncing,
        })
    }

    pub async fn get_balance(&self, address: &str) -> BlockchainResult<OnChainBalance> {
        validate_address(address)?;
        let balance_wei = self.rsc.get_balance_wei(address, "latest").await?;
        let block_number = self.rsc.latest_block_number().await.unwrap_or(0);

        Ok(OnChainBalance {
            address: address.to_string(),
            balance_wei,
            block_number,
        })
    }

    pub async fn get_transaction(&self, tx_hash: &str) -> BlockchainResult<OnChainTransaction> {
        validate_tx_hash(tx_hash)?;
        self.rsc.get_transaction(tx_hash).await
    }

    pub async fn get_latest_block(&self) -> BlockchainResult<OnChainBlock> {
        self.rsc.get_latest_block().await
    }

    pub async fn get_block(&self, number: u64) -> BlockchainResult<OnChainBlock> {
        self.rsc.get_block_by_number(number).await
    }

    pub async fn broadcast_transaction(
        &self,
        req: BroadcastTxRequest,
    ) -> BlockchainResult<BroadcastTxResponse> {
        req.validate()
            .map_err(|e| BlockchainError::Validation(e.to_string()))?;

        let raw = normalize_raw_tx(&req.raw_tx_hex)?;
        if let Some(expected) = self.config.rsc_chain_id {
            let actual = self.rsc.chain_id().await?;
            if actual != expected {
                return Err(BlockchainError::Validation(format!(
                    "chain id mismatch: node={actual}, expected={expected}"
                )));
            }
        }

        let tx_hash = self
            .rsc
            .send_raw_transaction(&raw)
            .await
            .map_err(|e| match e {
                BlockchainError::Rpc(msg) => BlockchainError::BroadcastRejected(msg),
                other => other,
            })?;

        Ok(BroadcastTxResponse { tx_hash })
    }

    /// Extract block number from a `newHeads` WebSocket payload.
    pub fn block_number_from_event(event: &BlockchainEvent) -> Option<u64> {
        if event.event_type != BlockchainEventType::NewHead {
            return None;
        }
        let number_hex = event
            .payload
            .get("result")
            .and_then(|r| r.get("number"))
            .and_then(|n| n.as_str())?;
        hex_to_u64(number_hex).ok()
    }

    pub async fn latest_block_number(&self) -> BlockchainResult<u64> {
        self.rsc.latest_block_number().await
    }

    pub async fn get_block_transfers(
        &self,
        block_number: u64,
    ) -> BlockchainResult<Vec<BlockTransfer>> {
        self.rsc.get_block_transfers(block_number).await
    }

    pub fn deposit_min_confirmations(&self) -> u64 {
        self.config.deposit_min_confirmations
    }
}

#[derive(Clone)]
pub struct BlockchainServiceHandle(pub Arc<BlockchainService>);

impl BlockchainServiceHandle {
    pub fn new(http: Client, config: BlockchainConfig) -> Self {
        Self(Arc::new(BlockchainService::new(http, config)))
    }

    pub fn with_event_channel(
        http: Client,
        config: BlockchainConfig,
        tx: mpsc::Sender<BlockchainEvent>,
    ) -> Self {
        Self(Arc::new(
            BlockchainService::new(http, config).with_event_channel(tx),
        ))
    }
}

impl std::ops::Deref for BlockchainServiceHandle {
    type Target = BlockchainService;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl BlockchainServiceHandle {
    pub fn spawn_event_listener(&self) -> Option<tokio::task::JoinHandle<()>> {
        self.0.spawn_event_listener()
    }
}

fn validate_address(address: &str) -> BlockchainResult<()> {
    let a = address.trim();
    if a.len() >= 42 && a.starts_with("0x") && a[2..].chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(());
    }
    Err(BlockchainError::InvalidAddress)
}

fn validate_tx_hash(hash: &str) -> BlockchainResult<()> {
    let h = hash.trim();
    if h.len() >= 10 && h.starts_with("0x") && h[2..].chars().all(|c| c.is_ascii_hexdigit()) {
        return Ok(());
    }
    Err(BlockchainError::InvalidTxHash)
}

fn normalize_raw_tx(raw: &str) -> BlockchainResult<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(BlockchainError::Validation(
            "raw_tx_hex cannot be empty".into(),
        ));
    }
    if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
        Ok(trimmed.to_string())
    } else {
        Ok(format!("0x{trimmed}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_raw_tx_prefix() {
        assert_eq!(normalize_raw_tx("abc").unwrap(), "0xabc");
        assert_eq!(normalize_raw_tx("0xabc").unwrap(), "0xabc");
    }

    #[test]
    fn validates_address() {
        assert!(validate_address("0x1234567890123456789012345678901234567890").is_ok());
        assert!(validate_address("bad").is_err());
    }

    #[test]
    fn parses_new_head_block_number() {
        use crate::internal::blockchain::models::{BlockchainEvent, BlockchainEventType};
        use chrono::Utc;

        let event = BlockchainEvent {
            network: "rsc-mainnet".into(),
            event_type: BlockchainEventType::NewHead,
            payload: serde_json::json!({
                "result": { "number": "0x64" }
            }),
            received_at: Utc::now(),
        };
        assert_eq!(
            BlockchainService::block_number_from_event(&event),
            Some(100)
        );
    }
}
