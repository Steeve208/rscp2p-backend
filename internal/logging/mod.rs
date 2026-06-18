//! Structured logging bootstrap — delegates to [`crate::internal::observability`].

use crate::internal::config::{Environment, ObservabilityConfig};

/// Initialize logging only (no metrics, no OTLP). Prefer [`crate::internal::observability::init`].
pub fn init(environment: Environment) {
    let cfg = ObservabilityConfig {
        metrics_enabled: false,
        otlp_endpoint: None,
        otlp_service_name: "rsc-gateway".into(),
        otlp_sample_ratio: 1.0,
        health_blockchain_check: true,
    };
    crate::internal::observability::telemetry::init(environment, &cfg)
        .expect("failed to initialize logging");
}
