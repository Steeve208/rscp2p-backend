//! HTTP route composition — public API surface only.
//!
//! This module wires Axum routers to internal handlers. It must **not** contain
//! business logic, SQL, provider calls, or domain rules.
//!
//! | Responsibility here | Lives in `internal/*/handlers` |
//! |---------------------|--------------------------------|
//! | URL prefixes (`/auth`, `/payments`, …) | Request extraction + service calls |
//! | Public vs JWT-protected split | Validation, errors, DTO mapping |
//! | Admin role middleware (`require_admin`) | Authorization rules in services |
//!
//! ## Layout
//!
//! ```text
//! routes/
//! ├── mod.rs        — compose public + protected + global middleware
//! ├── auth.rs
//! ├── wallet.rs
//! ├── users.rs
//! ├── payments.rs
//! ├── blockchain.rs
//! └── swaps.rs
//! ```

mod auth;
mod banking;
mod blockchain;
mod payments;
mod swaps;
mod users;
mod wallet;

use axum::middleware::from_fn_with_state;
use axum::Router;

use crate::internal::middleware;
use crate::internal::middleware::auth::require_auth;
use crate::internal::observability;
use crate::internal::state::AppState;

/// Builds the full HTTP API: health/metrics (public), domain routes, JWT gate, global stack.
pub fn create_router(state: AppState) -> Router {
    let public = Router::new()
        .merge(observability::health_router())
        .merge(auth::public_routes())
        .merge(payments::public_routes())
        .merge(banking::public_routes());

    let protected = Router::new()
        .merge(auth::protected_routes())
        .merge(users::router())
        .merge(users::admin_router())
        .merge(banking::admin_router())
        .merge(wallet::router())
        .merge(blockchain::router())
        .merge(payments::router())
        .merge(banking::router())
        .merge(swaps::router())
        .route_layer(from_fn_with_state(state.clone(), require_auth));

    let api = public.merge(protected);

    middleware::apply(api, state)
}
