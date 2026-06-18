//! RSC chain JSON-RPC client (Ethereum-compatible methods).

use serde::Deserialize;
use serde_json::json;

use crate::internal::blockchain::error::{BlockchainError, BlockchainResult};
use crate::internal::blockchain::models::{
    BlockTransfer, OnChainBlock, OnChainTransaction, OnChainTxState,
};
use crate::internal::blockchain::rpc::{hex_to_u64, hex_wei_to_string, JsonRpcClient};

#[derive(Clone)]
pub struct RscRpcClient {
    rpc: JsonRpcClient,
}

impl RscRpcClient {
    pub fn new(rpc: JsonRpcClient) -> Self {
        Self { rpc }
    }

    pub fn endpoint(&self) -> &str {
        self.rpc.endpoint()
    }

    pub async fn chain_id(&self) -> BlockchainResult<u64> {
        let hex: String = self.rpc.call("eth_chainId", json!([])).await?;
        hex_to_u64(&hex)
    }

    pub async fn latest_block_number(&self) -> BlockchainResult<u64> {
        let hex: String = self.rpc.call("eth_blockNumber", json!([])).await?;
        hex_to_u64(&hex)
    }

    pub async fn get_balance_wei(&self, address: &str, block: &str) -> BlockchainResult<String> {
        let hex: String = self
            .rpc
            .call("eth_getBalance", json!([address, block]))
            .await?;
        hex_wei_to_string(&hex)
    }

    pub async fn get_block_by_number(&self, number: u64) -> BlockchainResult<OnChainBlock> {
        let tag = format!("0x{:x}", number);
        let block: Option<RpcBlock> = self
            .rpc
            .call_optional("eth_getBlockByNumber", json!([tag, false]))
            .await?;

        let block = block.ok_or(BlockchainError::BlockNotFound)?;
        translate_block(block)
    }

    pub async fn get_latest_block(&self) -> BlockchainResult<OnChainBlock> {
        let block: Option<RpcBlock> = self
            .rpc
            .call_optional("eth_getBlockByNumber", json!(["latest", false]))
            .await?;

        let block = block.ok_or(BlockchainError::BlockNotFound)?;
        translate_block(block)
    }

    pub async fn get_transaction(&self, tx_hash: &str) -> BlockchainResult<OnChainTransaction> {
        let tx: Option<RpcTransaction> = self
            .rpc
            .call_optional("eth_getTransactionByHash", json!([tx_hash]))
            .await?;

        let receipt: Option<RpcReceipt> = self
            .rpc
            .call_optional("eth_getTransactionReceipt", json!([tx_hash]))
            .await?;

        match (tx, receipt) {
            (None, None) => Ok(OnChainTransaction {
                hash: normalize_hash(tx_hash)?,
                state: OnChainTxState::NotFound,
                block_number: None,
                confirmations: 0,
                from: None,
                to: None,
                value_wei: None,
                gas_used: None,
            }),
            (Some(tx), receipt) => {
                let latest = self.latest_block_number().await.unwrap_or(0);
                let block_number = match tx.block_number.as_ref() {
                    Some(h) => Some(hex_to_u64(h)?),
                    None => None,
                };

                let confirmations = match block_number {
                    Some(bn) if latest >= bn => latest - bn + 1,
                    _ => 0,
                };

                let state = if let Some(ref rcpt) = receipt {
                    if rcpt.status.as_deref() == Some("0x0")
                        || rcpt.status.as_deref() == Some("0x00")
                    {
                        OnChainTxState::Failed
                    } else {
                        OnChainTxState::Success
                    }
                } else {
                    OnChainTxState::Pending
                };

                Ok(OnChainTransaction {
                    hash: tx.hash.unwrap_or_else(|| tx_hash.to_string()),
                    state,
                    block_number,
                    confirmations,
                    from: tx.from,
                    to: tx.to,
                    value_wei: tx.value.map(|v| hex_wei_to_string(&v)).transpose()?,
                    gas_used: receipt
                        .and_then(|r| r.gas_used)
                        .map(|g| hex_wei_to_string(&g))
                        .transpose()?,
                })
            }
            (None, Some(_)) => Err(BlockchainError::Rpc("receipt without transaction".into())),
        }
    }

