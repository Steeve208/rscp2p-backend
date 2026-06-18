//! Transak webhook verification and order event parsing.

use serde_json::Value;

use crate::internal::config::Environment;
use crate::internal::providers::transak::config::TransakProviderConfig;
use crate::internal::providers::transak::error::{TransakError, TransakResult};
use crate::internal::providers::transak::models::OrderStatus;

/// Normalized order notification for the fiat orchestration layer.
#[derive(Debug, Clone)]
pub struct OrderNotification {
    pub order_id: String,
    pub status: OrderStatus,
    pub partner_order_id: Option<String>,
    pub raw: Value,
}

pub fn verify_webhook_secret(
    config: &TransakProviderConfig,
    environment: Environment,
    secret: Option<&str>,
) -> TransakResult<()> {
    let Some(expected) = config.webhook_secret.as_deref() else {
        if environment.is_production() {
            return Err(TransakError::NotConfigured);
        }
        return Ok(());
    };
    match secret {
        Some(s) if s == expected => Ok(()),
        _ => Err(TransakError::WebhookForbidden),
    }
}

pub fn parse_webhook(body: &Value) -> TransakResult<OrderNotification> {
    let data = body.get("data").unwrap_or(body);

    let order_id = data
        .get("id")
        .or_else(|| data.get("orderId"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| TransakError::Validation("missing order id".into()))?
        .to_string();

    let status_str = data
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("UNKNOWN");

    let partner_order_id = data
        .get("partnerOrderId")
        .or_else(|| data.get("partnerCustomerId"))
        .or_else(|| body.get("partnerOrderId"))
        .and_then(|v| v.as_str())
        .map(str::to_string);

    Ok(OrderNotification {
        order_id,
        status: OrderStatus::parse(status_str),
        partner_order_id,
        raw: body.clone(),
    })
}
