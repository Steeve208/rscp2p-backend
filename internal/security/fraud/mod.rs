//! Fraud detection engine — rule-based with extensibility hooks for ML.
//!
//! ## Architecture
//!
//! ```text
//! FraudContext ──► FraudEngine ──► [Rule1, Rule2, …] ──► FraudAssessment
//!                                                              │
//!                                              score + decision + signals
//! ```
//!
//! ## Score → Decision thresholds
//!
//! | Score  | Decision      |
//! |--------|---------------|
//! | 0–29   | Allow         |
//! | 30–59  | Monitor       |
//! | 60–79  | ChallengeMfa  |
//! | 80–100 | Block         |
//!
//! ## Adding a new rule
//!
//! 1. Implement [`rules::FraudRule`].
//! 2. Add it to [`FraudEngine::default_rules`].
//! 3. Add new [`signals::SignalKind`] variants if needed.

pub mod behavioral;
pub mod rules;
pub mod signals;

use chrono::{DateTime, Utc};
use redis::aio::ConnectionManager;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use behavioral::DeviceFingerprint;
use rules::{AmountAnomalyRule, BotDetectionRule, FraudRule, IpReputationRule, VelocityRule};
use signals::FraudSignal;
use crate::internal::security::ip_intel::ThreatIntelStore;

// ─────────────────────────────────────────────────────────────────────────────
// Context
// ─────────────────────────────────────────────────────────────────────────────

/// The type of user action being assessed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FraudActionType {
    Login,
    Registration,
    Payment,
    Withdrawal,
    WithdrawalRequest,
    SettlementRequest,
    ApiAccess,
}

impl FraudActionType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Login => "login",
            Self::Registration => "registration",
            Self::Payment => "payment",
            Self::Withdrawal => "withdrawal",
            Self::WithdrawalRequest => "withdrawal_request",
            Self::SettlementRequest => "settlement_request",
            Self::ApiAccess => "api_access",
        }
    }
}

/// All information the engine needs to assess a request for fraud.
pub struct FraudContext {
    pub user_id: Option<Uuid>,
    pub ip: Option<String>,
    pub action: FraudActionType,
    pub amount_usd: Option<Decimal>,
    pub device: Option<DeviceFingerprint>,
    pub metadata: serde_json::Value,
}

impl FraudContext {
    pub fn new(action: FraudActionType) -> Self {
        Self {
            user_id: None,
            ip: None,
            action,
            amount_usd: None,
            device: None,
            metadata: serde_json::Value::Null,
        }
    }

    pub fn with_user(mut self, user_id: Uuid) -> Self {
        self.user_id = Some(user_id);
        self
    }

    pub fn with_ip(mut self, ip: impl Into<String>) -> Self {
        self.ip = Some(ip.into());
        self
    }

    pub fn with_device(mut self, device: DeviceFingerprint) -> Self {
        self.device = Some(device);
        self
    }

    pub fn with_amount(mut self, usd: Decimal) -> Self {
        self.amount_usd = Some(usd);
        self
    }

    pub fn with_metadata(mut self, meta: serde_json::Value) -> Self {
        self.metadata = meta;
        self
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Decision
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FraudDecision {
    /// Request is clean — proceed normally.
    Allow,
    /// Suspicious but not blocked — log and monitor.
    Monitor,
    /// Require MFA step-up before proceeding.
    ChallengeMfa,
    /// Hard block — reject the request.
    Block,
}

impl FraudDecision {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Allow => "allow",
            Self::Monitor => "monitor",
            Self::ChallengeMfa => "challenge_mfa",
            Self::Block => "block",
        }
    }

    pub fn is_blocked(self) -> bool {
        self == Self::Block
    }

