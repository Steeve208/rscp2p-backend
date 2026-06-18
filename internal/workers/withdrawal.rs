//! Withdrawal worker — confirms on-chain outgoing transfers and finalizes ledger debits.
//!
//! Transient failures enqueue per-transaction retry jobs with exponential backoff.

use std::time::Duration;

use serde_json::Value;
use tracing::{error, info};
use uuid::Uuid;

use crate::internal::blockchain::models::OnChainTxState;
use crate::internal::blockchain::BlockchainServiceHandle;
use crate::internal::config::{BlockchainConfig, WorkerConfig};
use crate::internal::wallets::services::WalletServiceHandle;

use super::error::{WorkerError, WorkerResult};
use super::job::{kinds, queues};
use super::queue::RetryQueue;

#[derive(Clone)]
pub struct WithdrawalWorkerDeps {
    pub blockchain: BlockchainServiceHandle,
    pub wallets: WalletServiceHandle,
    pub queue: RetryQueue,
    pub blockchain_config: BlockchainConfig,
    pub worker_config: WorkerConfig,
}

pub fn spawn_withdrawal_worker(deps: WithdrawalWorkerDeps) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        if !deps.worker_config.withdrawal_worker_enabled {
            info!("withdrawal worker disabled (WORKER_WITHDRAWAL_ENABLED=false)");
            return;
        }

        let min_conf = deps.blockchain_config.deposit_min_confirmations.max(1);
        let interval_secs = deps.worker_config.withdrawal_poll_interval_secs.max(5);
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        info!(min_confirmations = min_conf, interval_secs, "withdrawal worker started");

        loop {
            interval.tick().await;
            if let Err(e) = process_confirming(&deps, min_conf).await {
                error!(error = %e, "withdrawal worker tick failed");
            }
        }
    })
}

async fn process_confirming(
    deps: &WithdrawalWorkerDeps,
    min_confirmations: u64,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let pending = deps.wallets.list_confirming_withdrawals(50).await?;

    for tx in pending {
        let Some(tx_hash) = tx.tx_hash.clone() else {
            continue;
        };

        if let Err(e) = confirm_withdrawal(deps, tx.id, &tx_hash, min_confirmations).await {
            if is_pending_confirm_error(&e) {
                continue;
            }
            error!(
                transaction_id = %tx.id,
                tx_hash = %tx_hash,
                error = %e,
                "withdrawal confirmation failed; enqueueing retry"
            );
            let payload = serde_json::json!({
                "transaction_id": tx.id,
                "tx_hash": tx_hash,
                "min_confirmations": min_confirmations,
            });
            if let Err(qe) = deps
                .queue
                .enqueue(queues::WITHDRAWAL, kinds::WITHDRAWAL_CONFIRM, payload)
                .await
            {
                error!(error = %qe, "failed to enqueue withdrawal retry job");
            }
        }
    }

    Ok(())
}

/// Retry handler for a single withdrawal confirmation.
pub async fn process_confirm_job(
    deps: &WithdrawalWorkerDeps,
    payload: &Value,
) -> WorkerResult<()> {
    let transaction_id = payload
        .get("transaction_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(|| WorkerError::Processing("missing transaction_id".into()))?;
    let tx_hash = payload
        .get("tx_hash")
        .and_then(|v| v.as_str())
        .ok_or_else(|| WorkerError::Processing("missing tx_hash".into()))?
        .to_string();
    let min_conf = payload
        .get("min_confirmations")
        .and_then(|v| v.as_u64())
        .unwrap_or(deps.blockchain_config.deposit_min_confirmations.max(1));

    confirm_withdrawal(deps, transaction_id, &tx_hash, min_conf).await
}

async fn confirm_withdrawal(
    deps: &WithdrawalWorkerDeps,
    transaction_id: Uuid,
    tx_hash: &str,
    min_confirmations: u64,
) -> WorkerResult<()> {
    let on_chain = deps
        .blockchain
        .get_transaction(tx_hash)
        .await
        .map_err(|e| WorkerError::Processing(e.to_string()))?;

    match on_chain.state {
        OnChainTxState::Success if on_chain.confirmations >= min_confirmations => {
            if let Some(finalized) = deps
                .wallets
                .finalize_confirmed_withdrawal(
                    transaction_id,
                    tx_hash,
                    on_chain.confirmations as i32,
                )
                .await
                .map_err(|e| WorkerError::Processing(e.to_string()))?
            {
                info!(
                    transaction_id = %finalized.id,
                    tx_hash = %tx_hash,
                    "withdrawal confirmed and finalized"
                );
            }
            Ok(())
        }
        OnChainTxState::Failed => {
            if let Some(failed) = deps
                .wallets
                .fail_withdrawal(transaction_id, tx_hash)
                .await
                .map_err(|e| WorkerError::Processing(e.to_string()))?
            {
                info!(
                    transaction_id = %failed.id,
                    tx_hash = %tx_hash,
                    "withdrawal failed on-chain; funds released"
                );
            }
            Ok(())
        }
        OnChainTxState::NotFound => {
            tracing::debug!(tx_hash = %tx_hash, "withdrawal tx not yet on chain");
            Err(WorkerError::Processing("transaction not found on chain".into()))
        }
        _ => Err(WorkerError::Processing(format!(
            "insufficient confirmations ({}/{})",
            on_chain.confirmations, min_confirmations
        ))),
    }
}

fn is_pending_confirm_error(err: &WorkerError) -> bool {
    matches!(
        err,
        WorkerError::Processing(msg)
            if msg.contains("not found") || msg.contains("insufficient confirmations")
    )
}
