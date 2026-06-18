//! Observability configuration: metrics, distributed tracing, health probes.

use crate::internal::config::{env, error::ConfigError};

#[derive(Debug, Clone)]
pub struct ObservabilityConfig {
    /// Expose `/metrics` (Prometheus scrape). Default: `true`.
    pub metrics_enabled: bool,
    /// OTLP gRPC endpoint (e.g. `http://localhost:4317`). Empty = tracing export disabled.
    pub otlp_endpoint: Option<String>,
    /// Service name sent to the trace backend. Default: `rsc-gateway`.
    pub otlp_service_name: String,
    /// Trace sampling ratio `0.0`–`1.0`. Default: `1.0`.
    pub otlp_sample_ratio: f64,
    /// Include blockchain node check in `/health` and `/health/ready`. Default: `true`.
    pub health_blockchain_check: bool,
}

impl ObservabilityConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        let otlp_endpoint = env::optional("OTEL_EXPORTER_OTLP_ENDPOINT")
            .filter(|v| !v.is_empty());

        let sample_ratio = env::with_default("OTEL_TRACES_SAMPLE_RATIO", "1.0")
            .parse::<f64>()
            .map_err(|_| ConfigError::Invalid {
                field: "OTEL_TRACES_SAMPLE_RATIO",
                message: "must be a number between 0.0 and 1.0".into(),
            })?;

        if !(0.0..=1.0).contains(&sample_ratio) {
            return Err(ConfigError::Invalid {
                field: "OTEL_TRACES_SAMPLE_RATIO",
                message: "must be between 0.0 and 1.0".into(),
            });
        }

        Ok(Self {
            metrics_enabled: env::bool("METRICS_ENABLED", true)?,
            otlp_endpoint,
            otlp_service_name: env::with_default("OTEL_SERVICE_NAME", "rsc-gateway"),
            otlp_sample_ratio: sample_ratio,
            health_blockchain_check: env::bool("HEALTH_CHECK_BLOCKCHAIN", true)?,
        })
    }

    pub fn otlp_enabled(&self) -> bool {
        self.otlp_endpoint.is_some()
    }
}