    pub fn requires_mfa(self) -> bool {
        matches!(self, Self::ChallengeMfa | Self::Block)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Assessment
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FraudAssessment {
    /// Aggregate fraud score 0–100.
    pub score: u8,
    pub decision: FraudDecision,
    pub signals: Vec<FraudSignal>,
    pub assessed_at: DateTime<Utc>,
}

impl FraudAssessment {
    pub fn clean() -> Self {
        Self {
            score: 0,
            decision: FraudDecision::Allow,
            signals: vec![],
            assessed_at: Utc::now(),
        }
    }

    pub fn is_blocked(&self) -> bool {
        self.decision.is_blocked()
    }

    pub fn requires_mfa(&self) -> bool {
        self.decision.requires_mfa()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Engine
// ─────────────────────────────────────────────────────────────────────────────

const MONITOR_THRESHOLD: u8 = 30;
const CHALLENGE_THRESHOLD: u8 = 60;
const BLOCK_THRESHOLD: u8 = 80;

pub struct FraudEngine {
    rules: Vec<Box<dyn FraudRule>>,
}

impl FraudEngine {
    pub fn new(rules: Vec<Box<dyn FraudRule>>) -> Self {
        Self { rules }
    }

    /// Default production ruleset (4 rules, all composable).
    pub fn default_rules(intel: ThreatIntelStore) -> Self {
        Self::new(vec![
            Box::new(VelocityRule::for_login()),
            Box::new(IpReputationRule::new(intel)),
            Box::new(AmountAnomalyRule::default()),
            Box::new(BotDetectionRule),
        ])
    }

    /// Assess a request — runs all rules concurrently, aggregates signals.
    pub async fn assess(&self, ctx: &FraudContext, redis: &ConnectionManager) -> FraudAssessment {
        let mut all_signals: Vec<FraudSignal> = Vec::new();

        for rule in &self.rules {
            let signals = rule.evaluate(ctx, redis).await;
            if !signals.is_empty() {
                tracing::debug!(
                    rule = rule.name(),
                    count = signals.len(),
                    "fraud signals emitted"
                );
            }
            all_signals.extend(signals);
        }

        let score = compute_score(&all_signals);
        let decision = score_to_decision(score);

        if !matches!(decision, FraudDecision::Allow) {
            tracing::warn!(
                score,
                decision = decision.as_str(),
                action = ctx.action.as_str(),
                ip = ?ctx.ip,
                user_id = ?ctx.user_id,
                signals = all_signals.len(),
                "fraud assessment: non-allow decision"
            );
        }

        FraudAssessment {
            score,
            decision,
            signals: all_signals,
            assessed_at: Utc::now(),
        }
    }
}

fn compute_score(signals: &[FraudSignal]) -> u8 {
    if signals.is_empty() {
        return 0;
    }
    // Sum severity × 10, cap at 100.
    let total: u32 = signals.iter().map(|s| s.severity as u32 * 10).sum();
    total.min(100) as u8
}

fn score_to_decision(score: u8) -> FraudDecision {
    if score >= BLOCK_THRESHOLD {
        FraudDecision::Block
    } else if score >= CHALLENGE_THRESHOLD {
        FraudDecision::ChallengeMfa
    } else if score >= MONITOR_THRESHOLD {
        FraudDecision::Monitor
    } else {
        FraudDecision::Allow
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use signals::SignalKind;

    #[test]
    fn zero_signals_is_clean() {
        assert_eq!(compute_score(&[]), 0);
        assert_eq!(score_to_decision(0), FraudDecision::Allow);
    }

    #[test]
    fn blocklisted_ip_scores_100() {
        let signals = vec![FraudSignal::new(SignalKind::BlocklistedIp, "blocked")];
        let score = compute_score(&signals);
        assert_eq!(score, 100);
        assert_eq!(score_to_decision(score), FraudDecision::Block);
    }

    #[test]
    fn new_device_is_monitored() {
        let signals = vec![FraudSignal::new(SignalKind::NewDevice, "new device")];
        let score = compute_score(&signals);
        assert_eq!(score_to_decision(score), FraudDecision::Monitor);
    }

    #[test]
    fn fraud_context_builder() {
        let ctx = FraudContext::new(FraudActionType::Payment)
            .with_ip("1.2.3.4")
            .with_amount(Decimal::new(100, 0));
        assert_eq!(ctx.ip.as_deref(), Some("1.2.3.4"));
        assert_eq!(ctx.amount_usd, Some(Decimal::new(100, 0)));
    }
}
