//! Wallet HTTP bindings — `/wallets/*`.
//!
//! No business logic: delegates to [`crate::internal::wallets::handlers`].

use axum::Router;

use crate::internal::state::AppState;
use crate::internal::wallets::handlers;

/// Authenticated wallet, balance, deposit, and withdrawal endpoints.
pub fn router() -> Router<AppState> {
    Router::new().nest("/wallets", handlers::router())
}