    pub async fn send_raw_transaction(&self, raw_tx_hex: &str) -> BlockchainResult<String> {
        let hash: String = self
            .rpc
            .call("eth_sendRawTransaction", json!([raw_tx_hex]))
            .await?;
        Ok(hash)
    }

    /// Full block with transaction objects (`eth_getBlockByNumber` second param = true).
    pub async fn get_block_transfers(&self, number: u64) -> BlockchainResult<Vec<BlockTransfer>> {
        let tag = format!("0x{:x}", number);
        let block: Option<RpcBlock> = self
            .rpc
            .call_optional("eth_getBlockByNumber", json!([tag, true]))
            .await?;

        let block = block.ok_or(BlockchainError::BlockNotFound)?;
        let block_number = block
            .number
            .as_ref()
            .map(|h| hex_to_u64(h))
            .transpose()?
            .unwrap_or(number);

        let mut transfers = Vec::new();
        let Some(txs) = block.transactions else {
            return Ok(transfers);
        };

        for tx_value in txs {
            let tx: RpcFullTransaction = serde_json::from_value(tx_value)
                .map_err(|e| BlockchainError::Rpc(format!("invalid block tx: {e}")))?;

            let Some(to) = tx.to.filter(|t| !t.is_empty()) else {
                continue;
            };

            let value_hex = tx.value.unwrap_or_else(|| "0x0".into());
            let value_wei = hex_wei_to_string(&value_hex)?;
            if value_wei == "0" {
                continue;
            }

            let tx_hash = tx
                .hash
                .ok_or_else(|| BlockchainError::Rpc("block transaction missing hash".into()))?;

            transfers.push(BlockTransfer {
                tx_hash,
                from: tx.from,
                to,
                value_wei,
                block_number,
            });
        }

        Ok(transfers)
    }

    pub async fn syncing(&self) -> BlockchainResult<bool> {
        let status: serde_json::Value = self.rpc.call("eth_syncing", json!([])).await?;
        if let Some(syncing) = status.as_bool() {
            Ok(syncing)
        } else {
            // Object payload means the node is actively syncing.
            Ok(true)
        }
    }
}

fn translate_block(block: RpcBlock) -> BlockchainResult<OnChainBlock> {
    let number = block
        .number
        .as_ref()
        .ok_or(BlockchainError::BlockNotFound)
        .and_then(|h| hex_to_u64(h))?;

    let hash = block
        .hash
        .ok_or_else(|| BlockchainError::Rpc("block missing hash".into()))?;

    let parent_hash = block
        .parent_hash
        .ok_or_else(|| BlockchainError::Rpc("block missing parent_hash".into()))?;

    let timestamp = block
        .timestamp
        .as_ref()
        .map(|t| hex_to_u64(t))
        .transpose()?
        .unwrap_or(0);

    let transaction_count = block.transactions.map(|t| t.len()).unwrap_or(0);

    Ok(OnChainBlock {
        number,
        hash,
        parent_hash,
        timestamp,
        transaction_count,
    })
}

fn normalize_hash(hash: &str) -> BlockchainResult<String> {
    let h = hash.trim();
    if h.len() < 10 || !h.starts_with("0x") {
        return Err(BlockchainError::InvalidTxHash);
    }
    Ok(h.to_string())
}

#[derive(Debug, Deserialize)]
struct RpcBlock {
    number: Option<String>,
    hash: Option<String>,
    parent_hash: Option<String>,
    timestamp: Option<String>,
    transactions: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
struct RpcTransaction {
    hash: Option<String>,
    block_number: Option<String>,
    from: Option<String>,
    to: Option<String>,
    value: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RpcReceipt {
    status: Option<String>,
    gas_used: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RpcFullTransaction {
    hash: Option<String>,
    from: Option<String>,
    to: Option<String>,
    value: Option<String>,
}
