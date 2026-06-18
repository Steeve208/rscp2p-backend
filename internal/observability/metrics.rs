//! Prometheus metrics — HTTP + business counters/histograms.
//!
//! Recorder is installed at startup via [`init_recorder`]. Scrape via `/metrics`.

use std::sync::OnceLock;

use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};

static PROMETHEUS: OnceLock<PrometheusHandle> = OnceLock::new();

/// Install the global Prometheus recorder. Idempotent — second call is a no-op.
pub fn init_recorder() -> anyhow::Result<()> {
    if PROMETHEUS.get().is_some() {
        return Ok(());
    }

    let handle = PrometheusBuilder::new().install_recorder()?;
    PROMETHEUS
        .set(handle)
        .map_err(|_| anyhow::anyhow!("prometheus recorder already initialized"))?;

    describe_metrics();
    Ok(())
}

/// Handle for rendering the Prometheus text exposition format.
pub fn handle() -> Option<&'static PrometheusHandle> {
    PROMETHEUS.get()
}

fn describe_metrics() {
    metrics::describe_counter!(
        "http_requests_total",
        "Total HTTP requests processed by the gateway"
    );
    metrics::describe_histogram!(
        "http_request_duration_seconds",
        "HTTP request latency in seconds"
    );
    metrics::describe_counter!(
        "rsc_auth_login_total",
        "Authentication login attempts"
    );
    metrics::describe_counter!(
        "rsc_auth_register_total",
        "User registration attempts"
    );
    metrics::describe_counter!(
        "rsc_payments_total",
        "Payment operations"
    );
    metrics::describe_counter!(
        "rsc_swaps_total",
        "Swap operations"
    );
    metrics::describe_counter!(
        "rsc_fraud_assessments_total",
        "Fraud engine assessments"
    );
    metrics::describe_histogram!(
        "rsc_fraud_score",
        "Fraud assessment score distribution"
    );
    metrics::describe_counter!(
        "rsc_fiat_conversions_total",
        "Fiat on-ramp conversion events"
    );
}

/// Normalize URL paths for low-cardinality metric labels (UUIDs → `:id`).
pub fn normalize_path(path: &str) -> String {
    path.split('/')
        .map(|segment| {
            if uuid::Uuid::parse_str(segment).is_ok() {
                ":id"
            } else {
                segment
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_path_replaces_uuids() {
        let id = uuid::Uuid::new_v4();
        let path = format!("/users/{id}/wallets");
        assert_eq!(normalize_path(&path), "/users/:id/wallets");
    }
}
