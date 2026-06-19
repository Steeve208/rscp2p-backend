//! Structured logging + optional OTLP trace export (Jaeger, Tempo, Datadog, etc.).

use std::sync::OnceLock;

use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::{Sampler, TracerProvider};
use opentelemetry_sdk::Resource;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

use crate::internal::config::{Environment, ObservabilityConfig};

static TRACER_PROVIDER: OnceLock<TracerProvider> = OnceLock::new();

/// Bootstrap the global tracing subscriber (stdout/json + optional OTLP layer).
pub fn init(environment: Environment, cfg: &ObservabilityConfig) -> anyhow::Result<()> {
    let default_filter = match environment {
        Environment::Production => "rsc_gateway=info,tower_http=warn,sqlx=warn",
        Environment::Staging => "rsc_gateway=info,tower_http=info,sqlx=warn",
        Environment::Development => "rsc_gateway=debug,tower_http=debug,sqlx=info",
    };

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| default_filter.into());

    let fmt_layer = match environment {
        Environment::Production => fmt::layer().json().flatten_event(true).boxed(),
        _ => fmt::layer().boxed(),
    };

    let registry = tracing_subscriber::registry().with(env_filter).with(fmt_layer);

    if cfg.otlp_enabled() {
        let provider = build_otlp_provider(cfg)?;
        TRACER_PROVIDER
            .set(provider.clone())
            .map_err(|_| anyhow::anyhow!("tracer provider already initialized"))?;

        let tracer = provider.tracer(cfg.otlp_service_name.clone());
        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer).boxed();

        registry.with(otel_layer).init();
        tracing::info!(
            endpoint = cfg.otlp_endpoint.as_deref().unwrap_or("-"),
            service = %cfg.otlp_service_name,
            sample_ratio = cfg.otlp_sample_ratio,
            "OTLP trace export enabled"
        );
    } else {
        registry.init();
    }

    Ok(())
}

/// Flush pending spans — call on graceful shutdown.
pub fn shutdown() {
    if let Some(provider) = TRACER_PROVIDER.get() {
        if let Err(e) = provider.shutdown() {
            tracing::warn!(error = %e, "failed to shutdown tracer provider");
        }
    }
}

fn build_otlp_provider(cfg: &ObservabilityConfig) -> anyhow::Result<TracerProvider> {
    let endpoint = cfg
        .otlp_endpoint
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("OTLP endpoint not configured"))?;

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .build()?;

    let resource = Resource::new(vec![opentelemetry::KeyValue::new(
        "service.name",
        cfg.otlp_service_name.clone(),
    )]);

    let sampler = match cfg.otlp_sample_ratio {
        r if r >= 1.0 => Sampler::AlwaysOn,
        r if r <= 0.0 => Sampler::AlwaysOff,
        r => Sampler::TraceIdRatioBased(r),
    };

    Ok(TracerProvider::builder()
        .with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)
        .with_sampler(sampler)
        .with_resource(resource)
        .build())
}
