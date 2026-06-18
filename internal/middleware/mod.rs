//! Axum middleware stack — intercepts requests/responses before handlers.
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`auth`] | JWT verification (`require_auth`) |
//! | [`logging`] | Structured HTTP access logs |
//! | [`rate_limit`] | Redis per-IP anti-abuse |
//! | [`cors`] | Browser cross-origin policy |
//! | [`request_id`] | `X-Request-Id` correlation |
//! | [`require_role`] | Role-based authorization (after auth) |
//!
//! Applied globally via [`apply`] in `routes/mod.rs`. Route-level JWT uses
//! [`auth::require_auth`] on protected routers.

pub mod auth;
pub mod cors;
pub mod logging;
pub mod rate_limit;
pub mod request_id;
pub mod require_role;

use std::time::Duration;

use axum::middleware::{from_fn, from_fn_with_state};
use axum::Router;
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;
use tower_http::timeout::TimeoutLayer;

use crate::internal::observability;
use crate::internal::state::AppState;

pub use auth::require_auth;
pub use request_id::RequestId;

/// Global middleware stack — consumes `state` into the router and returns `Router<()>`
/// ready for `into_make_service` / `into_make_service_with_connect_info`.
pub fn apply(router: Router<AppState>, state: AppState) -> Router<()> {
    let config = &state.config.clone();
    let timeout = Duration::from_secs(config.server.request_timeout_secs);

    router
        .layer(
            ServiceBuilder::new()
                .layer(cors::layer(config))
                .layer(CompressionLayer::new())
                .layer(TimeoutLayer::new(timeout)),
        )
        .layer(from_fn(logging::middleware))
        .layer(from_fn(observability::middleware::middleware))
        .layer(from_fn_with_state(state.clone(), rate_limit::middleware))
        .layer(from_fn(request_id::middleware))
        .with_state(state)
}
