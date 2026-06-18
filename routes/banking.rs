//! RSC Bank HTTP bindings — `/banking/*` (white-label, no provider names).

use axum::middleware::from_fn;
use axum::Router;

use crate::internal::core::financial_gateway::handlers;
use crate::internal::middleware::require_role::require_admin;
use crate::internal::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new().nest("/banking", handlers::router())
}

pub fn admin_router() -> Router<AppState> {
    Router::new()
        .nest("/admin", handlers::admin_router())
        .route_layer(from_fn(require_admin))
}

pub fn public_routes() -> Router<AppState> {
    Router::new()
        .merge(crate::internal::providers::striga::handlers::public_router())
        .merge(crate::internal::core::financial_gateway::handlers::public_webhook_router())
}
