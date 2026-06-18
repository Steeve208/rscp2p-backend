//! RSC Bank white-label banking API — no provider names exposed.

use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};

use crate::internal::auth::AuthenticatedUser;
use crate::internal::core::financial_gateway::{
    ActivateCardRequest, CryptoQuoteRequest, CryptoTradeRequest, GatewayResult,
};
use crate::internal::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/profile", get(get_banking_profile))
        .route("/kyc/start", post(start_kyc))
        .route("/kyc/status", get(kyc_status))
        .route("/cards", get(list_cards))
        .route("/cards/virtual", post(create_virtual_card))
        .route("/cards/physical", post(create_physical_card))
        .route("/cards/:card_id", get(get_card))
        .route("/cards/:card_id/freeze", post(freeze_card))
        .route("/cards/:card_id/unfreeze", post(unfreeze_card))
        .route("/cards/:card_id/terminate", post(terminate_card))
        .route("/cards/:card_id/activate", post(activate_card))
        .route("/cards/:card_id/transactions", get(card_transactions))
        .route("/crypto/quote", post(crypto_quote))
        .route("/crypto/buy", post(crypto_buy))
        .route("/crypto/sell", post(crypto_sell))
        .route("/crypto/off-ramp", post(crypto_off_ramp))
        .route("/crypto/fiat-to-crypto", post(crypto_buy))
        .route("/crypto/crypto-to-fiat", post(crypto_sell))
        .route("/crypto/orders", get(list_crypto_orders))
        .route("/crypto/orders/:order_id", get(get_crypto_order))
        .route("/crypto/assets", get(crypto_assets))
}

pub fn public_webhook_router() -> Router<AppState> {
    Router::new().route("/webhooks/transak", post(transak_webhook))
}

pub fn admin_router() -> Router<AppState> {
    Router::new().route("/providers", get(admin_providers))
}

