use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use uuid::Uuid;

use crate::internal::auth::AuthenticatedUser;
use crate::internal::state::AppState;
use crate::internal::swaps::error::SwapResult;
use crate::internal::swaps::models::{
    CreateSwapOrderRequest, CreateSwapOrderResponse, SwapOrder, SwapPair, SwapProviderInfo,
    SwapQuoteRequest, SwapQuoteResponse,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/pairs", get(list_pairs))
        .route("/providers", get(list_providers))
        .route("/quote", post(quote))
        .route("/orders", post(create_order))
        .route("/orders/:order_id", get(get_order))
}

async fn list_pairs(State(state): State<AppState>) -> Json<Vec<SwapPair>> {
    Json(state.swaps.list_pairs())
}

async fn list_providers(State(state): State<AppState>) -> Json<Vec<SwapProviderInfo>> {
    Json(state.swaps.list_providers())
}

async fn quote(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<SwapQuoteRequest>,
) -> SwapResult<Json<SwapQuoteResponse>> {
    let _ = user;
    Ok(Json(state.swaps.quote(body).await?))
}

async fn create_order(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<CreateSwapOrderRequest>,
) -> SwapResult<Json<CreateSwapOrderResponse>> {
    Ok(Json(
        state.swaps.create_order(user.claims.sub, body).await?,
    ))
}

async fn get_order(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(order_id): Path<Uuid>,
) -> SwapResult<Json<SwapOrder>> {
    Ok(Json(state.swaps.get_order(user.claims.sub, order_id).await?))
}
