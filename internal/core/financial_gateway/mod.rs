//! Financial Gateway — unified facade hiding Striga and Transak from RSC App.
//!
//! ```text
//! RSC App → Financial Gateway → Striga Connector | Transak Connector
//! ```

mod card_service;
mod crypto_repository;
mod crypto_service;
mod error;
mod kyc_service;
mod models;
mod user_service;

pub mod handlers;

use std::sync::Arc;

use reqwest::Client;
use sqlx::PgPool;
use uuid::Uuid;

use crate::internal::config::Config;
use crate::internal::providers::striga::api::StrigaApiClient;
use crate::internal::providers::striga::repository::StrigaRepository;
use crate::internal::providers::FiatConversionServiceHandle;

pub use error::{GatewayError, GatewayResult};
pub use models::*;

use crypto_repository::CryptoOrderRepository;
use card_service::CardService;
use crypto_service::CryptoService;
use kyc_service::KycService;
use user_service::UserService;

#[derive(Clone)]
pub struct FinancialGatewayHandle {
    inner: Arc<FinancialGatewayInner>,
}

struct FinancialGatewayInner {
    db: PgPool,
    config: Arc<Config>,
    striga_client: StrigaApiClient,
    striga_repo: StrigaRepository,
    fiat: FiatConversionServiceHandle,
    users: UserService,
    kyc: KycService,
    cards: CardService,
    crypto: CryptoService,
}

impl FinancialGatewayHandle {
    pub fn new(
        db: PgPool,
        http: Client,
        config: Arc<Config>,
        fiat: FiatConversionServiceHandle,
    ) -> Self {
        let striga_config = config.providers.striga.clone();
        let striga_client = StrigaApiClient::new(http.clone(), striga_config);
        let striga_repo = StrigaRepository::new(db.clone());

        let users = UserService::new(striga_client.clone(), striga_repo.clone());
        let kyc = KycService::new(striga_client.clone(), striga_repo.clone());
        let cards = CardService::new(striga_client.clone(), striga_repo.clone());
        let crypto = CryptoService::new(
            http.clone(),
            config.clone(),
            CryptoOrderRepository::new(db.clone()),
            striga_repo.clone(),
        );

        Self {
            inner: Arc::new(FinancialGatewayInner {
                db,
                config,
                striga_client,
                striga_repo,
                fiat,
                users,
                kyc,
                cards,
                crypto,
            }),
        }
    }

    // ── UserService ──────────────────────────────────────────────────────────

    pub async fn provision_banking_user(&self, user_id: Uuid, email: &str) -> GatewayResult<()> {
        self.inner.users.provision_user(user_id, email).await
    }

    pub async fn get_banking_user(&self, user_id: Uuid) -> GatewayResult<BankingUserResponse> {
        self.inner.users.get_user(user_id).await
    }

    // ── KYCService ───────────────────────────────────────────────────────────

    pub async fn start_kyc(&self, user_id: Uuid, tier: i32) -> GatewayResult<StartKycGatewayResponse> {
        self.inner.kyc.start_kyc(user_id, tier).await
    }

    pub async fn get_kyc_status(&self, user_id: Uuid) -> GatewayResult<KycStatusGatewayResponse> {
        self.inner.kyc.get_kyc_status(user_id).await
    }

    pub async fn handle_striga_webhook(
        &self,
        secret: Option<&str>,
        body: serde_json::Value,
    ) -> GatewayResult<()> {
        self.inner
            .kyc
            .handle_striga_webhook(&self.inner.config, secret, body)
            .await
    }

    // ── CardService ──────────────────────────────────────────────────────────

    pub async fn create_virtual_card(&self, user_id: Uuid) -> GatewayResult<CardGatewayResponse> {
        self.inner.cards.create_virtual_card(user_id).await
    }

    pub async fn create_physical_card(&self, user_id: Uuid) -> GatewayResult<CardGatewayResponse> {
        self.inner.cards.create_physical_card(user_id).await
    }

    pub async fn get_card(&self, user_id: Uuid, card_id: Uuid) -> GatewayResult<CardGatewayResponse> {
        self.inner.cards.get_card(user_id, card_id).await
    }

    pub async fn freeze_card(&self, user_id: Uuid, card_id: Uuid) -> GatewayResult<CardGatewayResponse> {
        self.inner.cards.freeze_card(user_id, card_id).await
    }

    pub async fn unfreeze_card(&self, user_id: Uuid, card_id: Uuid) -> GatewayResult<CardGatewayResponse> {
        self.inner.cards.unfreeze_card(user_id, card_id).await
    }

    pub async fn terminate_card(&self, user_id: Uuid, card_id: Uuid) -> GatewayResult<CardGatewayResponse> {
        self.inner.cards.terminate_card(user_id, card_id).await
    }

