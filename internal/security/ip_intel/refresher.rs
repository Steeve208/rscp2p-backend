//! Background task — periodically refreshes Tor + datacenter threat feeds.

use std::time::Duration;

use reqwest::Client;
use tracing::{error, info, warn};

use crate::internal::config::ThreatIntelConfig;

use super::store::{feed_http_client, ThreatIntelStore};

pub fn spawn_refresher(
    store: ThreatIntelStore,
    config: ThreatIntelConfig,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        if !config.enabled {
            info!("threat intel refresher disabled (THREAT_INTEL_ENABLED=false)");
            return;
        }

        let http = match feed_http_client(config.http_timeout_secs) {
            Ok(c) => c,
            Err(e) => {
                error!(error = %e, "failed to build threat-intel HTTP client");
                return;
            }
        };

        store.hydrate_from_redis().await;

        if let Err(e) = refresh_all(&store, &http, &config).await {
            warn!(error = %e, "initial threat-intel refresh failed; will retry on interval");
        }

        let interval_secs = config.refresh_interval_secs.max(60);
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        info!(interval_secs, "threat intel refresher started");

        loop {
            interval.tick().await;
            if let Err(e) = refresh_all(&store, &http, &config).await {
                error!(error = %e, "threat-intel refresh failed");
            }
        }
    })
}

async fn refresh_all(
    store: &ThreatIntelStore,
    http: &Client,
    config: &ThreatIntelConfig,
) -> anyhow::Result<()> {
    let mut tor_count = 0usize;
    let mut dc_count = 0usize;

    if config.tor_feed_enabled {
        tor_count = store.refresh_tor(http).await?;
    }
    if config.datacenter_feed_enabled {
        dc_count = store.refresh_datacenter(http).await?;
    }

    info!(tor_count, dc_count, "threat-intel feeds up to date");
    Ok(())
}
