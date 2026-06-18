//! Retry-queue processor — polls Redis and dispatches due jobs to handlers.

use std::time::Duration;

use tracing::{error, info, warn};

use crate::internal::blockchain::BlockchainServiceHandle;
use crate::internal::config::{BlockchainConfig, WorkerConfig};
use crate::internal::wallets::services::WalletServiceHandle;

use super::deposit::{self, DepositWorkerDeps};
use super::error::WorkerError;
use super::job::{kinds, queues};
use super::queue::RetryQueue;
use super::withdrawal::{self, WithdrawalWorkerDeps};

pub fn spawn_queue_processor(
    queue: RetryQueue,
    worker_config: WorkerConfig,
    blockchain_config: BlockchainConfig,
    blockchain: BlockchainServiceHandle,
    wallets: WalletServiceHandle,
) -> tokio::task::JoinHandle<()> {
    let deposit_deps = DepositWorkerDeps {
        blockchain: blockchain.clone(),
        wallets: wallets.clone(),
        queue: queue.clone(),
        blockchain_config: blockchain_config.clone(),
        worker_config: worker_config.clone(),
    };
    let withdrawal_deps = WithdrawalWorkerDeps {
        blockchain,
        wallets,
        queue: queue.clone(),
        blockchain_config,
        worker_config: worker_config.clone(),
    };

    tokio::spawn(async move {
        let interval_secs = worker_config.queue_poll_interval_secs.max(1);
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        info!(interval_secs, "worker retry-queue processor started");

        loop {
            interval.tick().await;
            if let Err(e) = poll_queue(&queue, &deposit_deps, &withdrawal_deps, queues::DEPOSIT).await
            {
                error!(queue = queues::DEPOSIT, error = %e, "deposit queue poll failed");
            }
            if let Err(e) =
                poll_queue(&queue, &deposit_deps, &withdrawal_deps, queues::WITHDRAWAL).await
            {
                error!(queue = queues::WITHDRAWAL, error = %e, "withdrawal queue poll failed");
            }
        }
    })
}

async fn poll_queue(
    queue: &RetryQueue,
    deposit_deps: &DepositWorkerDeps,
    withdrawal_deps: &WithdrawalWorkerDeps,
    queue_name: &str,
) -> Result<(), WorkerError> {
    let jobs = queue.dequeue_due(queue_name, 32).await?;
    for job in jobs {
        let result = match (queue_name, job.kind.as_str()) {
            (queues::DEPOSIT, kinds::DEPOSIT_PROCESS_HEAD) => {
                deposit::process_head_job(deposit_deps, &job.payload).await
            }
            (queues::DEPOSIT, kinds::DEPOSIT_RECORD) => {
                deposit::process_record_job(deposit_deps, &job.payload).await
            }
            (queues::WITHDRAWAL, kinds::WITHDRAWAL_CONFIRM) => {
                withdrawal::process_confirm_job(withdrawal_deps, &job.payload).await
            }
            (_, kind) => Err(WorkerError::UnknownKind(kind.to_string())),
        };

        match result {
            Ok(()) => {
                queue.ack(&job.id).await?;
            }
            Err(e) => {
                warn!(
                    job_id = %job.id,
                    kind = %job.kind,
                    error = %e,
                    "worker job failed"
                );
                queue.nack(job, &e.to_string()).await?;
            }
        }
    }
    Ok(())
}
