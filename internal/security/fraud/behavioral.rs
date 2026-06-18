//! Device fingerprinting and behavioral session context.
//!
//! These structs capture signals at request time and are designed to feed
//! future ML pipelines (behavioral analytics, anomaly detection).
//!
//! Current implementation: deterministic fingerprint hash from HTTP headers.
//! Future: enrich with canvas fingerprint, timing entropy, navigation path.

use axum::http::HeaderMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Signals derived from HTTP headers to identify a client device.
///
/// The `fingerprint_hash` is a stable, opaque identifier that can be stored
/// and compared across sessions to detect device changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceFingerprint {
    pub user_agent: Option<String>,
    pub accept_language: Option<String>,
    pub accept_encoding: Option<String>,
    /// SHA-256 of the above signals, hex-encoded.
    pub fingerprint_hash: String,
}

impl DeviceFingerprint {
    /// Build a fingerprint from Axum request headers.
    pub fn from_headers(headers: &HeaderMap) -> Self {
        let user_agent = header_str(headers, "user-agent").map(str::to_string);
        let accept_language = header_str(headers, "accept-language").map(str::to_string);
        let accept_encoding = header_str(headers, "accept-encoding").map(str::to_string);

        let fingerprint_hash = compute_hash(
            user_agent.as_deref(),
            accept_language.as_deref(),
            accept_encoding.as_deref(),
        );

        Self {
            user_agent,
            accept_language,
            accept_encoding,
            fingerprint_hash,
        }
    }

    /// Heuristic: detect headless browsers, crawlers, or missing UA.
    pub fn is_bot(&self) -> bool {
        let ua = self.user_agent.as_deref().unwrap_or("").to_lowercase();
        ua.is_empty()
            || ua.contains("bot")
            || ua.contains("crawler")
            || ua.contains("spider")
            || ua.contains("headless")
            || ua.contains("phantomjs")
            || ua.contains("selenium")
            || ua.contains("puppeteer")
            || ua.contains("playwright")
    }

    /// Returns `true` if this fingerprint matches `other` (same device).
    pub fn matches(&self, other: &DeviceFingerprint) -> bool {
        self.fingerprint_hash == other.fingerprint_hash
    }
}

fn compute_hash(ua: Option<&str>, lang: Option<&str>, enc: Option<&str>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(ua.unwrap_or("").as_bytes());
    hasher.update(b"\x00");
    hasher.update(lang.unwrap_or("").as_bytes());
    hasher.update(b"\x00");
    hasher.update(enc.unwrap_or("").as_bytes());
    hex::encode(hasher.finalize())
}

fn header_str<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name)?.to_str().ok()
}

// ─────────────────────────────────────────────────────────────────────────────
// Behavioral session context — built up over the lifetime of a session.
// ─────────────────────────────────────────────────────────────────────────────

/// Behavioral context for a single user session.
///
/// Stores device fingerprint and action counters now; reserved fields
/// for future ML signals are documented with `// future:` comments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehavioralContext {
    pub device: DeviceFingerprint,
    pub session_started_at: DateTime<Utc>,
    pub actions_in_session: u32,

    // future: typing_entropy: Option<f32>,
    // future: mouse_entropy: Option<f32>,
    // future: navigation_path: Vec<String>,
    // future: time_between_actions_ms: Vec<u64>,
}

impl BehavioralContext {
    pub fn new(device: DeviceFingerprint) -> Self {
        Self {
            device,
            session_started_at: Utc::now(),
            actions_in_session: 0,
        }
    }

    pub fn record_action(&mut self) {
        self.actions_in_session = self.actions_in_session.saturating_add(1);
    }

    pub fn session_age_secs(&self) -> i64 {
        (Utc::now() - self.session_started_at).num_seconds()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_is_stable() {
        let mut h = HeaderMap::new();
        h.insert("user-agent", "Mozilla/5.0".parse().unwrap());
        h.insert("accept-language", "en-US".parse().unwrap());
        let fp1 = DeviceFingerprint::from_headers(&h);
        let fp2 = DeviceFingerprint::from_headers(&h);
        assert_eq!(fp1.fingerprint_hash, fp2.fingerprint_hash);
    }

    #[test]
    fn empty_ua_is_bot() {
        let fp = DeviceFingerprint::from_headers(&HeaderMap::new());
        assert!(fp.is_bot());
    }

    #[test]
    fn headless_ua_is_bot() {
        let mut h = HeaderMap::new();
        h.insert("user-agent", "HeadlessChrome/120".parse().unwrap());
        let fp = DeviceFingerprint::from_headers(&h);
        assert!(fp.is_bot());
    }

    #[test]
    fn real_ua_is_not_bot() {
        let mut h = HeaderMap::new();
        h.insert(
            "user-agent",
            "Mozilla/5.0 (iPhone; CPU iPhone OS 17_0) AppleWebKit/605.1.15"
                .parse()
                .unwrap(),
        );
        let fp = DeviceFingerprint::from_headers(&h);
        assert!(!fp.is_bot());
    }
}
