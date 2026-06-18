use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use std::time::Duration;
use tokio::time::sleep;
use tracing::warn;

use crate::internal::providers::striga::auth::sign_request;
use crate::internal::providers::striga::config::StrigaProviderConfig;
use crate::internal::providers::striga::error::{StrigaError, StrigaResult};
use crate::internal::providers::striga::models::{
    CardResponse, CardStatus, CardTransaction, CardType, CreateStrigaUserRequest, KycStatus,
    KycStatusResponse, StartKycResponse, StrigaUser,
};

const MAX_RETRIES: u32 = 3;
const RETRY_BACKOFF_MS: u64 = 500;

#[derive(Clone)]
pub struct StrigaApiClient {
    http: Client,
    config: StrigaProviderConfig,
}

impl StrigaApiClient {
    pub fn new(http: Client, config: StrigaProviderConfig) -> Self {
        Self { http, config }
    }

    pub fn uses_mock(&self) -> bool {
        self.config.mock_mode || !self.config.is_configured()
    }

    pub fn config(&self) -> &StrigaProviderConfig {
        &self.config
    }

    // ── Users ────────────────────────────────────────────────────────────────

    pub async fn create_user(&self, req: &CreateStrigaUserRequest) -> StrigaResult<StrigaUser> {
        if self.uses_mock() {
            return Ok(mock_user(&req.email));
        }

        let body = serde_json::json!({
            "firstName": req.first_name,
            "lastName": req.last_name,
            "email": req.email,
        });

        let value = self
            .request("POST", "/user/create", Some(&body))
            .await?;
        parse_user(&value)
    }

    pub async fn get_user(&self, striga_user_id: &str) -> StrigaResult<StrigaUser> {
        if self.uses_mock() {
            return Ok(StrigaUser {
                id: striga_user_id.to_string(),
                email: "mock@rscbank.test".into(),
                first_name: Some("RSC".into()),
                last_name: Some("User".into()),
            });
        }

        let path = format!("/user/{striga_user_id}");
        let value = self.request("GET", &path, None).await?;
        parse_user(&value)
    }

    pub async fn update_user(
        &self,
        striga_user_id: &str,
        body: &Value,
    ) -> StrigaResult<StrigaUser> {
        if self.uses_mock() {
            return Ok(StrigaUser {
                id: striga_user_id.to_string(),
                email: body
                    .get("email")
                    .and_then(|v| v.as_str())
                    .unwrap_or("mock@rscbank.test")
                    .into(),
                first_name: body
                    .get("firstName")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                last_name: body
                    .get("lastName")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
            });
        }

        let path = format!("/user/{striga_user_id}");
        let value = self.request("PATCH", &path, Some(body)).await?;
        parse_user(&value)
    }

    pub async fn delete_user(&self, striga_user_id: &str) -> StrigaResult<()> {
        if self.uses_mock() {
            return Ok(());
        }

        let path = format!("/user/{striga_user_id}");
        self.request("DELETE", &path, None).await?;
        Ok(())
    }

    // ── KYC ──────────────────────────────────────────────────────────────────

    pub async fn start_kyc(&self, striga_user_id: &str, tier: i32) -> StrigaResult<StartKycResponse> {
        if self.uses_mock() {
            return Ok(StartKycResponse {
                status: KycStatus::InReview,
                verification_token: Some(format!("mock-kyc-token-{striga_user_id}")),
            });
        }

        let body = serde_json::json!({
            "userId": striga_user_id,
            "tier": tier,
        });
        let value = self
            .request("POST", "/user/kyc/start", Some(&body))
            .await?;

        Ok(StartKycResponse {
            status: KycStatus::InReview,
            verification_token: value
                .get("token")
                .and_then(|v| v.as_str())
                .map(str::to_string),
        })
    }

    pub async fn get_kyc_status(&self, striga_user_id: &str) -> StrigaResult<KycStatusResponse> {
        if self.uses_mock() {
            return Ok(KycStatusResponse {
                status: KycStatus::Pending,
                tier: 1,
                rejection_reason: None,
            });
        }

        let path = format!("/user/{striga_user_id}/kyc/status");
        let value = self.request("GET", &path, None).await?;

        let status_str = value
            .get("status")
            .or_else(|| value.get("kycStatus"))
            .and_then(|v| v.as_str())
            .unwrap_or("PENDING");

        Ok(KycStatusResponse {
            status: KycStatus::parse(status_str),
            tier: value
                .get("tier")
                .and_then(|v| v.as_i64())
                .unwrap_or(1) as i16,
            rejection_reason: value
                .get("rejectionReason")
                .or_else(|| value.pointer("/errorDetails/message"))
                .and_then(|v| v.as_str())
                .map(str::to_string),
        })
    }

