//! Redis-backed threat intel store with in-memory cache for fast classification.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Utc};
use ipnet::IpNet;
use redis::aio::ConnectionManager;
use reqwest::Client;
use tokio::sync::RwLock;
use tracing::info;

use crate::internal::config::ThreatIntelConfig;

use super::feeds;
use super::IpClass;

const REDIS_TOR_KEY: &str = "security:intel:tor";
const REDIS_DATACENTER_KEY: &str = "security:intel:datacenter";

#[derive(Debug, Clone, Default)]
struct IntelCache {
    tor_exits: HashSet<String>,
    datacenter_nets: Vec<IpNet>,
    tor_updated_at: Option<DateTime<Utc>>,
    datacenter_updated_at: Option<DateTime<Utc>>,
}

/// Live threat-intel store — refreshed from external feeds, cached in Redis + memory.
#[derive(Clone)]
pub struct ThreatIntelStore {
    cache: Arc<RwLock<IntelCache>>,
    redis: ConnectionManager,
    config: ThreatIntelConfig,
}

impl ThreatIntelStore {
    pub fn new(redis: ConnectionManager, config: ThreatIntelConfig) -> Self {
        Self {
            cache: Arc::new(RwLock::new(IntelCache::default())),
            redis,
            config,
        }
    }

    /// Classify an IP using the latest cached threat-intel data.
    pub async fn classify(&self, ip: &str) -> IpClass {
        let Ok(addr) = ip.parse::<std::net::IpAddr>() else {
            return IpClass::Public;
        };
        if super::is_private(&addr) {
            return IpClass::Private;
        }

        if !self.config.enabled {
            return IpClass::Public;
        }

        let cache = self.cache.read().await;
        if cache.tor_exits.contains(ip) {
            return IpClass::TorExitNode;
        }
        for net in &cache.datacenter_nets {
            if net.contains(&addr) {
                return IpClass::Datacenter;
            }
        }
        IpClass::Public
    }

    /// Load cached data from Redis (best-effort on startup).
    pub async fn hydrate_from_redis(&self) {
        if let Ok(exits) = self.load_tor_from_redis().await {
            if !exits.is_empty() {
                let mut cache = self.cache.write().await;
                cache.tor_exits = exits;
                info!(count = cache.tor_exits.len(), "hydrated Tor exits from Redis");
            }
        }
        if let Ok(nets) = self.load_datacenter_from_redis().await {
            if !nets.is_empty() {
                let mut cache = self.cache.write().await;
                cache.datacenter_nets = nets;
                info!(
                    count = cache.datacenter_nets.len(),
                    "hydrated datacenter CIDRs from Redis"
                );
            }
        }
    }

    /// Refresh Tor exit list from feed URL.
    pub async fn refresh_tor(&self, http: &Client) -> anyhow::Result<usize> {
        if !self.config.tor_feed_enabled {
            return Ok(0);
        }

        let exits = feeds::tor::fetch_tor_exits(http, &self.config.tor_feed_url).await?;
        self.persist_tor_to_redis(&exits).await?;

        let count = exits.len();
        {
            let mut cache = self.cache.write().await;
            cache.tor_exits = exits;
            cache.tor_updated_at = Some(Utc::now());
        }

        info!(count, url = %self.config.tor_feed_url, "Tor exit feed refreshed");
        Ok(count)
    }

    /// Refresh datacenter CIDR netset from feed URL.
    pub async fn refresh_datacenter(&self, http: &Client) -> anyhow::Result<usize> {
        if !self.config.datacenter_feed_enabled {
            return Ok(0);
        }

        let nets =
            feeds::datacenter::fetch_datacenter_nets(http, &self.config.datacenter_feed_url)
                .await?;
        self.persist_datacenter_to_redis(&nets).await?;

        let count = nets.len();
        {
            let mut cache = self.cache.write().await;
            cache.datacenter_nets = nets;
            cache.datacenter_updated_at = Some(Utc::now());
        }

        info!(
            count,
            url = %self.config.datacenter_feed_url,
            "datacenter feed refreshed"
        );
        Ok(count)
    }

    pub async fn stats(&self) -> (usize, usize) {
        let cache = self.cache.read().await;
        (cache.tor_exits.len(), cache.datacenter_nets.len())
    }

    async fn persist_tor_to_redis(&self, exits: &HashSet<String>) -> anyhow::Result<()> {
        if exits.is_empty() {
            return Ok(());
        }
        let mut conn = self.redis.clone();
        let _: () = redis::pipe()
            .atomic()
            .cmd("DEL")
            .arg(REDIS_TOR_KEY)
            .ignore()
            .cmd("SADD")
            .arg(REDIS_TOR_KEY)
            .arg(exits.iter().collect::<Vec<_>>())
            .ignore()
            .query_async(&mut conn)
            .await?;
        Ok(())
    }

    async fn persist_datacenter_to_redis(&self, nets: &[IpNet]) -> anyhow::Result<()> {
        let payload: Vec<String> = nets.iter().map(|n| n.to_string()).collect();
        let json = serde_json::to_string(&payload)?;
        let mut conn = self.redis.clone();
        let _: () = redis::cmd("SET")
            .arg(REDIS_DATACENTER_KEY)
            .arg(json)
            .query_async(&mut conn)
            .await?;
        Ok(())
    }

    async fn load_tor_from_redis(&self) -> anyhow::Result<HashSet<String>> {
        let mut conn = self.redis.clone();
        let members: Vec<String> = redis::cmd("SMEMBERS")
            .arg(REDIS_TOR_KEY)
            .query_async(&mut conn)
            .await?;
        Ok(members.into_iter().collect())
    }

    async fn load_datacenter_from_redis(&self) -> anyhow::Result<Vec<IpNet>> {
        let mut conn = self.redis.clone();
        let json: Option<String> = redis::cmd("GET")
            .arg(REDIS_DATACENTER_KEY)
            .query_async(&mut conn)
            .await?;
        let Some(json) = json else {
            return Ok(vec![]);
        };
        let lines: Vec<String> = serde_json::from_str(&json)?;
        Ok(lines
            .iter()
            .filter_map(|s| s.parse::<IpNet>().ok())
            .collect())
    }
}

/// Build an HTTP client tuned for feed downloads.
pub fn feed_http_client(timeout_secs: u64) -> anyhow::Result<Client> {
    Ok(Client::builder()
        .user_agent(format!("rsc-gateway-threat-intel/{}", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(timeout_secs.max(5)))
        .build()?)
}
