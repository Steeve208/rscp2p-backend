//! Per-request correlation id (`X-Request-Id`).

use axum::extract::Request;
use axum::http::header::{HeaderName, HeaderValue};
use axum::middleware::Next;
use axum::response::Response;
use uuid::Uuid;

pub const HEADER_NAME: &str = "x-request-id";

static HEADER: HeaderName = HeaderName::from_static(HEADER_NAME);

/// Correlation id attached to the request and echoed on the response.
#[derive(Clone, Debug)]
pub struct RequestId(pub String);

impl RequestId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Reads the id from extensions or the incoming header (before middleware runs).
pub fn from_request<B>(req: &Request<B>) -> String {
    req.extensions()
        .get::<RequestId>()
        .map(|id| id.0.clone())
        .or_else(|| read_header(req.headers()))
        .unwrap_or_else(|| "unknown".into())
}

fn read_header(headers: &axum::http::HeaderMap) -> Option<String> {
    headers
        .get(&HEADER)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn generate_id() -> String {
    Uuid::new_v4().to_string()
}

pub async fn middleware(mut req: Request, next: Next) -> Response {
    let id = read_header(req.headers()).unwrap_or_else(generate_id);

    req.extensions_mut().insert(RequestId(id.clone()));

    let mut response = next.run(req).await;

    if let Ok(value) = HeaderValue::from_str(&id) {
        response.headers_mut().insert(HEADER.clone(), value);
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    #[test]
    fn accepts_incoming_header() {
        let mut headers = HeaderMap::new();
        headers.insert(&HEADER, "abc-123".parse().unwrap());
        assert_eq!(read_header(&headers).as_deref(), Some("abc-123"));
    }

    #[test]
    fn rejects_empty_header() {
        let mut headers = HeaderMap::new();
        headers.insert(&HEADER, "   ".parse().unwrap());
        assert!(read_header(&headers).is_none());
    }
}
