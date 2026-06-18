//! RSC blockchain / node configuration.

use crate::internal::config::{env, error::ConfigError};

#[derive(Debug, Clone)]
pub struct BlockchainConfig {
    /// JSON-RPC HTTP endpoint (default: `http://127.0.0.1:8545`).
    pub rsc_rpc_url: String,
    /// WebSocket endpoint for event subscriptions (`RSC_WS_URL`).
    pub rsc_ws_url: Option<String>,
    /// Optional chain ID for sanity checks when broadcasting (`RSC_CHAIN_ID`).
    pub rsc_chain_id: Option<u64>,
    /// Timeout for outbound RPC/WS calls in seconds (default: `30`).
    pub rpc_timeout_secs: u64,
    /// Minimum block confirmations before crediting a deposit (default: `12`).
    pub deposit_min_confirmations: u64,
    /// Enable the deposit event worker (default: `true`).
    pub deposit_worker_enabled: bool,
}

impl BlockchainConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            rsc_rpc_url: env::with_default("RSC_RPC_URL", "http://127.0.0.1:8545"),
            rsc_ws_url: env::optional("RSC_WS_URL"),
            rsc_chain_id: env::optional_u64("RSC_CHAIN_ID")?,
            rpc_timeout_secs: env::u64("RSC_RPC_TIMEOUT_SECS", "30")?,
            deposit_min_confirmations: env::u64("DEPOSIT_MIN_CONFIRMATIONS", "12")?,
            deposit_worker_enabled: env::bool("DEPOSIT_WORKER_ENABLED", true)?,
        })
    }
}
