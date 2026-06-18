//! Cross-origin resource sharing for browser clients.

use axum::http::{HeaderName, HeaderValue, Method};
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::internal::config::Config;
use crate::internal::middleware::request_id::HEADER_NAME;

pub fn layer(config: &Config) -> CorsLayer {
    let mut cors = CorsLayer::new()
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
            axum::http::header::ACCEPT,
            HeaderName::from_static(HEADER_NAME),
        ])
        .expose_headers([HeaderName::from_static(HEADER_NAME)]);

    if config.server.allowed_origins.len() == 1 && config.server.allowed_origins[0] == "*" {
        cors = cors.allow_origin(AllowOrigin::any());
    } else {
        let origins: Vec<HeaderValue> = config
            .server
            .allowed_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        cors = cors.allow_origin(AllowOrigin::list(origins));
    }

    cors
}
