use uuid::Uuid;

use crate::internal::core::financial_gateway::error::{GatewayResult};
use crate::internal::core::financial_gateway::models::BankingUserResponse;
use crate::internal::providers::striga::api::StrigaApiClient;
use crate::internal::providers::striga::models::{CreateStrigaUserRequest, KycStatus};
use crate::internal::providers::striga::repository::StrigaRepository;

#[derive(Clone)]
pub struct UserService {
    client: StrigaApiClient,
    repo: StrigaRepository,
}

impl UserService {
    pub fn new(client: StrigaApiClient, repo: StrigaRepository) -> Self {
        Self { client, repo }
    }

    pub async fn provision_user(&self, user_id: Uuid, email: &str) -> GatewayResult<()> {
        if self.repo.get_striga_user_id(user_id).await?.is_some() {
            return Ok(());
        }

        let local_part = email.split('@').next().unwrap_or("user");
        let first_name = sanitize_name(local_part, "RSC");
        let last_name = "User".to_string();

        let striga_user = self
            .client
            .create_user(&CreateStrigaUserRequest {
                first_name,
                last_name,
                email: email.to_string(),
            })
            .await?;

        self.repo
            .set_striga_user_id(user_id, &striga_user.id)
            .await?;

        self.repo
            .upsert_kyc_record(
                user_id,
                &striga_user.id,
                KycStatus::Pending,
                1,
                None,
                None,
            )
            .await?;

        self.repo
            .update_provider_status("striga", "healthy", None)
            .await?;

        tracing::info!(%user_id, striga_user_id = %striga_user.id, "banking user provisioned");
        Ok(())
    }

    pub async fn get_user(&self, user_id: Uuid) -> GatewayResult<BankingUserResponse> {
        let striga_id = self.repo.get_striga_user_id(user_id).await?;
        let kyc = self.repo.get_kyc_record(user_id).await?;
        let cards = self.repo.list_user_cards(user_id).await?;

        let email = if let Some(ref sid) = striga_id {
            Some(self.client.get_user(sid).await?.email)
        } else {
            None
        };

        Ok(BankingUserResponse {
            user_id,
            email,
            kyc_status: kyc.map(|k| k.status),
            has_card: !cards.is_empty(),
        })
    }
}

fn sanitize_name(input: &str, fallback: &str) -> String {
    let cleaned: String = input
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == ' ')
        .take(40)
        .collect();
    if cleaned.len() >= 2 {
        cleaned
    } else {
        fallback.to_string()
    }
}
