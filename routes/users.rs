//! Users HTTP bindings — `/users/*` and `/admin/users/*`.
//!
//! No business logic: delegates to [`crate::internal::users::handlers`].

use axum::middleware::from_fn;
use axum::Router;

use crate::internal::middleware::require_role::require_admin;
use crate::internal::state::AppState;
use crate::internal::users::handlers;

/// Self-service profile and account lifecycle (JWT required via `routes/mod.rs`).
pub fn router() -> Router<AppState> {
    Router::new().nest("/users", handlers::router())
}

/// Admin operations — JWT + [`require_admin`] role.
pub fn admin_router() -> Router<AppState> {
    Router::new()
        .nest("/admin", handlers::admin_router())
        .route_layer(from_fn(require_admin))
}
