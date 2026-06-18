//! Observability layer — metrics, tracing, health probes, business instrumentation.
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`metrics`] | Prometheus recorder + path normalization |
//! | [`telemetry`] | Subscriber bootstrap + OTLP export |
//! | [`health`] | Liveness, readiness, detailed health, `/metrics` |
//! | [`business`] | Domain counters and tracing spans |
//! | [`middleware`] | HTTP request latency/status metrics |
//!
//! ## Startup
//!
//! Call [`init`] once from `cmd/main.rs` before building `AppState`.
//! Call [`shutdown`] on graceful termination to flush OTLP spans.

pub mod business;
pub mod health;
pub mod metrics;
pub mod middleware;
pub mod telemetry;

pub use business::{auth, fiat, fraud, payments, swaps};
pub use health::router as health_router;

use crate::internal::config::{Environment, ObservabilityConfig};

/// Bootstrap metrics recorder and tracing subscriber.
pub fn init(environment: Environment, cfg: &ObservabilityConfig) -> anyhow::Result<()> {
    if cfg.metrics_enabled {
        metrics::init_recorder()?;
        tracing::info!("Prometheus metrics enabled at /metrics");
    }

    telemetry::init(environment, cfg)?;
    Ok(())
}

/// Flush pending OTLP spans on shutdown.
pub fn shutdown() {
    telemetry::shutdown();
}
