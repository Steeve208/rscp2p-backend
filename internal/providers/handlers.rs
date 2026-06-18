use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use uuid::Uuid;

use crate::internal::auth::AuthenticatedUser;
use crate::internal::providers::error::ProviderResult;
use crate::internal::providers::models::{
    FiatConversionOrder, FiatProviderInfo, FiatQuoteRequest, FiatQuoteResponse,
    StartFiatInvoicePayRequest, StartFiatInvoicePayResponse,
};
use crate::internal::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/providers", get(list_providers))
        .route("/quote", post(quote))
        .route("/invoices/:invoice_id/start", post(start_invoice_fiat_pay))
        .route("/orders/:order_id", get(get_order))
        .route("/orders/:order_id/mock-complete", post(mock_complete_order))
        .merge(protected_router())
}

pub fn public_router() -> Router<AppState> {
    Router::new()
        .nest(
            "/transak",
            crate::internal::providers::transak::handlers::public_router(),
        )
        .route("/webhooks/transak", post(transak_webhook))
}

pub fn protected_router() -> Router<AppState> {
    Router::new().nest(
        "/transak",
        crate::internal::providers::transak::handlers::protected_router(),
    )
}

async fn list_providers(State(state): State<AppState>) -> Json<Vec<FiatProviderInfo>> {
    Json(state.fiat.list_providers())
}

async fn quote(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<FiatQuoteRequest>,
) -> ProviderResult<Json<Vec<FiatQuoteResponse>>> {
    let _ = user;
    Ok(Json(state.fiat.quote(body).await?))
}

async fn start_invoice_fiat_pay(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(invoice_id): Path<Uuid>,
    Json(body): Json<StartFiatInvoicePayRequest>,
) -> ProviderResult<Json<StartFiatInvoicePayResponse>> {
    Ok(Json(
        state
            .fiat
            .start_invoice_fiat_pay(user.claims.sub, invoice_id, body)
            .await?,
    ))
}

async fn get_order(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(order_id): Path<Uuid>,
) -> ProviderResult<Json<FiatConversionOrder>> {
    Ok(Json(
        state.fiat.get_order(user.claims.sub, order_id).await?,
    ))
}

async fn mock_complete_order(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(order_id): Path<Uuid>,
) -> ProviderResult<Json<FiatConversionOrder>> {
    Ok(Json(
        state
            .fiat
            .mock_complete_order(user.claims.sub, order_id)
            .await?,
    ))
}

async fn transak_webhook(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> ProviderResult<Json<serde_json::Value>> {
    state.fiat.handle_transak_webhook(body).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
