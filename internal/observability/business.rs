//! Business-domain instrumentation — counters and tracing spans.
//!
//! Call these from domain services (auth, payments, swaps, fraud) rather than
//! sprinkling raw `metrics::` macros across handlers.

use tracing::Span;

// ── Auth ─────────────────────────────────────────────────────────────────────

pub mod auth {
    use super::Span;

    pub fn login_success() {
        metrics::counter!("rsc_auth_login_total", "result" => "success").increment(1);
    }

    pub fn login_failure(reason: &str) {
        metrics::counter!(
            "rsc_auth_login_total",
            "result" => "failure",
            "reason" => reason.to_string()
        )
        .increment(1);
    }

    pub fn mfa_required() {
        metrics::counter!(
            "rsc_auth_login_total",
            "result" => "mfa_required"
        )
        .increment(1);
    }

    pub fn register_success() {
        metrics::counter!("rsc_auth_register_total", "result" => "success").increment(1);
    }

    pub fn register_failure(reason: &str) {
        metrics::counter!(
            "rsc_auth_register_total",
            "result" => "failure",
            "reason" => reason.to_string()
        )
        .increment(1);
    }

    pub fn span(operation: &str) -> Span {
        tracing::info_span!("auth", operation = operation)
    }
}

// ── Payments ───────────────────────────────────────────────────────────────────

pub mod payments {
    use super::Span;

    pub fn created(currency: &str) {
        metrics::counter!(
            "rsc_payments_total",
            "operation" => "create",
            "currency" => currency.to_string()
        )
        .increment(1);
    }

    pub fn settled() {
        metrics::counter!(
            "rsc_payments_total",
            "operation" => "settle"
        )
        .increment(1);
    }

    pub fn failed(reason: &str) {
        metrics::counter!(
            "rsc_payments_total",
            "operation" => "fail",
            "reason" => reason.to_string()
        )
        .increment(1);
    }

    pub fn span(operation: &str) -> Span {
        tracing::info_span!("payments", operation = operation)
    }
}

// ── Swaps ──────────────────────────────────────────────────────────────────────

pub mod swaps {
    use super::Span;

    pub fn quoted(provider: &str) {
        metrics::counter!(
            "rsc_swaps_total",
            "operation" => "quote",
            "provider" => provider.to_string()
        )
        .increment(1);
    }

    pub fn executed(provider: &str, status: &str) {
        metrics::counter!(
            "rsc_swaps_total",
            "operation" => "execute",
            "provider" => provider.to_string(),
            "status" => status.to_string()
        )
        .increment(1);
    }

    pub fn span(operation: &str) -> Span {
        tracing::info_span!("swaps", operation = operation)
    }
}

// ── Fraud ──────────────────────────────────────────────────────────────────────

pub mod fraud {
    use super::Span;

    pub fn assessed(decision: &str, score: u8) {
        metrics::counter!(
            "rsc_fraud_assessments_total",
            "decision" => decision.to_string()
        )
        .increment(1);
        metrics::histogram!("rsc_fraud_score", "decision" => decision.to_string())
            .record(f64::from(score));
    }

    pub fn span(action: &str) -> Span {
        tracing::info_span!("fraud", action = action)
    }
}

// ── Fiat on-ramp ───────────────────────────────────────────────────────────────

pub mod fiat {
    use super::Span;

    pub fn conversion(provider: &str, status: &str) {
        metrics::counter!(
            "rsc_fiat_conversions_total",
            "provider" => provider.to_string(),
            "status" => status.to_string()
        )
        .increment(1);
    }

    pub fn span(operation: &str) -> Span {
        tracing::info_span!("fiat", operation = operation)
    }
}
