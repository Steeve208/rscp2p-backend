use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};

use crate::internal::auth::AuthenticatedUser;
use crate::internal::blockchain::error::BlockchainResult;
use crate::internal::blockchain::models::{
    BroadcastTxRequest, BroadcastTxResponse, NodeHealth, OnChainBalance, OnChainBlock,
    OnChainTransaction,
};
use crate::internal::state::AppState;

/// Authenticated blockchain connector routes (read + broadcast).
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(node_health))
        .route("/rsc/balance/:address", get(get_balance))
        .route("/rsc/tx/:hash", get(get_transaction))
        .route("/rsc/block/latest", get(get_latest_block))
        .route("/rsc/block/:number", get(get_block))
        .route("/rsc/broadcast", post(broadcast_tx))
}

async fn node_health(State(state): State<AppState>) -> BlockchainResult<Json<NodeHealth>> {
    Ok(Json(state.blockchain.health().await?))
}

async fn get_balance(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(address): Path<String>,
) -> BlockchainResult<Json<OnChainBalance>> {
    Ok(Json(state.blockchain.get_balance(&address).await?))
}

async fn get_transaction(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(hash): Path<String>,
) -> BlockchainResult<Json<OnChainTransaction>> {
    Ok(Json(state.blockchain.get_transaction(&hash).await?))
}

async fn get_latest_block(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
) -> BlockchainResult<Json<OnChainBlock>> {
    Ok(Json(state.blockchain.get_latest_block().await?))
}

async fn get_block(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Path(number): Path<u64>,
) -> BlockchainResult<Json<OnChainBlock>> {
    Ok(Json(state.blockchain.get_block(number).await?))
}

async fn broadcast_tx(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
    Json(body): Json<BroadcastTxRequest>,
) -> BlockchainResult<Json<BroadcastTxResponse>> {
    Ok(Json(state.blockchain.broadcast_transaction(body).await?))
}
