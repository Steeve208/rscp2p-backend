//! Fintech-grade security layer.
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`client_ip`] | Client IP resolution (proxy-aware) |
//! | [`encryption`] | AES-256-GCM envelope encryption, domain-separated key derivation |
//! | [`signature`] | HMAC-SHA256 webhook signature verification (constant-time) |
//! | [`ip_intel`] | Threat-intel feeds (Tor/datacenter), IP classification, blocklist |
//! | [`fraud`] | Rule-based fraud engine; hooks for behavioral analysis + ML |
//!
//! ## Extension points
//!
//! - **AI anti-fraud**: implement [`fraud::rules::FraudRule`] + register in `FraudEngine`.
//! - **Behavioral analysis**: populate [`fraud::behavioral::BehavioralContext`] per session.
//! - **Custom threat feeds**: set `THREAT_INTEL_*_FEED_URL` env vars or extend [`ip_intel::feeds`].

pub mod client_ip;
pub mod encryption;
pub mod fraud;
pub mod ip_intel;
pub mod signature;

// Convenience re-exports
pub use encryption::{
    decrypt, derive_key, derive_mfa_key, encrypt,
};
pub use fraud::{FraudActionType, FraudAssessment, FraudContext, FraudDecision, FraudEngine};
pub use ip_intel::ThreatIntelStore;
pub use signature::{sign as sign_webhook, verify_webhook};
