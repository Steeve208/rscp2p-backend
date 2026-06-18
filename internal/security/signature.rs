//! HMAC-SHA256 webhook signature verification — constant-time, provider-agnostic.
//!
//! Replaces ad-hoc secret comparisons in provider webhook handlers.
//! Providers that do HMAC signing should delegate here.

use hmac::{Hmac, Mac};
use sha2::Sha256;
use thiserror::Error;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Error)]
pub enum SignatureError {
    #[error("webhook secret not configured")]
    MissingSecret,
    #[error("invalid signature")]
    Invalid,
    #[error("malformed signature (expected hex)")]
    BadFormat,
}

pub type SignatureResult<T> = Result<T, SignatureError>;

/// Compute HMAC-SHA256(`secret`, `payload`). Returns lowercase hex string.
pub fn sign(secret: &str, payload: &[u8]) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC accepts any key length");
    mac.update(payload);
    hex::encode(mac.finalize().into_bytes())
}

/// Verify `signature` (hex) against HMAC-SHA256(`secret`, `payload`).
///
/// Uses constant-time comparison — safe against timing attacks.
pub fn verify(secret: &str, payload: &[u8], signature: &str) -> SignatureResult<()> {
    let sig_bytes = hex::decode(signature).map_err(|_| SignatureError::BadFormat)?;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC accepts any key length");
    mac.update(payload);
    mac.verify_slice(&sig_bytes).map_err(|_| SignatureError::Invalid)
}

/// Verify a webhook signature where:
/// - `secret` may be unconfigured (allowed in non-production)
/// - `signature` comes from a request header (`None` if absent)
/// - `is_production` enforces that secret + signature are always present
pub fn verify_webhook(
    secret: Option<&str>,
    payload: &[u8],
    signature: Option<&str>,
    is_production: bool,
) -> SignatureResult<()> {
    match secret {
        None if is_production => Err(SignatureError::MissingSecret),
        None => Ok(()),
        Some(s) => {
            let sig = signature.ok_or(SignatureError::Invalid)?;
            verify(s, payload, sig)
        }
    }
}

/// Verify a simple shared-secret header (non-HMAC, e.g. provider `secret` field in body).
///
/// Uses constant-time comparison via HMAC verify to avoid timing leaks.
pub fn verify_shared_secret(expected: &str, provided: &str) -> SignatureResult<()> {
    if expected.len() != provided.len() {
        return Err(SignatureError::Invalid);
    }
    // Use a fixed HMAC to get constant-time comparison
    let mut mac = HmacSha256::new_from_slice(b"rsc-gateway-constant-time-compare")
        .expect("static key");
    mac.update(expected.as_bytes());
    let expected_tag = mac.finalize().into_bytes();

    let mut mac2 = HmacSha256::new_from_slice(b"rsc-gateway-constant-time-compare")
        .expect("static key");
    mac2.update(provided.as_bytes());
    mac2.verify_slice(&expected_tag).map_err(|_| SignatureError::Invalid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_and_verify_roundtrip() {
        let secret = "whsec_test";
        let payload = b"hello webhook";
        let sig = sign(secret, payload);
        assert!(verify(secret, payload, &sig).is_ok());
    }

    #[test]
    fn wrong_secret_fails() {
        let sig = sign("correct", b"data");
        assert!(verify("wrong", b"data", &sig).is_err());
    }

    #[test]
    fn tampered_payload_fails() {
        let sig = sign("secret", b"original");
        assert!(verify("secret", b"tampered", &sig).is_err());
    }

    #[test]
    fn invalid_hex_fails() {
        assert!(matches!(
            verify("secret", b"data", "not-hex!!"),
            Err(SignatureError::BadFormat)
        ));
    }

    #[test]
    fn no_secret_allowed_in_dev() {
        assert!(verify_webhook(None, b"payload", None, false).is_ok());
    }

    #[test]
    fn no_secret_blocked_in_prod() {
        assert!(matches!(
            verify_webhook(None, b"payload", None, true),
            Err(SignatureError::MissingSecret)
        ));
    }
}
