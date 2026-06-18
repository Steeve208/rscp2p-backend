use std::sync::Arc;

use uuid::Uuid;

use crate::internal::config::Config;
use crate::internal::core::financial_gateway::error::{GatewayError, GatewayResult};
use crate::internal::core::financial_gateway::models::{
    KycStatusGatewayResponse, StartKycGatewayResponse,
};
use crate::internal::providers::striga::api::StrigaApiClient;
use crate::internal::providers::striga::models::KycStatus;
use crate::internal::providers::striga::repository::StrigaRepository;
use crate::internal::providers::striga::webhooks::{parse_webhook, verify_webhook_secret};

#[derive(Clone)]
pub struct KycService {
    client: StrigaApiClient,
    repo: StrigaRepository,
}

impl KycService {
    pub fn new(client: StrigaApiClient, repo: StrigaRepository) -> Self {
        Self { client, repo }
    }

    pub async fn start_kyc(&self, user_id: Uuid, tier: i32) -> GatewayResult<StartKycGatewayResponse> {
        let striga_user_id = self
            .require_striga_user_id(user_id)
            .await?;

        let response = self.client.start_kyc(&striga_user_id, tier).await?;

        self.repo
            .upsert_kyc_record(
                user_id,
                &striga_user_id,
                response.status,
                tier as i16,
                response.verification_token.as_deref(),
                None,
            )
            .await?;

        Ok(StartKycGatewayResponse {
            status: response.status,
            verification_token: response.verification_token,
        })
    }

    pub async fn get_kyc_status(&self, user_id: Uuid) -> GatewayResult<KycStatusGatewayResponse> {
        let striga_user_id = self
            .require_striga_user_id(user_id)
            .await?;

        let remote = self.client.get_kyc_status(&striga_user_id).await?;

        self.repo
            .upsert_kyc_record(
                user_id,
                &striga_user_id,
                remote.status,
                remote.tier,
                None,
                remote.rejection_reason.as_deref(),
            )
            .await?;

        Ok(KycStatusGatewayResponse {
            status: remote.status,
            tier: remote.tier,
            rejection_reason: remote.rejection_reason,
        })
    }

    pub async fn handle_striga_webhook(
        &self,
        config: &Arc<Config>,
        secret: Option<&str>,
        body: serde_json::Value,
    ) -> GatewayResult<()> {
        verify_webhook_secret(&config.providers.striga, config.environment, secret)?;

        let event = parse_webhook(&body)?;
        let log_id = self
            .repo
            .log_webhook(
                "striga",
                &event.event_type,
                event.external_id.as_deref(),
                &body,
                false,
                None,
            )
            .await?;

        let result = self.process_event(&event).await;

        match &result {
            Ok(()) => {
                self.repo.mark_webhook_processed(log_id, true, None).await?;
            }
            Err(e) => {
                self.repo
                    .mark_webhook_processed(log_id, false, Some(&e.to_string()))
                    .await?;
            }
        }

        result
    }

    async fn process_event(
        &self,
        event: &crate::internal::providers::striga::models::StrigaWebhookEvent,
    ) -> GatewayResult<()> {
        let event_upper = event.event_type.to_uppercase();

        if event_upper.contains("KYC") {
            if let Some(ref striga_user_id) = event.user_id {
                if let Some(user_id) = self.repo.find_user_by_striga_id(striga_user_id).await? {
                    let status = infer_kyc_status(&event_upper, &event.raw);
                    let rejection = event
                        .raw
                        .pointer("/rejectionReason")
                        .and_then(|v| v.as_str());

                    self.repo
                        .upsert_kyc_record(
                            user_id,
                            striga_user_id,
                            status,
                            1,
                            None,
                            rejection,
                        )
                        .await?;
                }
            }
        }

        if event_upper.contains("CARD") {
            if let Some(ref card_id) = event.card_id {
                let status = infer_card_status(&event_upper);
                if let Some(s) = status {
                    self.repo.update_card_status(card_id, s).await?;
                }
            }
        }

        if event_upper.contains("TRANSFER") {
            tracing::info!(event = %event.event_type, "transfer webhook received");
        }

        Ok(())
    }

    async fn require_striga_user_id(&self, user_id: Uuid) -> GatewayResult<String> {
        self.repo
            .get_striga_user_id(user_id)
            .await?
            .ok_or(GatewayError::Validation(
                "banking profile not ready — complete registration first".into(),
            ))
    }
}

fn infer_kyc_status(event_type: &str, body: &serde_json::Value) -> KycStatus {
    if event_type.contains("APPROVED") || event_type.contains("VERIFIED") {
        return KycStatus::Approved;
    }
    if event_type.contains("REJECTED") || event_type.contains("FAILED") {
        return KycStatus::Rejected;
    }
    if event_type.contains("REVIEW") || event_type.contains("SUBMITTED") {
        return KycStatus::InReview;
    }
    body.get("status")
        .and_then(|v| v.as_str())
        .map(KycStatus::parse)
        .unwrap_or(KycStatus::Pending)
}

fn infer_card_status(
    event_type: &str,
) -> Option<crate::internal::providers::striga::models::CardStatus> {
    use crate::internal::providers::striga::models::CardStatus;
    if event_type.contains("FROZEN") || event_type.contains("BLOCKED") {
        Some(CardStatus::Frozen)
    } else if event_type.contains("UNFROZEN") || event_type.contains("UNBLOCKED") {
        Some(CardStatus::Active)
    } else if event_type.contains("TERMINATED") || event_type.contains("CLOSED") {
        Some(CardStatus::Terminated)
    } else if event_type.contains("CREATED") {
        Some(CardStatus::Active)
    } else {
        None
    }
}
