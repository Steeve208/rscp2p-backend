//! Redis-backed per-IP rate limiting (anti-abuse).

use axum::extract::{connect_info::ConnectInfo, Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;
use std::net::SocketAddr;
use tracing::warn;

use crate::internal::middleware::request_id::RequestId;
use crate::internal::security::client_ip;
use crate::internal::state::AppState;

/// Paths that skip rate limiting (health probes, metrics scrapes).
const SKIP_PREFIXES: &[&str] = &["/health", "/metrics"];

pub async fn middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = req.uri().path();
    if req.method() == axum::http::Method::OPTIONS
        || SKIP_PREFIXES.iter().any(|p| path == *p || path.starts_with(&format!("{p}/")))
    {
        return Ok(next.run(req).await);
    }

    let limit = state.config.server.rate_limit_per_second;
    if limit == 0 {
        return Ok(next.run(req).await);
    }

    let peer = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|info| info.0);
    let client_ip = client_ip::resolve_client_ip(
        peer,
        req.headers(),
        &state.config.auth.trusted_proxies,
    )
    .unwrap_or_else(|| "unknown".into());

    match check_limit(&state, &client_ip, limit).await {
        Ok(allowed) if allowed => Ok(next.run(req).await),
        Ok(_) => {
            let request_id = req
                .extensions()
                .get::<RequestId>()
                .map(|id| id.as_str())
                .unwrap_or("-");
            warn!(
                client_ip = %client_ip,
                path = %path,
                request_id = %request_id,
                limit,
                "rate limit exceeded"
            );
            Err(StatusCode::TOO_MANY_REQUESTS)
        }
        Err(e) => {
            warn!(error = %e, "rate limit check failed; allowing request");
            Ok(next.run(req).await)
        }
    }
}

async fn check_limit(state: &AppState, client_ip: &str, limit: u64) -> Result<bool, redis::RedisError> {
    let window = chrono::Utc::now().timestamp();
    let key = format!("rl:{client_ip}:{window}");

    let mut conn = state.redis.clone();
    let count: i64 = redis::cmd("INCR")
        .arg(&key)
        .query_async(&mut conn)
        .await?;

    if count == 1 {
        let _: () = redis::cmd("EXPIRE")
            .arg(&key)
            .arg(2_i64)
            .query_async(&mut conn)
            .await?;
    }

    Ok((count as u64) <= limit)
}
