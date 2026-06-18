//! HTTP request/response logging.

use std::time::Instant;

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

use crate::internal::middleware::request_id;

pub async fn middleware(req: Request, next: Next) -> Response {
    let request_id = request_id::from_request(&req);
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    let span = tracing::info_span!(
        "http_request",
        request_id = %request_id,
        %method,
        path = %path,
    );
    let _guard = span.enter();

    tracing::debug!("request started");

    let started = Instant::now();
    let response = next.run(req).await;
    let latency_ms = started.elapsed().as_millis() as u64;
    let status = response.status().as_u16();

    tracing::info!(status, latency_ms, "request completed");

    response
}
