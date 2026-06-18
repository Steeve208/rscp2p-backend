//! HTTP request metrics middleware — latency + status counters.

use std::time::Instant;

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

use super::metrics;

pub async fn middleware(req: Request, next: Next) -> Response {
    let record = metrics::handle().is_some();
    let method = req.method().clone();
    let path = metrics::normalize_path(req.uri().path());
    let start = Instant::now();

    let response = next.run(req).await;

    if record {
        let status = response.status().as_u16();
        let elapsed_secs = start.elapsed().as_secs_f64();

        ::metrics::counter!(
            "http_requests_total",
            "method" => method.as_str().to_string(),
            "path" => path.clone(),
            "status" => status.to_string()
        )
        .increment(1);

        ::metrics::histogram!(
            "http_request_duration_seconds",
            "method" => method.as_str().to_string(),
            "path" => path
        )
        .record(elapsed_secs);
    }

    response
}
