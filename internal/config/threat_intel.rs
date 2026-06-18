//! Threat intelligence feed configuration.

use crate::internal::config::{env, error::ConfigError};

/// Default Tor Project bulk exit list.
pub const DEFAULT_TOR_FEED_URL: &str = "https://check.torproject.org/torbulkexitlist";

/// Default datacenter/hosting CIDR feed (FireHOL netset — one CIDR per line).
pub const DEFAULT_DATACENTER_FEED_URL: &str =
    "https://raw.githubusercontent.com/firehol/blocklist-ipsets/master/datacenter.netset";

#[derive(Debug, Clone)]
pub struct ThreatIntelConfig {
    /// Master switch for background feed refresh + live classification.
    pub enabled: bool,
    /// Fetch and classify known Tor exit nodes.
    pub tor_feed_enabled: bool,
    pub tor_feed_url: String,
    /// Fetch and classify datacenter/hosting CIDR ranges.
    pub datacenter_feed_enabled: bool,
    pub datacenter_feed_url: String,
    /// How often to refresh feeds (default: 3600 s = 1 h).
    pub refresh_interval_secs: u64,
    /// HTTP timeout when downloading feeds (default: 60 s).
    pub http_timeout_secs: u64,
}

impl ThreatIntelConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            enabled: env::bool("THREAT_INTEL_ENABLED", true)?,
            tor_feed_enabled: env::bool("THREAT_INTEL_TOR_ENABLED", true)?,
            tor_feed_url: env::with_default("THREAT_INTEL_TOR_FEED_URL", DEFAULT_TOR_FEED_URL),
            datacenter_feed_enabled: env::bool("THREAT_INTEL_DATACENTER_ENABLED", true)?,
            datacenter_feed_url: env::with_default(
                "THREAT_INTEL_DATACENTER_FEED_URL",
                DEFAULT_DATACENTER_FEED_URL,
            ),
            refresh_interval_secs: env::u64("THREAT_INTEL_REFRESH_INTERVAL_SECS", "3600")?,
            http_timeout_secs: env::u64("THREAT_INTEL_HTTP_TIMEOUT_SECS", "60")?,
        })
    }
}
