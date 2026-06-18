//! Fraud rule trait + built-in rules.
//!
//! Each rule is independent, async, and composable. Add new rules by implementing
//! [`FraudRule`] and registering them in [`super::FraudEngine::default_rules`].

use async_trait::async_trait;
use redis::aio::ConnectionManager;
use rust_decimal::Decimal;

use crate::internal::security::fraud::signals::{FraudSignal, SignalKind};
use crate::internal::security::ip_intel::{IpClass, ThreatIntelStore};

use super::{FraudActionType, FraudContext};

// ─────────────────────────────────────────────────────────────────────────────
// Trait
// ─────────────────────────────────────────────────────────────────────────────

#[async_trait]
pub trait FraudRule: Send + Sync {
    fn name(&self) -> &'static str;

    /// Evaluate the rule against the given context.
    /// Returns zero or more signals — never panics.
    async fn evaluate(&self, ctx: &FraudContext, redis: &ConnectionManager) -> Vec<FraudSignal>;
}

// ─────────────────────────────────────────────────────────────────────────────
// Velocity Rule — Redis sliding window counter per (action × IP)
// ─────────────────────────────────────────────────────────────────────────────

pub struct VelocityRule {
    pub limit: u32,
    pub window_secs: u64,
}

impl VelocityRule {
    pub fn for_login() -> Self {
        Self { limit: 5, window_secs: 60 }
    }
    pub fn for_payment() -> Self {
        Self { limit: 10, window_secs: 3_600 }
    }
    pub fn for_withdrawal() -> Self {
        Self { limit: 3, window_secs: 3_600 }
    }
    pub fn for_registration() -> Self {
        Self { limit: 3, window_secs: 3_600 }
    }
}

#[async_trait]
impl FraudRule for VelocityRule {
    fn name(&self) -> &'static str {
        "velocity"
    }

    async fn evaluate(&self, ctx: &FraudContext, redis: &ConnectionManager) -> Vec<FraudSignal> {
        let Some(ip) = ctx.ip.as_deref() else {
            return vec![];
        };

        let window = chrono::Utc::now().timestamp() / self.window_secs as i64;
        let key = format!("fraud:vel:{}:{}:{}", ctx.action.as_str(), ip, window);

        let count: u32 = {
            let mut conn = redis.clone();
            let n: u32 = redis::cmd("INCR")
                .arg(&key)
                .query_async(&mut conn)
                .await
                .unwrap_or(0);

            if n == 1 {
                let _: () = redis::cmd("EXPIRE")
                    .arg(&key)
                    .arg(self.window_secs as i64 + 60)
                    .query_async(&mut conn)
                    .await
                    .unwrap_or(());
            }
            n
        };

        if count > self.limit {
            let kind = match ctx.action {
                FraudActionType::Login => SignalKind::LoginVelocityExceeded,
                FraudActionType::Registration => SignalKind::RegistrationVelocityExceeded,
                FraudActionType::Payment => SignalKind::PaymentVelocityExceeded,
                FraudActionType::Withdrawal | FraudActionType::WithdrawalRequest => {
                    SignalKind::WithdrawalVelocityExceeded
                }
                _ => SignalKind::PaymentVelocityExceeded,
            };
            vec![FraudSignal::new(
                kind,
                format!("{count} actions in {}s window (limit {})", self.window_secs, self.limit),
            )]
        } else {
            vec![]
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// IP Reputation Rule — blocklist + threat-intel feeds (Tor / datacenter)
// ─────────────────────────────────────────────────────────────────────────────

pub struct IpReputationRule {
    intel: ThreatIntelStore,
}

impl IpReputationRule {
    pub fn new(intel: ThreatIntelStore) -> Self {
        Self { intel }
    }
}

#[async_trait]
impl FraudRule for IpReputationRule {
    fn name(&self) -> &'static str {
        "ip_reputation"
    }

    async fn evaluate(&self, ctx: &FraudContext, redis: &ConnectionManager) -> Vec<FraudSignal> {
        let Some(ip) = ctx.ip.as_deref() else {
            return vec![];
        };

        let mut signals = vec![];

        let redis_blocked: bool = {
            let mut conn = redis.clone();
            let r: Option<String> = redis::cmd("GET")
                .arg(format!("security:blocklist:ip:{ip}"))
                .query_async(&mut conn)
                .await
                .unwrap_or(None);
            r.is_some()
        };

        if redis_blocked {
            signals.push(FraudSignal::new(
                SignalKind::BlocklistedIp,
                format!("IP {ip} is in the Redis blocklist"),
            ));
        }

        match self.intel.classify(ip).await {
            IpClass::TorExitNode => {
                signals.push(FraudSignal::new(
                    SignalKind::TorExitNode,
                    format!("Tor exit node: {ip}"),
                ));
            }
            IpClass::Datacenter => {
                signals.push(FraudSignal::new(
                    SignalKind::DatacenterIp,
                    format!("Datacenter/hosting IP: {ip}"),
                ));
            }
            _ => {}
        }

        signals
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Amount Anomaly Rule
// ─────────────────────────────────────────────────────────────────────────────

pub struct AmountAnomalyRule {
    /// USD threshold above which a transaction is considered "large".
    pub large_threshold_usd: Decimal,
}

impl Default for AmountAnomalyRule {
    fn default() -> Self {
        Self { large_threshold_usd: Decimal::new(5_000, 0) }
    }
}

#[async_trait]
impl FraudRule for AmountAnomalyRule {
    fn name(&self) -> &'static str {
        "amount_anomaly"
    }

    async fn evaluate(&self, ctx: &FraudContext, _redis: &ConnectionManager) -> Vec<FraudSignal> {
        let Some(amount) = ctx.amount_usd else {
            return vec![];
        };

        if amount >= self.large_threshold_usd {
            vec![FraudSignal::new(
                SignalKind::LargeWithdrawal,
                format!(
                    "Amount ${amount} exceeds threshold ${}",
                    self.large_threshold_usd
                ),
            )]
        } else {
            vec![]
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Bot Detection Rule — user-agent heuristics
// ─────────────────────────────────────────────────────────────────────────────

pub struct BotDetectionRule;

#[async_trait]
impl FraudRule for BotDetectionRule {
    fn name(&self) -> &'static str {
        "bot_detection"
    }

    async fn evaluate(&self, ctx: &FraudContext, _redis: &ConnectionManager) -> Vec<FraudSignal> {
        if let Some(device) = &ctx.device {
            if device.is_bot() {
                return vec![FraudSignal::new(
                    SignalKind::AutomatedClient,
                    format!(
                        "Automated client detected (UA: {:?})",
                        device.user_agent.as_deref().unwrap_or("")
                    ),
                )];
            }
        }
        vec![]
    }
}
