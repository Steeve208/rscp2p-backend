//! Crypto order sync worker — polls provider for pending orders.

use std::time::Duration;
use tokio::time::interval;

use crate::internal::core::financial_gateway::FinancialGatewayHandle;

const SYNC_INTERVAL_SECS: u64 = 300;

pub fn spawn_crypto_sync_worker(gateway: FinancialGatewayHandle) {
    tokio::spawn(async move {
        let mut ticker = interval(Duration::from_secs(SYNC_INTERVAL_SECS));
        loop {
            ticker.tick().await;
            match gateway.sync_crypto_orders().await {
                Ok(count) => {
                    tracing::info!(orders_synced = count, "crypto order sync completed");
                }
                Err(e) => {
                    tracing::error!(error = %e, "crypto order sync failed");
                }
            }
        }
    });
}
