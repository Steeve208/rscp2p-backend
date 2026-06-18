//! Swaps HTTP bindings — `/swaps/*`.
//!
//! No business logic: delegates to [`crate::internal::swaps::handlers`].

use axum::Router;

use crate::internal::state::AppState;
use crate::internal::swaps::handlers;

/// Authenticated swap quote and execution endpoints.
pub fn router() -> Router<AppState> {
    Router::new().nest("/swaps", handlers::router())
}
