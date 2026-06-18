//! Transak-specific HTTP routes (webhooks, KYC, payment methods).

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use crate::internal::providers::error::ProviderResult;
use crate::internal::providers::transak::cards::PaymentMethodsService;
use crate::internal::providers::transak::kyc::KycService;
use crate::internal::state::AppState;

pub fn public_router() -> Router<AppState> {
    Router::new().route("/webhooks", post(webhook))
}

pub fn protected_router() -> Router<AppState> {
    Router::new()
        .route("/kyc/users/:transak_user_id", get(kyc_status))
        .route("/payment-methods", get(payment_methods))
}

async fn webhook(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> ProviderResult<Json<serde_json::Value>> {
    state.fiat.handle_transak_webhook(body).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn kyc_status(
    State(state): State<AppState>,
    Path(transak_user_id): Path<String>,
) -> ProviderResult<Json<crate::internal::providers::transak::models::KycUserStatus>> {
    let http = state.http.clone();
    let config = state.config.providers.transak.clone();
    let service = KycService::new(http, config);
    Ok(Json(
        service
            .get_user_status(&transak_user_id)
            .await
            .map_err(crate::internal::providers::error::ProviderError::from)?,
    ))
}

#[derive(Debug, Deserialize)]
struct PaymentMethodsQuery {
    fiat_currency: String,
}

async fn payment_methods(
    State(state): State<AppState>,
    Query(query): Query<PaymentMethodsQuery>,
) -> ProviderResult<Json<Vec<crate::internal::providers::transak::models::PaymentMethodOption>>> {
    let http = state.http.clone();
    let config = state.config.providers.transak.clone();
    let service = PaymentMethodsService::new(http, config);
    Ok(Json(
        service
            .list_payment_methods(&query.fiat_currency.to_uppercase())
            .await
            .map_err(crate::internal::providers::error::ProviderError::from)?,
    ))
}
