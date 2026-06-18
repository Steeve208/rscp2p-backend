//! Health probes and Prometheus scrape endpoint.
//!
//! | Route | Purpose |
//! |-------|---------|
//! | `GET /health/live` | Liveness — process is running |
//! | `GET /health/ready` | Readiness — dependencies available |
//! | `GET /health` | Detailed status for ops dashboards |
//! | `GET /metrics` | Prometheus text exposition |

use std::collections::BTreeMap;
use std::time::Instant;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::Router;
use serde::Serialize;

use crate::internal::database;
use crate::internal::redis;
use crate::internal::state::AppState;

use super::metrics;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health_detailed))
        .route("/health/live", get(liveness))
        .route("/health/ready", get(readiness))
        .route("/metrics", get(prometheus_metrics))
}

// ── Liveness ───────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct LivenessResponse {
    status: &'static str,
}

async fn liveness() -> axum::Json<LivenessResponse> {
    axum::Json(LivenessResponse { status: "ok" })
}

// ── Readiness ──────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ReadinessResponse {
    status: &'static str,
    checks: BTreeMap<String, ComponentStatus>,
}

async fn readiness(State(state): State<AppState>) -> Response {
    let checks = run_core_checks(&state).await;
    let all_up = checks.values().all(|c| c.status == CheckState::Up);

    let body = ReadinessResponse {
        status: if all_up { "ready" } else { "not_ready" },
        checks: checks
            .into_iter()
            .map(|(k, v)| (k, ComponentStatus::from(v)))
            .collect(),
    };

    let status = if all_up {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (status, axum::Json(body)).into_response()
}

// ── Detailed health ────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct DetailedHealthResponse {
    status: OverallStatus,
    version: &'static str,
    uptime_secs: u64,
    environment: String,
    checks: BTreeMap<String, ComponentStatus>,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum OverallStatus {
    Ok,
    Degraded,
    Unhealthy,
}

#[derive(Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum CheckState {
    Up,
    Down,
    Degraded,
}

#[derive(Serialize)]
struct ComponentStatus {
    status: CheckState,
    #[serde(skip_serializing_if = "Option::is_none")]
    latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<serde_json::Value>,
}

impl From<CheckResult> for ComponentStatus {
    fn from(r: CheckResult) -> Self {
        Self {
            status: r.status,
            latency_ms: r.latency_ms,
            details: r.details,
        }
    }
}

struct CheckResult {
    status: CheckState,
    latency_ms: Option<u64>,
    details: Option<serde_json::Value>,
}

async fn health_detailed(State(state): State<AppState>) -> axum::Json<DetailedHealthResponse> {
    let mut checks = run_core_checks(&state).await;

    if state.config.observability.health_blockchain_check {
        checks.insert("blockchain".into(), check_blockchain(&state).await);
    }

    checks.insert(
        "metrics".into(),
        CheckResult {
            status: if metrics::handle().is_some() {
                CheckState::Up
            } else {
                CheckState::Degraded
            },
            latency_ms: None,
            details: Some(serde_json::json!({
                "enabled": state.config.observability.metrics_enabled,
            })),
        },
    );

    checks.insert(
        "tracing".into(),
        CheckResult {
            status: if state.config.observability.otlp_enabled() {
                CheckState::Up
            } else {
                CheckState::Degraded
            },
            latency_ms: None,
            details: Some(serde_json::json!({
                "otlp_enabled": state.config.observability.otlp_enabled(),
                "service_name": state.config.observability.otlp_service_name,
            })),
        },
    );

    let overall = compute_overall(&checks);

    axum::Json(DetailedHealthResponse {
        status: overall,
        version: env!("CARGO_PKG_VERSION"),
        uptime_secs: state.uptime_secs(),
        environment: state.config.environment.as_str().to_string(),
        checks: checks
            .into_iter()
            .map(|(k, v)| (k, ComponentStatus::from(v)))
            .collect(),
    })
}

fn compute_overall(checks: &BTreeMap<String, CheckResult>) -> OverallStatus {
    let core = ["database", "redis"];
    let core_down = core.iter().any(|name| {
        checks
            .get(*name)
            .is_some_and(|c| c.status == CheckState::Down)
    });

    if core_down {
        return OverallStatus::Unhealthy;
    }

    let any_degraded = checks.values().any(|c| c.status != CheckState::Up);
    if any_degraded {
        OverallStatus::Degraded
    } else {
        OverallStatus::Ok
    }
}

async fn run_core_checks(state: &AppState) -> BTreeMap<String, CheckResult> {
    let mut checks = BTreeMap::new();
    checks.insert("database".into(), timed_check(check_database(state)).await);
    checks.insert("redis".into(), timed_check(check_redis(state)).await);
    checks
}

async fn timed_check<Fut>(fut: Fut) -> CheckResult
where
    Fut: std::future::Future<Output = CheckResult>,
{
    let start = Instant::now();
    let mut result = fut.await;
    result.latency_ms = Some(start.elapsed().as_millis() as u64);
    result
}

async fn check_database(state: &AppState) -> CheckResult {
    let ok = database::ping(&state.db).await;
    CheckResult {
        status: if ok { CheckState::Up } else { CheckState::Down },
        latency_ms: None,
        details: Some(serde_json::json!({
            "pool_size": state.db.size(),
            "idle_connections": state.db.num_idle(),
        })),
    }
}

async fn check_redis(state: &AppState) -> CheckResult {
    let ok = redis::ping(&state.redis).await;
    CheckResult {
        status: if ok { CheckState::Up } else { CheckState::Down },
        latency_ms: None,
        details: None,
    }
}

async fn check_blockchain(state: &AppState) -> CheckResult {
    match state.blockchain.health().await {
        Ok(node) => {
            let status = if node.syncing {
                CheckState::Degraded
            } else {
                CheckState::Up
            };
            CheckResult {
                status,
                latency_ms: None,
                details: Some(serde_json::json!({
                    "chain_id": node.chain_id,
                    "latest_block": node.latest_block,
                    "syncing": node.syncing,
                    "ws_configured": node.ws_configured,
                })),
            }
        }
        Err(e) => CheckResult {
            status: CheckState::Down,
            latency_ms: None,
            details: Some(serde_json::json!({ "error": e.to_string() })),
        },
    }
}

// ── Prometheus ─────────────────────────────────────────────────────────────────

async fn prometheus_metrics() -> impl IntoResponse {
    match metrics::handle() {
        Some(h) => (
            StatusCode::OK,
            [(
                axum::http::header::CONTENT_TYPE,
                "text/plain; version=0.0.4; charset=utf-8",
            )],
            h.render(),
        )
            .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            "# metrics disabled\n".to_string(),
        )
            .into_response(),
    }
}
