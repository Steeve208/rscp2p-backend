//! Transak KYC integration.

use reqwest::Client;

use crate::internal::providers::transak::api::TransakApiClient;
use crate::internal::providers::transak::config::TransakProviderConfig;
use crate::internal::providers::transak::error::{TransakError, TransakResult};
use crate::internal::providers::transak::models::{KycStatus, KycUserStatus};

pub struct KycService {
    api: TransakApiClient,
}

impl KycService {
    pub fn new(http: Client, config: TransakProviderConfig) -> Self {
        Self {
            api: TransakApiClient::new(http, config),
        }
    }

    pub async fn get_user_status(&self, transak_user_id: &str) -> TransakResult<KycUserStatus> {
        if self.api.uses_mock() {
            return Ok(KycUserStatus {
                user_id: transak_user_id.to_string(),
                status: KycStatus::Approved,
                level: Some("standard".into()),
            });
        }

        let _ = transak_user_id;
        Err(TransakError::Upstream(
            "Transak KYC API not configured; set TRANSAK_API_KEY and partner KYC endpoint".into(),
        ))
    }
}
