//! Blockchain HTTP bindings — `/blockchain/*`.
//!
//! No business logic: delegates to [`crate::internal::blockchain::handlers`].

use axum::Router;

use crate::internal::blockchain::handlers;
use crate::internal::state::AppState;

/// Authenticated chain/RPC helper endpoints.
pub fn router() -> Router<AppState> {
    Router::new().nest("/blockchain", handlers::router())
}