    // ── Cards ────────────────────────────────────────────────────────────────

    pub async fn create_virtual_card(
        &self,
        striga_user_id: &str,
        account_id: Option<&str>,
    ) -> StrigaResult<CardResponse> {
        self.create_card(striga_user_id, CardType::Virtual, account_id)
            .await
    }

    pub async fn create_physical_card(
        &self,
        striga_user_id: &str,
        account_id: Option<&str>,
    ) -> StrigaResult<CardResponse> {
        self.create_card(striga_user_id, CardType::Physical, account_id)
            .await
    }

    async fn create_card(
        &self,
        striga_user_id: &str,
        card_type: CardType,
        account_id: Option<&str>,
    ) -> StrigaResult<CardResponse> {
        if self.uses_mock() {
            let id = uuid::Uuid::new_v4().to_string();
            return Ok(CardResponse {
                id: id.clone(),
                card_type,
                status: if card_type == CardType::Physical {
                    CardStatus::Dispatched
                } else {
                    CardStatus::Active
                },
                last_four: Some("4242".into()),
                expiry_month: Some(12),
                expiry_year: Some(2028),
            });
        }

        let mut body = serde_json::json!({
            "userId": striga_user_id,
            "type": match card_type {
                CardType::Virtual => "VIRTUAL",
                CardType::Physical => "PHYSICAL",
            },
        });
        if let Some(acc) = account_id {
            body["accountId"] = Value::String(acc.to_string());
        }

        let value = self.request("POST", "/card/create", Some(&body)).await?;
        parse_card(&value, card_type)
    }

    pub async fn get_card(&self, striga_card_id: &str) -> StrigaResult<CardResponse> {
        if self.uses_mock() {
            return Ok(CardResponse {
                id: striga_card_id.to_string(),
                card_type: CardType::Virtual,
                status: CardStatus::Active,
                last_four: Some("4242".into()),
                expiry_month: Some(12),
                expiry_year: Some(2028),
            });
        }

        let path = format!("/card/{striga_card_id}");
        let value = self.request("GET", &path, None).await?;
        let card_type = value
            .get("type")
            .and_then(|v| v.as_str())
            .map(|t| {
                if t.eq_ignore_ascii_case("PHYSICAL") {
                    CardType::Physical
                } else {
                    CardType::Virtual
                }
            })
            .unwrap_or(CardType::Virtual);
        parse_card(&value, card_type)
    }

    pub async fn freeze_card(&self, striga_card_id: &str) -> StrigaResult<CardResponse> {
        self.set_card_blocked(striga_card_id, true).await
    }

    pub async fn unfreeze_card(&self, striga_card_id: &str) -> StrigaResult<CardResponse> {
        self.set_card_blocked(striga_card_id, false).await
    }

    async fn set_card_blocked(
        &self,
        striga_card_id: &str,
        blocked: bool,
    ) -> StrigaResult<CardResponse> {
        if self.uses_mock() {
            return Ok(CardResponse {
                id: striga_card_id.to_string(),
                card_type: CardType::Virtual,
                status: if blocked {
                    CardStatus::Frozen
                } else {
                    CardStatus::Active
                },
                last_four: Some("4242".into()),
                expiry_month: Some(12),
                expiry_year: Some(2028),
            });
        }

        let endpoint = if blocked {
            "/card/block"
        } else {
            "/card/unblock"
        };
        let body = serde_json::json!({ "cardId": striga_card_id });
        let value = self.request("POST", endpoint, Some(&body)).await?;
        parse_card(&value, CardType::Virtual)
    }

    pub async fn terminate_card(&self, striga_card_id: &str) -> StrigaResult<CardResponse> {
        if self.uses_mock() {
            return Ok(CardResponse {
                id: striga_card_id.to_string(),
                card_type: CardType::Virtual,
                status: CardStatus::Terminated,
                last_four: Some("4242".into()),
                expiry_month: None,
                expiry_year: None,
            });
        }

        let body = serde_json::json!({ "cardId": striga_card_id });
        let value = self
            .request("POST", "/card/close", Some(&body))
            .await?;
        parse_card(&value, CardType::Virtual)
    }

