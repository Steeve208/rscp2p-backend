//! Payments HTTP bindings — `/payments/*` and fiat provider webhooks.
//!
//! No business logic: delegates to [`crate::internal::payments::handlers`] and
//! [`crate::internal::providers::handlers`].

use axum::Router;

use crate::internal::payments::handlers;
use crate::internal::providers;
use crate::internal::state::AppState;

/// Authenticated merchant, invoice, settlement, and fiat on-ramp routes.
pub fn router() -> Router<AppState> {
    Router::new().nest("/payments", handlers::router())
}

/// Public invoice preview (QR) and provider webhooks (no JWT).
pub fn public_routes() -> Router<AppState> {
    Router::new()
        .nest("/payments", handlers::public_router())
        .nest("/payments/fiat", providers::handlers::public_router())
}
