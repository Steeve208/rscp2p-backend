//! Global application state injected into Axum handlers.

use std::sync::Arc;
use std::time::{Duration, Instant};

use redis::aio::ConnectionManager;
use reqwest::Client;
use sqlx::PgPool;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::internal::auth::AuthService;
use crate::internal::auth::JwtConfigWrapper;
use crate::internal::blockchain::{BlockchainEvent, BlockchainServiceHandle};
use crate::internal::config::Config;
use crate::internal::core::financial_gateway::FinancialGatewayHandle;
use crate::internal::database;
use crate::internal::payments::PaymentServiceHandle;
use crate::internal::providers::FiatConversionServiceHandle;
use crate::internal::swaps::SwapServiceHandle;
use crate::internal::redis as redis_store;
use crate::internal::users::UserServiceHandle;
use crate::internal::wallets::services::WalletServiceHandle;
use crate::internal::security::ip_intel::{refresher, ThreatIntelStore};
use crate::internal::workers::{
    spawn_card_sync_worker, spawn_crypto_sync_worker, spawn_deposit_worker, spawn_queue_processor, spawn_withdrawal_worker,
    DepositWorkerDeps, RetryQueue, WithdrawalWorkerDeps,
};

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: PgPool,
    pub redis: ConnectionManager,
    pub http: Client,
    pub auth: AuthService,
    pub users: UserServiceHandle,
    pub wallets: WalletServiceHandle,
    pub blockchain: BlockchainServiceHandle,
    pub payments: PaymentServiceHandle,
    pub fiat: FiatConversionServiceHandle,
    pub financial_gateway: FinancialGatewayHandle,
    pub swaps: SwapServiceHandle,
    /// Threat-intel store (Tor exits + datacenter CIDRs) for fraud detection.
    pub threat_intel: ThreatIntelStore,
    /// Holds customer payments until merchant settlement.
    pub clearing_wallet_id: Uuid,
    pub started_at: Instant,
}

impl AppState {
    pub async fn build(config: Config) -> anyhow::Result<Self> {
        let config = Arc::new(config);

        let db = database::connect(config.database_url(), config.db_max_connections()).await?;
        database::migrate(&db).await?;
        let redis = redis_store::connect(config.redis_url()).await?;

        let jwt = Arc::new(JwtConfigWrapper::new(&config.jwt));

        let http = Client::builder()
            .user_agent(format!("rsc-gateway/{}", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(config.request_timeout_secs()))
            .build()?;

        let blockchain_http = Client::builder()
            .user_agent(format!("rsc-gateway/{}", env!("CARGO_PKG_VERSION")))
            .timeout(Duration::from_secs(config.blockchain.rpc_timeout_secs))
            .build()?;

        let users = UserServiceHandle::new(db.clone(), redis.clone());
        let wallets = WalletServiceHandle::new(db.clone(), redis.clone());
        let clearing_wallet_id = wallets.ensure_clearing_wallet().await?;
        tracing::info!(%clearing_wallet_id, "settlement clearing wallet ready");

        let fiat = FiatConversionServiceHandle::new(
            db.clone(),
            http.clone(),
            config.clone(),
            wallets.clone(),
            clearing_wallet_id,
        );
        let financial_gateway =
            FinancialGatewayHandle::new(db.clone(), http.clone(), config.clone(), fiat.clone());
        let auth = AuthService::new(
            db.clone(),
            redis.clone(),
            jwt,
            config.auth.clone(),
        )
        .with_financial_gateway(financial_gateway.clone());

        let payments = PaymentServiceHandle::new(
            db.clone(),
            redis.clone(),
            wallets.clone(),
            clearing_wallet_id,
        );
        let swaps = SwapServiceHandle::new(db.clone(), config.swaps.clone(), http.clone());

        let worker_queue = RetryQueue::new(
            redis.clone(),
            db.clone(),
            config.workers.clone(),
        );

        let threat_intel = ThreatIntelStore::new(redis.clone(), config.threat_intel.clone());
        if config.threat_intel.enabled {
            let refresher = refresher::spawn_refresher(
                threat_intel.clone(),
                config.threat_intel.clone(),
            );
            tokio::spawn(async move {
                if let Err(e) = refresher.await {
                    tracing::error!(error = ?e, "threat intel refresher panicked");
                }
            });
        }

        let (event_tx, event_rx) = mpsc::channel::<BlockchainEvent>(512);
        let blockchain = BlockchainServiceHandle::with_event_channel(
            blockchain_http,
            config.blockchain.clone(),
            event_tx,
        );

        let deposit_deps = DepositWorkerDeps {
            blockchain: blockchain.clone(),
            wallets: wallets.clone(),
            queue: worker_queue.clone(),
            blockchain_config: config.blockchain.clone(),
            worker_config: config.workers.clone(),
        };

        let withdrawal_deps = WithdrawalWorkerDeps {
            blockchain: blockchain.clone(),
            wallets: wallets.clone(),
            queue: worker_queue.clone(),
            blockchain_config: config.blockchain.clone(),
            worker_config: config.workers.clone(),
        };

        let deposit_worker = spawn_deposit_worker(event_rx, deposit_deps);
        let withdrawal_worker = spawn_withdrawal_worker(withdrawal_deps);
        let queue_processor = spawn_queue_processor(
            worker_queue,
            config.workers.clone(),
            config.blockchain.clone(),
            blockchain.clone(),
            wallets.clone(),
        );

        if config.blockchain.rsc_ws_url.is_some() {
            if let Some(_ws_handle) = blockchain.spawn_event_listener() {
                tracing::info!("blockchain websocket listener started");
            }
        } else {
            tracing::warn!("RSC_WS_URL not set; deposit worker will not receive newHead events");
        }

        for (name, handle) in [
            ("deposit", deposit_worker),
            ("withdrawal", withdrawal_worker),
            ("queue_processor", queue_processor),
        ] {
            tokio::spawn(async move {
                if let Err(e) = handle.await {
                    tracing::error!(worker = name, error = ?e, "worker task panicked");
                }
            });
        }

        spawn_card_sync_worker(financial_gateway.clone());
        spawn_crypto_sync_worker(financial_gateway.clone());

        Ok(Self {
            config,
            db,
            redis,
            http,
            auth,
            users,
            wallets,
            blockchain,
            payments,
            fiat,
            financial_gateway,
            swaps,
            threat_intel,
            clearing_wallet_id,
            started_at: Instant::now(),
        })
    }

    pub fn uptime_secs(&self) -> u64 {
        self.started_at.elapsed().as_secs()
    }
}
