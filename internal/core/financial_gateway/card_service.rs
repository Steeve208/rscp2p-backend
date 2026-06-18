use serde_json::json;
use uuid::Uuid;

use crate::internal::core::financial_gateway::error::{GatewayError, GatewayResult};
use crate::internal::core::financial_gateway::models::{CardGatewayResponse, CardTransactionResponse};
use crate::internal::providers::striga::api::StrigaApiClient;
use crate::internal::providers::striga::models::{CardType};
use crate::internal::providers::striga::repository::{CardRow, StrigaRepository};

#[derive(Clone)]
pub struct CardService {
    client: StrigaApiClient,
    repo: StrigaRepository,
}

impl CardService {
    pub fn new(client: StrigaApiClient, repo: StrigaRepository) -> Self {
        Self { client, repo }
    }

    pub async fn create_virtual_card(&self, user_id: Uuid) -> GatewayResult<CardGatewayResponse> {
        self.create_card(user_id, CardType::Virtual).await
    }

    pub async fn create_physical_card(&self, user_id: Uuid) -> GatewayResult<CardGatewayResponse> {
        self.create_card(user_id, CardType::Physical).await
    }

    async fn create_card(
        &self,
        user_id: Uuid,
        card_type: CardType,
    ) -> GatewayResult<CardGatewayResponse> {
        let striga_user_id = self.require_striga_user_id(user_id).await?;

        let card = match card_type {
            CardType::Virtual => self.client.create_virtual_card(&striga_user_id, None).await?,
            CardType::Physical => self.client.create_physical_card(&striga_user_id, None).await?,
        };

        let local_id = self
            .repo
            .insert_card(
                user_id,
                &card.id,
                card.card_type,
                card.status,
                card.last_four.as_deref(),
                card.expiry_month,
                card.expiry_year,
                &json!({}),
            )
            .await?;

        let row = self
            .repo
            .get_card_by_striga_id(&card.id)
            .await?
            .ok_or(GatewayError::NotFound)?;

        debug_assert_eq!(row.id, local_id);
        Ok(row.into())
    }

    pub async fn get_card(&self, user_id: Uuid, card_id: Uuid) -> GatewayResult<CardGatewayResponse> {
        let row = self.require_user_card(user_id, card_id).await?;

        let remote = self.client.get_card(&row.striga_card_id).await?;
        self.repo
            .update_card_status(&row.striga_card_id, remote.status)
            .await?;

        let updated = self
            .repo
            .get_card_by_striga_id(&row.striga_card_id)
            .await?
            .ok_or(GatewayError::NotFound)?;

        Ok(updated.into())
    }

    pub async fn freeze_card(&self, user_id: Uuid, card_id: Uuid) -> GatewayResult<CardGatewayResponse> {
        let row = self.require_user_card(user_id, card_id).await?;
        let striga_id = row.striga_card_id.clone();
        let remote = self.client.freeze_card(&striga_id).await?;
        self.repo
            .update_card_status(&striga_id, remote.status)
            .await?;
        self.get_card(user_id, card_id).await
    }

    pub async fn unfreeze_card(&self, user_id: Uuid, card_id: Uuid) -> GatewayResult<CardGatewayResponse> {
        let row = self.require_user_card(user_id, card_id).await?;
        let striga_id = row.striga_card_id.clone();
        let remote = self.client.unfreeze_card(&striga_id).await?;
        self.repo
            .update_card_status(&striga_id, remote.status)
            .await?;
        self.get_card(user_id, card_id).await
    }

    pub async fn terminate_card(
        &self,
        user_id: Uuid,
        card_id: Uuid,
    ) -> GatewayResult<CardGatewayResponse> {
        let row = self.require_user_card(user_id, card_id).await?;
        let striga_id = row.striga_card_id.clone();
        let remote = self.client.terminate_card(&striga_id).await?;
        self.repo
            .update_card_status(&striga_id, remote.status)
            .await?;
        self.get_card(user_id, card_id).await
    }

    pub async fn activate_physical_card(
        &self,
        user_id: Uuid,
        card_id: Uuid,
        activation_code: &str,
    ) -> GatewayResult<CardGatewayResponse> {
        let row = self.require_user_card(user_id, card_id).await?;
        let remote = self
            .client
            .activate_physical_card(&row.striga_card_id, activation_code)
            .await?;
        self.repo
            .update_card_status(&row.striga_card_id, remote.status)
            .await?;
        self.get_card(user_id, card_id).await
    }

    pub async fn list_cards(&self, user_id: Uuid) -> GatewayResult<Vec<CardGatewayResponse>> {
        let rows = self.repo.list_user_cards(user_id).await?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn get_card_transactions(
        &self,
        user_id: Uuid,
        card_id: Uuid,
    ) -> GatewayResult<Vec<CardTransactionResponse>> {
        let row = self.require_user_card(user_id, card_id).await?;
        let txs = self.client.get_card_transactions(&row.striga_card_id).await?;

        for tx in &txs {
            self.repo
                .upsert_card_transaction(
                    row.id,
                    &tx.external_id,
                    tx.amount,
                    &tx.currency,
                    &tx.direction,
                    tx.merchant_name.as_deref(),
                    &tx.status,
                    tx.transacted_at,
                    &json!({}),
                )
                .await?;
        }

        Ok(txs
            .into_iter()
            .map(|tx| CardTransactionResponse {
                external_id: tx.external_id,
                amount: tx.amount,
                currency: tx.currency,
                direction: tx.direction,
                merchant_name: tx.merchant_name,
                status: tx.status,
                transacted_at: tx.transacted_at,
            })
            .collect())
    }

    pub async fn sync_all_transactions(&self) -> GatewayResult<u32> {
        let cards = self.repo.list_cards_for_sync().await?;
        let mut synced = 0u32;

        for card in cards {
            if let Err(e) = self.sync_card(&card).await {
                tracing::warn!(card_id = %card.id, error = %e, "card transaction sync failed");
                let _ = self
                    .repo
                    .update_provider_status("striga", "degraded", Some(&e.to_string()))
                    .await;
            } else {
                synced += 1;
            }
        }

        let _ = self.repo.update_provider_status("striga", "healthy", None).await;
        Ok(synced)
    }

    async fn sync_card(&self, card: &CardRow) -> GatewayResult<()> {
        let txs = self.client.get_card_transactions(&card.striga_card_id).await?;
        for tx in txs {
            self.repo
                .upsert_card_transaction(
                    card.id,
                    &tx.external_id,
                    tx.amount,
                    &tx.currency,
                    &tx.direction,
                    tx.merchant_name.as_deref(),
                    &tx.status,
                    tx.transacted_at,
                    &json!({}),
                )
                .await?;
        }
        Ok(())
    }

    async fn require_user_card(&self, user_id: Uuid, card_id: Uuid) -> GatewayResult<CardRow> {
        let rows = self.repo.list_user_cards(user_id).await?;
        rows.into_iter()
            .find(|c| c.id == card_id)
            .ok_or(GatewayError::NotFound)
    }

    async fn require_striga_user_id(&self, user_id: Uuid) -> GatewayResult<String> {
        self.repo
            .get_striga_user_id(user_id)
            .await?
            .ok_or(GatewayError::Validation(
                "complete KYC prerequisites before requesting a card".into(),
            ))
    }
}
