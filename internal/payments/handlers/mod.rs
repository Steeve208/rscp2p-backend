use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};

use crate::internal::auth::AuthenticatedUser;
use crate::internal::payments::error::PaymentResult;
use crate::internal::payments::models::{
    CreateInvoiceRequest, CreateInvoiceResponse, InvoicePublicView, MerchantResponse,
    PayInvoiceRequest, PayInvoiceResponse, QrPaymentPayload, RegisterMerchantRequest,
    RequestSettlementRequest, SettlementResponse,
};
use crate::internal::state::AppState;

/// Public routes (QR scan / pay preview without login).
pub fn public_router() -> Router<AppState> {
    Router::new().route("/qr/:reference_code", get(get_invoice_public))
}

/// Authenticated payment routes.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/merchants", post(register_merchant).get(get_my_merchant))
        .route("/invoices", post(create_invoice))
        .route("/invoices/:invoice_id", get(get_invoice_qr))
        .route("/invoices/:invoice_id/pay", post(pay_invoice))
        .route("/pay/:reference_code", post(pay_by_reference))
        .route(
            "/merchants/me/settlements",
            post(request_settlement).get(list_settlements),
        )
        .nest("/fiat", crate::internal::providers::handlers::router())
}

async fn register_merchant(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<RegisterMerchantRequest>,
) -> PaymentResult<Json<MerchantResponse>> {
    Ok(Json(
        state
            .payments
            .register_merchant(user.claims.sub, body)
            .await?,
    ))
}

async fn get_my_merchant(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> PaymentResult<Json<MerchantResponse>> {
    Ok(Json(state.payments.get_my_merchant(user.claims.sub).await?))
}

async fn create_invoice(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<CreateInvoiceRequest>,
) -> PaymentResult<Json<CreateInvoiceResponse>> {
    Ok(Json(
        state.payments.create_invoice(user.claims.sub, body).await?,
    ))
}

async fn get_invoice_qr(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(invoice_id): Path<uuid::Uuid>,
) -> PaymentResult<Json<QrPaymentPayload>> {
    Ok(Json(
        state
            .payments
            .get_qr_payload(user.claims.sub, invoice_id)
            .await?,
    ))
}

async fn get_invoice_public(
    State(state): State<AppState>,
    Path(reference_code): Path<String>,
) -> PaymentResult<Json<InvoicePublicView>> {
    Ok(Json(
        state.payments.get_invoice_public(&reference_code).await?,
    ))
}

async fn pay_invoice(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(invoice_id): Path<uuid::Uuid>,
    Json(body): Json<PayInvoiceRequest>,
) -> PaymentResult<Json<PayInvoiceResponse>> {
    Ok(Json(
        state
            .payments
            .pay_invoice(user.claims.sub, invoice_id, body)
            .await?,
    ))
}

async fn pay_by_reference(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(reference_code): Path<String>,
    Json(body): Json<PayInvoiceRequest>,
) -> PaymentResult<Json<PayInvoiceResponse>> {
    Ok(Json(
        state
            .payments
            .pay_invoice_by_reference(user.claims.sub, &reference_code, body)
            .await?,
    ))
}

async fn request_settlement(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<RequestSettlementRequest>,
) -> PaymentResult<Json<SettlementResponse>> {
    Ok(Json(
        state
            .payments
            .request_settlement(user.claims.sub, body)
            .await?,
    ))
}

async fn list_settlements(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Query(query): Query<ListQuery>,
) -> PaymentResult<Json<Vec<SettlementResponse>>> {
    Ok(Json(
        state
            .payments
            .list_settlements(user.claims.sub, query.limit)
            .await?,
    ))
}

#[derive(Debug, serde::Deserialize)]
struct ListQuery {
    limit: Option<i64>,
}