async fn get_banking_profile(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> GatewayResult<Json<crate::internal::core::financial_gateway::BankingUserResponse>> {
    Ok(Json(
        state
            .financial_gateway
            .get_banking_user(user.claims.sub)
            .await?,
    ))
}

async fn start_kyc(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> GatewayResult<Json<crate::internal::core::financial_gateway::StartKycGatewayResponse>> {
    Ok(Json(
        state.financial_gateway.start_kyc(user.claims.sub, 1).await?,
    ))
}

async fn kyc_status(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> GatewayResult<Json<crate::internal::core::financial_gateway::KycStatusGatewayResponse>> {
    Ok(Json(
        state.financial_gateway.get_kyc_status(user.claims.sub).await?,
    ))
}

async fn list_cards(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> GatewayResult<Json<Vec<crate::internal::core::financial_gateway::CardGatewayResponse>>> {
    Ok(Json(
        state.financial_gateway.list_cards(user.claims.sub).await?,
    ))
}

async fn create_virtual_card(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> GatewayResult<Json<crate::internal::core::financial_gateway::CardGatewayResponse>> {
    Ok(Json(
        state
            .financial_gateway
            .create_virtual_card(user.claims.sub)
            .await?,
    ))
}

async fn create_physical_card(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> GatewayResult<Json<crate::internal::core::financial_gateway::CardGatewayResponse>> {
    Ok(Json(
        state
            .financial_gateway
            .create_physical_card(user.claims.sub)
            .await?,
    ))
}

async fn get_card(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(card_id): Path<uuid::Uuid>,
) -> GatewayResult<Json<crate::internal::core::financial_gateway::CardGatewayResponse>> {
    Ok(Json(
        state
            .financial_gateway
            .get_card(user.claims.sub, card_id)
            .await?,
    ))
}

async fn freeze_card(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(card_id): Path<uuid::Uuid>,
) -> GatewayResult<Json<crate::internal::core::financial_gateway::CardGatewayResponse>> {
    Ok(Json(
        state
            .financial_gateway
            .freeze_card(user.claims.sub, card_id)
            .await?,
    ))
}

async fn unfreeze_card(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(card_id): Path<uuid::Uuid>,
) -> GatewayResult<Json<crate::internal::core::financial_gateway::CardGatewayResponse>> {
    Ok(Json(
        state
            .financial_gateway
            .unfreeze_card(user.claims.sub, card_id)
            .await?,
    ))
}

async fn terminate_card(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(card_id): Path<uuid::Uuid>,
) -> GatewayResult<Json<crate::internal::core::financial_gateway::CardGatewayResponse>> {
    Ok(Json(
        state
            .financial_gateway
            .terminate_card(user.claims.sub, card_id)
            .await?,
    ))
}

async fn activate_card(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(card_id): Path<uuid::Uuid>,
    Json(body): Json<ActivateCardRequest>,
) -> GatewayResult<Json<crate::internal::core::financial_gateway::CardGatewayResponse>> {
    Ok(Json(
        state
            .financial_gateway
            .activate_physical_card(user.claims.sub, card_id, &body.activation_code)
            .await?,
    ))
}

async fn card_transactions(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(card_id): Path<uuid::Uuid>,
) -> GatewayResult<Json<Vec<crate::internal::core::financial_gateway::CardTransactionResponse>>> {
    Ok(Json(
        state
            .financial_gateway
            .get_card_transactions(user.claims.sub, card_id)
            .await?,
    ))
}

async fn crypto_quote(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<CryptoQuoteRequest>,
) -> GatewayResult<Json<crate::internal::core::financial_gateway::CryptoQuoteResponse>> {
    Ok(Json(
        state
            .financial_gateway
            .get_crypto_quote(user.claims.sub, body)
            .await?,
    ))
}

async fn crypto_buy(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<CryptoTradeRequest>,
) -> GatewayResult<Json<crate::internal::core::financial_gateway::CryptoWidgetResponse>> {
    Ok(Json(
        state.financial_gateway.buy_crypto(user.claims.sub, body).await?,
    ))
}

async fn crypto_sell(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<CryptoTradeRequest>,
) -> GatewayResult<Json<crate::internal::core::financial_gateway::CryptoWidgetResponse>> {
    Ok(Json(
        state.financial_gateway.sell_crypto(user.claims.sub, body).await?,
    ))
}

async fn crypto_off_ramp(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<CryptoTradeRequest>,
) -> GatewayResult<Json<crate::internal::core::financial_gateway::CryptoWidgetResponse>> {
    Ok(Json(
        state.financial_gateway.off_ramp(user.claims.sub, body).await?,
    ))
}

async fn list_crypto_orders(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> GatewayResult<Json<Vec<crate::internal::core::financial_gateway::CryptoOrderResponse>>> {
    Ok(Json(
        state.financial_gateway.list_crypto_orders(user.claims.sub).await?,
    ))
}

async fn get_crypto_order(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(order_id): Path<uuid::Uuid>,
) -> GatewayResult<Json<crate::internal::core::financial_gateway::CryptoOrderResponse>> {
    Ok(Json(
        state
            .financial_gateway
            .get_crypto_order(user.claims.sub, order_id)
            .await?,
    ))
}

async fn transak_webhook(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(body): Json<serde_json::Value>,
) -> GatewayResult<Json<serde_json::Value>> {
    let secret = headers
        .get("x-webhook-secret")
        .or_else(|| headers.get("x-transak-signature"))
        .and_then(|v| v.to_str().ok());

    state
        .financial_gateway
        .handle_transak_webhook(secret, body)
        .await?;

    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn crypto_assets(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> GatewayResult<Json<Vec<String>>> {
    let _ = user;
    Ok(Json(
        state.financial_gateway.list_supported_crypto_assets().await?,
    ))
}

async fn admin_providers(
    State(state): State<AppState>,
    _admin: AuthenticatedUser,
) -> GatewayResult<Json<crate::internal::core::financial_gateway::ProvidersDashboardResponse>> {
    Ok(Json(state.financial_gateway.admin_provider_status().await?))
}