    pub async fn activate_physical_card(
        &self,
        striga_card_id: &str,
        activation_code: &str,
    ) -> StrigaResult<CardResponse> {
        if self.uses_mock() {
            return Ok(CardResponse {
                id: striga_card_id.to_string(),
                card_type: CardType::Physical,
                status: CardStatus::Active,
                last_four: Some("4242".into()),
                expiry_month: Some(12),
                expiry_year: Some(2028),
            });
        }

        let body = serde_json::json!({
            "cardId": striga_card_id,
            "activationCode": activation_code,
        });
        let value = self
            .request("POST", "/card/activate", Some(&body))
            .await?;
        parse_card(&value, CardType::Physical)
    }

    pub async fn replace_card(&self, striga_card_id: &str) -> StrigaResult<CardResponse> {
        if self.uses_mock() {
            return Ok(CardResponse {
                id: uuid::Uuid::new_v4().to_string(),
                card_type: CardType::Physical,
                status: CardStatus::Dispatched,
                last_four: Some("9999".into()),
                expiry_month: Some(12),
                expiry_year: Some(2029),
            });
        }

        let body = serde_json::json!({ "cardId": striga_card_id });
        let value = self
            .request("POST", "/card/replace", Some(&body))
            .await?;
        parse_card(&value, CardType::Physical)
    }

    pub async fn get_card_transactions(
        &self,
        striga_card_id: &str,
    ) -> StrigaResult<Vec<CardTransaction>> {
        if self.uses_mock() {
            return Ok(vec![CardTransaction {
                external_id: "mock-tx-1".into(),
                amount: rust_decimal::Decimal::new(2500, 2),
                currency: "EUR".into(),
                direction: "DEBIT".into(),
                merchant_name: Some("RSC Merchant".into()),
                status: "SETTLED".into(),
                transacted_at: chrono::Utc::now(),
            }]);
        }

        let path = format!("/card/{striga_card_id}/statement");
        let value = self.request("GET", &path, None).await?;
        parse_card_transactions(&value)
    }

    // ── HTTP layer ───────────────────────────────────────────────────────────

    async fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<&Value>,
    ) -> StrigaResult<Value> {
        let api_key = self
            .config
            .api_key
            .as_deref()
            .ok_or(StrigaError::NotConfigured)?;
        let api_secret = self
            .config
            .api_secret
            .as_deref()
            .ok_or(StrigaError::NotConfigured)?;

        let payload = body.cloned().unwrap_or_else(|| serde_json::json!({}));
        let url = format!("{}{}", self.config.base_url, path);
        let auth = sign_request(api_secret, method, path, &payload);

        let mut last_err = None;
        for attempt in 0..MAX_RETRIES {
            if attempt > 0 {
                sleep(Duration::from_millis(RETRY_BACKOFF_MS * attempt as u64)).await;
            }

            let mut builder = match method {
                "GET" => self.http.get(&url),
                "POST" => self.http.post(&url),
                "PATCH" => self.http.patch(&url),
                "DELETE" => self.http.delete(&url),
                other => {
                    return Err(StrigaError::Validation(format!(
                        "unsupported HTTP method: {other}"
                    )))
                }
            };

            builder = builder
                .header("api-key", api_key)
                .header("Authorization", &auth)
                .header("Content-Type", "application/json");

            let response = if method == "GET" {
                builder.send().await
            } else {
                builder.json(&payload).send().await
            };

            match response {
                Ok(resp) => {
                    let status = resp.status();
                    let text = resp.text().await.unwrap_or_default();

                    if status.is_success() {
                        if text.is_empty() {
                            return Ok(Value::Null);
                        }
                        return serde_json::from_str(&text)
                            .map_err(|e| StrigaError::Parse(e.to_string()));
                    }

                    if status.as_u16() == 429 || status.is_server_error() {
                        warn!(
                            attempt,
                            status = %status,
                            path,
                            "striga request retryable failure"
                        );
                        last_err = Some(StrigaError::Upstream(format!(
                            "HTTP {status}: {text}"
                        )));
                        continue;
                    }

                    return Err(StrigaError::Upstream(format!("HTTP {status}: {text}")));
                }
                Err(e) => {
                    warn!(attempt, error = %e, path, "striga request transport error");
                    last_err = Some(StrigaError::Upstream(format!("request failed: {e}")));
                }
            }
        }

        Err(last_err.unwrap_or(StrigaError::Upstream(
            "max retries exceeded".into(),
        )))
    }
}

