//! Deposit worker — listens to blockchain `newHeads` and credits confirmed deposits.
//!
//! Failed operations are enqueued in Redis with exponential backoff; exhausted
//! retries land in the dead-letter queue (Redis + PostgreSQL audit).

use std::str::FromStr;

use rust_decimal::Decimal;
use serde_json::Value;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::internal::blockchain::services::BlockchainService;
use crate::internal::blockchain::{BlockchainEvent, BlockchainEventType, BlockchainServiceHandle};
use crate::internal::config::{BlockchainConfig, WorkerConfig};
use crate::internal::wallets::models::RecordDepositRequest;
use crate::internal::wallets::services::WalletServiceHandle;
use crate::internal::wallets::WalletError;

use super::error::{WorkerError, WorkerResult};
use super::job::{kinds, queues};
use super::queue::RetryQueue;

#[derive(Clone)]
pub struct DepositWorkerDeps {
    pub blockchain: BlockchainServiceHandle,
    pub wallets: WalletServiceHandle,
    pub queue: RetryQueue,
    pub blockchain_config: BlockchainConfig,
    pub worker_config: WorkerConfig,
}

pub fn spawn_deposit_worker(
    mut events: mpsc::Receiver<BlockchainEvent>,
    deps: DepositWorkerDeps,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        if !deps.blockchain_config.deposit_worker_enabled {
            info!("deposit worker disabled (DEPOSIT_WORKER_ENABLED=false)");
            return;
        }

        let min_conf = deps.blockchain_config.deposit_min_confirmations.max(1);
        info!(min_confirmations = min_conf, "deposit worker started");

        while let Some(event) = events.recv().await {
            if event.event_type != BlockchainEventType::NewHead {
                continue;
            }

            if let Err(e) = process_new_head(&deps, min_conf, &event).await {
                error!(error = %e, "deposit worker failed processing new head; enqueueing retry");
                let latest = BlockchainService::block_number_from_event(&event).unwrap_or(0);
                let payload = serde_json::json!({
                    "latest_block": latest,
                    "network": event.network,
                    "min_confirmations": min_conf,
                });
                if let Err(qe) = deps
                    .queue
                    .enqueue(queues::DEPOSIT, kinds::DEPOSIT_PROCESS_HEAD, payload)
                    .await
                {
                    error!(error = %qe, "failed to enqueue deposit head retry job");
                }
            }
        }

        warn!("deposit worker stopped: blockchain event channel closed");
    })
}

/// Retry handler: re-process a block window from a queued job payload.
pub async fn process_head_job(deps: &DepositWorkerDeps, payload: &Value) -> WorkerResult<()> {
    let latest = payload
        .get("latest_block")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| WorkerError::Processing("missing latest_block".into()))?;
    let network = payload
        .get("network")
        .and_then(|v| v.as_str())
        .unwrap_or("rsc-mainnet");
    let min_conf = payload
        .get("min_confirmations")
        .and_then(|v| v.as_u64())
        .unwrap_or(deps.blockchain_config.deposit_min_confirmations.max(1));

    process_block_window(deps, min_conf, latest, network)
        .await
        .map_err(|e| WorkerError::Processing(e.to_string()))
}

/// Retry handler: record a single deposit transfer.
pub async fn process_record_job(deps: &DepositWorkerDeps, payload: &Value) -> WorkerResult<()> {
    let req: RecordDepositRequest = serde_json::from_value(payload.clone())
        .map_err(|e| WorkerError::Processing(format!("invalid deposit payload: {e}")))?;
    record_deposit(deps, req).await
}

async fn process_new_head(
    deps: &DepositWorkerDeps,
    min_confirmations: u64,
    event: &BlockchainEvent,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let latest = match BlockchainService::block_number_from_event(event) {
        Some(n) => n,
        None => deps.blockchain.latest_block_number().await?,
    };

    process_block_window(deps, min_confirmations, latest, &event.network).await
}

async fn process_block_window(
    deps: &DepositWorkerDeps,
    min_confirmations: u64,
    latest: u64,
    network: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if latest == 0 {
        return Ok(());
    }

    let window_start = latest.saturating_sub(min_confirmations.saturating_sub(1));

    for block_number in window_start..=latest {
        let transfers = deps.blockchain.get_block_transfers(block_number).await?;
        let confirmations = latest.saturating_sub(block_number) + 1;

        if confirmations < min_confirmations {
            continue;
        }

        for transfer in transfers {
            let Some(target) = deps
                .wallets
                .find_deposit_target_by_address(&transfer.to)
                .await?
            else {
                continue;
            };

            let amount =
                Decimal::from_str(&transfer.value_wei).map_err(|_| "invalid wei amount")?;
            if amount <= Decimal::ZERO {
                continue;
            }

            let req = RecordDepositRequest {
                wallet_id: target.wallet_id,
                asset: target.asset,
                chain: target.chain,
                tx_hash: transfer.tx_hash.clone(),
                confirmations: confirmations.min(i32::MAX as u64) as i32,
                idempotency_key: transfer.tx_hash.clone(),
                amount,
                from_address: transfer.from.clone(),
                to_address: Some(transfer.to.clone()),
                metadata: Some(serde_json::json!({
                    "source": "deposit_worker",
                    "block_number": block_number,
                    "network": network,
                })),
            };

            if let Err(e) = try_record_deposit(deps, &req).await {
                if is_retryable_wallet_error(&e) {
                    error!(
                        tx_hash = %transfer.tx_hash,
                        error = %e,
                        "failed to record deposit; enqueueing retry"
                    );
                    let payload = serde_json::to_value(&req)?;
                    if let Err(qe) = deps
                        .queue
                        .enqueue(queues::DEPOSIT, kinds::DEPOSIT_RECORD, payload)
                        .await
                    {
                        error!(error = %qe, "failed to enqueue deposit record retry");
                    }
                } else {
                    error!(
                        tx_hash = %transfer.tx_hash,
                        error = %e,
                        "non-retryable deposit failure"
                    );
                }
            }
        }
    }

    Ok(())
}

async fn record_deposit(deps: &DepositWorkerDeps, req: RecordDepositRequest) -> WorkerResult<()> {
    try_record_deposit(deps, &req)
        .await
        .map_err(|e| WorkerError::Processing(e.to_string()))
}

async fn try_record_deposit(
    deps: &DepositWorkerDeps,
    req: &RecordDepositRequest,
) -> Result<(), WalletError> {
    match deps.wallets.record_confirmed_deposit(req.clone()).await {
        Ok(resp) if resp.idempotent_replay => {
            tracing::debug!(
                tx_hash = %req.tx_hash,
                wallet_id = %req.wallet_id,
                "deposit already recorded"
            );
            Ok(())
        }
        Ok(resp) => {
            info!(
                tx_hash = %req.tx_hash,
                wallet_id = %req.wallet_id,
                amount = %resp.transaction.amount,
                asset = %resp.balance.asset.0,
                "deposit credited"
            );
            Ok(())
        }
        Err(WalletError::IdempotencyConflict) => Ok(()),
        Err(e) => Err(e),
    }
}

fn is_retryable_wallet_error(err: &WalletError) -> bool {
    matches!(
        err,
        WalletError::Database(_) | WalletError::Internal(_)
    )
}
