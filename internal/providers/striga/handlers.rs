//! Striga HTTP routes — webhooks only at provider level.
//! User-facing banking routes live under `/banking` via Financial Gateway.

use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use axum::http::HeaderMap;

use crate::internal::providers::error::ProviderResult;
use crate::internal::state::AppState;

pub fn public_router() -> Router<AppState> {
    Router::new().route("/webhooks/striga", post(webhook))
}

async fn webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> ProviderResult<Json<serde_json::Value>> {
    let secret = headers
        .get("x-webhook-secret")
        .or_else(|| headers.get("x-striga-signature"))
        .and_then(|v| v.to_str().ok());

    state
        .financial_gateway
        .handle_striga_webhook(secret, body)
        .await
        .map_err(crate::internal::providers::error::ProviderError::from)?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