fn mock_user(email: &str) -> StrigaUser {
    StrigaUser {
        id: uuid::Uuid::new_v4().to_string(),
        email: email.to_string(),
        first_name: Some("RSC".into()),
        last_name: Some("User".into()),
    }
}

fn parse_user(value: &Value) -> StrigaResult<StrigaUser> {
    let id = value
        .get("id")
        .or_else(|| value.get("userId"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| StrigaError::Parse("missing user id".into()))?
        .to_string();

    Ok(StrigaUser {
        id,
        email: value
            .get("email")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        first_name: value
            .get("firstName")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        last_name: value
            .get("lastName")
            .and_then(|v| v.as_str())
            .map(str::to_string),
    })
}

fn parse_card(value: &Value, default_type: CardType) -> StrigaResult<CardResponse> {
    let id = value
        .get("id")
        .or_else(|| value.get("cardId"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| StrigaError::Parse("missing card id".into()))?
        .to_string();

    let status_str = value
        .get("status")
        .or_else(|| value.get("cardStatus"))
        .and_then(|v| v.as_str())
        .unwrap_or("PENDING");

    Ok(CardResponse {
        id,
        card_type: value
            .get("type")
            .and_then(|v| v.as_str())
            .map(|t| {
                if t.eq_ignore_ascii_case("PHYSICAL") {
                    CardType::Physical
                } else {
                    CardType::Virtual
                }
            })
            .unwrap_or(default_type),
        status: CardStatus::parse(status_str),
        last_four: value
            .get("last4")
            .or_else(|| value.get("maskedPan"))
            .and_then(|v| v.as_str())
            .map(|s| s.chars().rev().take(4).collect::<String>().chars().rev().collect()),
        expiry_month: value
            .pointer("/expiry/month")
            .or_else(|| value.get("expiryMonth"))
            .and_then(|v| v.as_i64())
            .map(|m| m as i16),
        expiry_year: value
            .pointer("/expiry/year")
            .or_else(|| value.get("expiryYear"))
            .and_then(|v| v.as_i64())
            .map(|y| y as i16),
    })
}

fn parse_card_transactions(value: &Value) -> StrigaResult<Vec<CardTransaction>> {
    let items = value
        .get("transactions")
        .or_else(|| value.get("data"))
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let mut out = Vec::with_capacity(items.len());
    for item in items {
        let external_id = item
            .get("id")
            .or_else(|| item.get("transactionId"))
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        if external_id.is_empty() {
            continue;
        }

        let amount_str = item
            .get("amount")
            .and_then(|v| v.as_str().or_else(|| v.as_f64().map(|_| "")))
            .unwrap_or("0");
        let amount = if let Ok(d) = amount_str.parse::<rust_decimal::Decimal>() {
            d
        } else {
            item.get("amount")
                .and_then(|v| v.as_f64())
                .and_then(rust_decimal::Decimal::from_f64_retain)
                .unwrap_or(rust_decimal::Decimal::ZERO)
        };

        out.push(CardTransaction {
            external_id,
            amount,
            currency: item
                .get("currency")
                .and_then(|v| v.as_str())
                .unwrap_or("EUR")
                .to_uppercase(),
            direction: item
                .get("direction")
                .or_else(|| item.get("type"))
                .and_then(|v| v.as_str())
                .unwrap_or("DEBIT")
                .to_uppercase(),
            merchant_name: item
                .get("merchantName")
                .or_else(|| item.get("merchant"))
                .and_then(|v| v.as_str())
                .map(str::to_string),
            status: item
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("SETTLED")
                .to_uppercase(),
            transacted_at: item
                .get("createdAt")
                .or_else(|| item.get("timestamp"))
                .and_then(|v| v.as_str())
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&chrono::Utc))
                .unwrap_or_else(chrono::Utc::now),
        });
    }

    Ok(out)
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct StrigaErrorBody {
    status: Option<i32>,
    error_code: Option<String>,
}
