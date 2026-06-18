//! Fraud signal kinds and the `FraudSignal` value type.

use serde::{Deserialize, Serialize};

/// All possible signals the fraud engine can emit.
///
/// New signal kinds can be added here without breaking the scoring engine.
/// Future ML-generated signals use [`SignalKind::MlModel`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalKind {
    // ── Velocity ─────────────────────────────────────────────────────────────
    LoginVelocityExceeded,
    RegistrationVelocityExceeded,
    PaymentVelocityExceeded,
    WithdrawalVelocityExceeded,

    // ── Amount ───────────────────────────────────────────────────────────────
    UnusualPaymentAmount,
    LargeWithdrawal,

    // ── Network ──────────────────────────────────────────────────────────────
    SuspiciousIp,
    TorExitNode,
    DatacenterIp,
    BlocklistedIp,

    // ── Device / Behavioral ──────────────────────────────────────────────────
    AutomatedClient,
    NewDevice,
    DeviceFingerprintMismatch,

    // ── Geo ──────────────────────────────────────────────────────────────────
    GeoAnomaly,
    HighRiskCountry,

    // ── Account ──────────────────────────────────────────────────────────────
    NewAccount,
    AccountAgeAnomaly,

    // ── ML / Future ──────────────────────────────────────────────────────────
    /// Reserved for scores emitted by an ML anti-fraud model.
    MlModel,
}

impl SignalKind {
    /// Base severity weight (1–10). Rules may override this per-signal.
    pub fn base_severity(self) -> u8 {
        match self {
            Self::LoginVelocityExceeded => 7,
            Self::RegistrationVelocityExceeded => 6,
            Self::PaymentVelocityExceeded => 8,
            Self::WithdrawalVelocityExceeded => 9,
            Self::UnusualPaymentAmount => 5,
            Self::LargeWithdrawal => 6,
            Self::SuspiciousIp => 6,
            Self::TorExitNode => 8,
            Self::DatacenterIp => 4,
            Self::BlocklistedIp => 10,
            Self::AutomatedClient => 7,
            Self::NewDevice => 3,
            Self::DeviceFingerprintMismatch => 7,
            Self::GeoAnomaly => 7,
            Self::HighRiskCountry => 6,
            Self::NewAccount => 2,
            Self::AccountAgeAnomaly => 4,
            Self::MlModel => 5,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::LoginVelocityExceeded => "login_velocity_exceeded",
            Self::RegistrationVelocityExceeded => "registration_velocity_exceeded",
            Self::PaymentVelocityExceeded => "payment_velocity_exceeded",
            Self::WithdrawalVelocityExceeded => "withdrawal_velocity_exceeded",
            Self::UnusualPaymentAmount => "unusual_payment_amount",
            Self::LargeWithdrawal => "large_withdrawal",
            Self::SuspiciousIp => "suspicious_ip",
            Self::TorExitNode => "tor_exit_node",
            Self::DatacenterIp => "datacenter_ip",
            Self::BlocklistedIp => "blocklisted_ip",
            Self::AutomatedClient => "automated_client",
            Self::NewDevice => "new_device",
            Self::DeviceFingerprintMismatch => "device_fingerprint_mismatch",
            Self::GeoAnomaly => "geo_anomaly",
            Self::HighRiskCountry => "high_risk_country",
            Self::NewAccount => "new_account",
            Self::AccountAgeAnomaly => "account_age_anomaly",
            Self::MlModel => "ml_model",
        }
    }
}

/// A single fraud signal emitted by a rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FraudSignal {
    pub kind: SignalKind,
    /// Effective severity (1–10), may differ from `kind.base_severity()`.
    pub severity: u8,
    /// Human-readable explanation for audit logs.
    pub detail: String,
}

impl FraudSignal {
    pub fn new(kind: SignalKind, detail: impl Into<String>) -> Self {
        Self {
            severity: kind.base_severity(),
            kind,
            detail: detail.into(),
        }
    }

    /// Override the default severity.
    pub fn with_severity(mut self, severity: u8) -> Self {
        self.severity = severity.clamp(1, 10);
        self
    }
}