    pub async fn activate_physical_card(
        &self,
        user_id: Uuid,
        card_id: Uuid,
        activation_code: &str,
    ) -> GatewayResult<CardGatewayResponse> {
        self.inner
            .cards
            .activate_physical_card(user_id, card_id, activation_code)
            .await
    }

    pub async fn list_cards(&self, user_id: Uuid) -> GatewayResult<Vec<CardGatewayResponse>> {
        self.inner.cards.list_cards(user_id).await
    }

    pub async fn get_card_transactions(
        &self,
        user_id: Uuid,
        card_id: Uuid,
    ) -> GatewayResult<Vec<CardTransactionResponse>> {
        self.inner.cards.get_card_transactions(user_id, card_id).await
    }

    pub async fn sync_card_transactions(&self) -> GatewayResult<u32> {
        self.inner.cards.sync_all_transactions().await
    }

    // ── CryptoService (Transak) ────────────────────────────────────────────────

    pub async fn get_crypto_quote(
        &self,
        user_id: Uuid,
        req: CryptoQuoteRequest,
    ) -> GatewayResult<CryptoQuoteResponse> {
        let _ = user_id;
        self.inner.crypto.get_quote(req).await
    }

    pub async fn buy_crypto(
        &self,
        user_id: Uuid,
        req: CryptoTradeRequest,
    ) -> GatewayResult<CryptoWidgetResponse> {
        self.inner.crypto.buy_crypto(user_id, req).await
    }

    pub async fn sell_crypto(
        &self,
        user_id: Uuid,
        req: CryptoTradeRequest,
    ) -> GatewayResult<CryptoWidgetResponse> {
        self.inner.crypto.sell_crypto(user_id, req).await
    }

    pub async fn off_ramp(
        &self,
        user_id: Uuid,
        req: CryptoTradeRequest,
    ) -> GatewayResult<CryptoWidgetResponse> {
        self.inner.crypto.off_ramp(user_id, req).await
    }

    pub async fn get_crypto_order(
        &self,
        user_id: Uuid,
        order_id: Uuid,
    ) -> GatewayResult<CryptoOrderResponse> {
        self.inner.crypto.get_order(user_id, order_id).await
    }

    pub async fn list_crypto_orders(
        &self,
        user_id: Uuid,
    ) -> GatewayResult<Vec<CryptoOrderResponse>> {
        self.inner.crypto.list_orders(user_id).await
    }

    pub async fn handle_transak_webhook(
        &self,
        secret: Option<&str>,
        body: serde_json::Value,
    ) -> GatewayResult<()> {
        self.inner.crypto.handle_transak_webhook(secret, body).await
    }

    pub async fn sync_crypto_orders(&self) -> GatewayResult<u32> {
        self.inner.crypto.sync_pending_orders().await
    }

    pub async fn list_supported_crypto_assets(&self) -> GatewayResult<Vec<String>> {
        self.inner.crypto.get_supported_assets().await
    }

    // ── Admin ────────────────────────────────────────────────────────────────

    pub async fn admin_provider_status(&self) -> GatewayResult<ProvidersDashboardResponse> {
        let statuses = self.inner.striga_repo.get_provider_status().await?;
        let webhooks = self
            .inner
            .striga_repo
            .list_recent_webhooks(None, 20)
            .await?;

        let striga_status = statuses
            .iter()
            .find(|s| s.provider == "striga")
            .cloned();
        let transak_status = statuses
            .iter()
            .find(|s| s.provider == "transak")
            .cloned();

        Ok(ProvidersDashboardResponse {
            striga: ProviderHealth {
                configured: self.inner.striga_client.config().is_configured(),
                mock_mode: self.inner.striga_client.uses_mock(),
                status: striga_status
                    .as_ref()
                    .map(|s| s.status.clone())
                    .unwrap_or_else(|| "unknown".into()),
                last_sync_at: striga_status.as_ref().and_then(|s| s.last_sync_at),
                last_error: striga_status.and_then(|s| s.last_error),
            },
            transak: ProviderHealth {
                configured: self.inner.config.providers.transak.api_key.is_some(),
                mock_mode: self.inner.config.providers.transak.mock_mode,
                status: transak_status
                    .as_ref()
                    .map(|s| s.status.clone())
                    .unwrap_or_else(|| "unknown".into()),
                last_sync_at: transak_status.as_ref().and_then(|s| s.last_sync_at),
                last_error: transak_status.and_then(|s| s.last_error),
            },
            recent_webhooks: webhooks,
        })
    }

    pub fn striga_client(&self) -> &StrigaApiClient {
        &self.inner.striga_client
    }

    pub fn striga_repo(&self) -> &StrigaRepository {
        &self.inner.striga_repo
    }
}
