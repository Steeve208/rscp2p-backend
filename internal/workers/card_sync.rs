//! Card transaction sync worker — polls provider every 5 minutes.

use std::time::Duration;
use tokio::time::interval;

use crate::internal::core::financial_gateway::FinancialGatewayHandle;

const SYNC_INTERVAL_SECS: u64 = 300;

pub fn spawn_card_sync_worker(gateway: FinancialGatewayHandle) {
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(SYNC_INTERVAL_SECS));
        loop {
            ticker.tick().await;
            match gateway.sync_card_transactions().await {
                Ok(count) => {
                    tracing::info!(cards_synced = count, "card transaction sync completed");
                }
                Err(e) => {
                    tracing::error!(error = %e, "card transaction sync failed");
                }
            }
        }
    });
}
