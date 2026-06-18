//! Auth HTTP bindings — `/auth/*`.
//!
//! No business logic: delegates to [`crate::internal::auth::handlers`].

use axum::Router;

use crate::internal::auth::handlers;
use crate::internal::state::AppState;

/// Unauthenticated endpoints (register, login, refresh, JWKS, MFA login verify).
pub fn public_routes() -> Router<AppState> {
    Router::new().nest("/auth", handlers::public_router())
}

/// JWT-required session and MFA management.
pub fn protected_routes() -> Router<AppState> {
    Router::new().nest("/auth", handlers::protected_router())
}
