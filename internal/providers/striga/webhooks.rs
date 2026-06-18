//! Striga webhook verification and event parsing.

use serde_json::Value;

use crate::internal::config::Environment;
use crate::internal::providers::striga::config::StrigaProviderConfig;
use crate::internal::providers::striga::error::{StrigaError, StrigaResult};
use crate::internal::providers::striga::models::StrigaWebhookEvent;

pub fn verify_webhook_secret(
    config: &StrigaProviderConfig,
    environment: Environment,
    secret: Option<&str>,
) -> StrigaResult<()> {
    let Some(expected) = config.webhook_secret.as_deref() else {
        if environment.is_production() {
            return Err(StrigaError::NotConfigured);
        }
        return Ok(());
    };
    match secret {
        Some(s) if s == expected => Ok(()),
        _ => Err(StrigaError::WebhookForbidden),
    }
}

pub fn parse_webhook(body: &Value) -> StrigaResult<StrigaWebhookEvent> {
    let event_type = body
        .get("type")
        .or_else(|| body.get("event"))
        .or_else(|| body.get("eventType"))
        .and_then(|v| v.as_str())
        .unwrap_or("UNKNOWN")
        .to_string();

    let external_id = body
        .get("id")
        .or_else(|| body.get("eventId"))
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let user_id = body
        .get("userId")
        .or_else(|| body.pointer("/data/userId"))
        .and_then(|v| v.as_str())
        .map(str::to_string);

    let card_id = body
        .get("cardId")
        .or_else(|| body.pointer("/data/cardId"))
        .and_then(|v| v.as_str())
        .map(str::to_string);

    Ok(StrigaWebhookEvent {
        event_type,
        external_id,
        user_id,
        card_id,
        raw: body.clone(),
    })
}
