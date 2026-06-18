use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;

use crate::internal::auth::AuthenticatedUser;
use crate::internal::state::AppState;
use crate::internal::wallets::error::WalletResult;
use crate::internal::wallets::models::{
    BroadcastWithdrawalRequest, BroadcastWithdrawalResponse, CreateAddressRequest,
    DepositAddressResponse, EnsureDefaultWalletResponse, LedgerEntry, RequestWithdrawalRequest,
    RequestWithdrawalResponse, Wallet, WalletAddress, WalletBalance, WalletTransaction,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/me", get(list_wallets))
        .route("/me/default", post(ensure_default_wallet))
        .route(
            "/me/addresses",
            get(list_addresses).post(get_or_create_deposit_address),
        )
        .route("/me/transactions", get(list_transactions))
        .route("/me/ledger", get(list_ledger_entries))
        .route("/me/balances", get(list_balances))
        .route("/me/withdrawals", post(request_withdrawal))
        .route(
            "/me/withdrawals/:transaction_id",
            get(get_withdrawal).delete(cancel_withdrawal),
        )
        .route(
            "/me/withdrawals/:transaction_id/broadcast",
            post(broadcast_withdrawal),
        )
}

async fn list_wallets(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> WalletResult<Json<Vec<Wallet>>> {
    Ok(Json(state.wallets.list_wallets(user.claims.sub).await?))
}

async fn list_balances(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> WalletResult<Json<Vec<WalletBalance>>> {
    Ok(Json(state.wallets.list_balances(user.claims.sub).await?))
}

async fn ensure_default_wallet(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> WalletResult<Json<EnsureDefaultWalletResponse>> {
    Ok(Json(
        state.wallets.ensure_default_wallet(user.claims.sub).await?,
    ))
}

async fn list_addresses(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> WalletResult<Json<Vec<WalletAddress>>> {
    Ok(Json(state.wallets.list_addresses(user.claims.sub).await?))
}

async fn get_or_create_deposit_address(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<CreateAddressRequest>,
) -> WalletResult<Json<DepositAddressResponse>> {
    Ok(Json(
        state
            .wallets
            .get_or_create_deposit_address(user.claims.sub, body)
            .await?,
    ))
}

async fn list_transactions(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Query(query): Query<ListQuery>,
) -> WalletResult<Json<Vec<WalletTransaction>>> {
    Ok(Json(
        state
            .wallets
            .list_transactions(user.claims.sub, query.limit)
            .await?,
    ))
}

async fn request_withdrawal(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(body): Json<RequestWithdrawalRequest>,
) -> WalletResult<Json<RequestWithdrawalResponse>> {
    Ok(Json(
        state
            .wallets
            .request_withdrawal(user.claims.sub, body)
            .await?,
    ))
}

async fn get_withdrawal(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(transaction_id): Path<uuid::Uuid>,
) -> WalletResult<Json<WalletTransaction>> {
    Ok(Json(
        state
            .wallets
            .get_withdrawal(user.claims.sub, transaction_id)
            .await?,
    ))
}

async fn cancel_withdrawal(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(transaction_id): Path<uuid::Uuid>,
) -> WalletResult<Json<WalletTransaction>> {
    Ok(Json(
        state
            .wallets
            .cancel_withdrawal(user.claims.sub, transaction_id)
            .await?,
    ))
}

async fn broadcast_withdrawal(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(transaction_id): Path<uuid::Uuid>,
    Json(body): Json<BroadcastWithdrawalRequest>,
) -> WalletResult<Json<BroadcastWithdrawalResponse>> {
    Ok(Json(
        state
            .wallets
            .broadcast_withdrawal(user.claims.sub, transaction_id, body, &state.blockchain)
            .await?,
    ))
}

async fn list_ledger_entries(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Query(query): Query<ListQuery>,
) -> WalletResult<Json<Vec<LedgerEntry>>> {
    Ok(Json(
        state
            .wallets
            .list_ledger_entries(user.claims.sub, query.limit)
            .await?,
    ))
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    limit: Option<i64>,
}
