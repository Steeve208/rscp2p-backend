//! AES-256-GCM envelope encryption — generic, versioned, domain-separated.
//!
//! Format: base64( version(1) || nonce(12) || ciphertext+tag )
//!
//! Domain separation in key derivation prevents cross-context key reuse:
//! `mfa`, `pii`, `webhook_secret`, etc.

use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::{engine::general_purpose::STANDARD, Engine};
use rand::RngCore;
use sha2::{Digest, Sha256};
use thiserror::Error;

const NONCE_LEN: usize = 12;
const VERSION: u8 = 1;
// AES-GCM tag is 16 bytes; minimum valid payload = version + nonce + tag
const MIN_PAYLOAD_LEN: usize = 1 + NONCE_LEN + 16;
const LEGACY_V0_MIN_PAYLOAD_LEN: usize = NONCE_LEN + 16;
const MFA_DOMAIN: &str = "mfa";

#[derive(Debug, Error)]
pub enum EncryptionError {
    #[error("encrypt failed: {0}")]
    Encrypt(String),
    #[error("decrypt failed: authentication tag mismatch or corrupted ciphertext")]
    Decrypt,
    #[error("invalid ciphertext: too short or bad base64")]
    InvalidCiphertext,
}

pub type EncryptionResult<T> = Result<T, EncryptionError>;

/// Derive a 32-byte key from `raw` + `domain` (domain-separated via SHA-256).
///
/// Each domain (`"mfa"`, `"pii"`, …) produces a distinct key even with the same
/// raw secret, preventing cross-context decryption.
pub fn derive_key(raw: &str, domain: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    hasher.update(b"\x00");
    hasher.update(domain.as_bytes());
    hasher.update(b"\x00rsc-gateway-v1");
    let digest = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

/// Encrypt `plaintext` → versioned, base64-encoded envelope.
pub fn encrypt(key: &[u8; 32], plaintext: &str) -> EncryptionResult<String> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| EncryptionError::Encrypt(e.to_string()))?;

    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| EncryptionError::Encrypt(e.to_string()))?;

    let mut payload = Vec::with_capacity(MIN_PAYLOAD_LEN + ciphertext.len());
    payload.push(VERSION);
    payload.extend_from_slice(&nonce_bytes);
    payload.extend(ciphertext);

    Ok(STANDARD.encode(payload))
}

/// Decrypt a versioned, base64-encoded envelope produced by [`encrypt`].
pub fn decrypt(key: &[u8; 32], encoded: &str) -> EncryptionResult<String> {
    let payload = STANDARD
        .decode(encoded)
        .map_err(|_| EncryptionError::InvalidCiphertext)?;

    if payload.len() < MIN_PAYLOAD_LEN {
        return Err(EncryptionError::InvalidCiphertext);
    }

    // payload[0] = version — reserved for future rotation/dispatch
    let (nonce_bytes, ciphertext) = payload[1..].split_at(NONCE_LEN);

    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|_| EncryptionError::Decrypt)?;
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| EncryptionError::Decrypt)?;

    String::from_utf8(plaintext).map_err(|_| EncryptionError::Decrypt)
}

/// Legacy MFA key derivation used by `auth/crypto.rs` before the security module.
///
/// Kept for decrypting secrets written before migration to domain-separated keys.
pub fn derive_key_legacy_mfa(raw: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    hasher.update(b"rsc-gateway-mfa-v1");
    let digest = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

/// Legacy v0 envelope: base64(nonce(12) || ciphertext) — no version byte.
pub fn decrypt_legacy_v0(key: &[u8; 32], encoded: &str) -> EncryptionResult<String> {
    let payload = STANDARD
        .decode(encoded)
        .map_err(|_| EncryptionError::InvalidCiphertext)?;

    if payload.len() < LEGACY_V0_MIN_PAYLOAD_LEN {
        return Err(EncryptionError::InvalidCiphertext);
    }

    let (nonce_bytes, ciphertext) = payload.split_at(NONCE_LEN);

    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| EncryptionError::Decrypt)?;
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| EncryptionError::Decrypt)?;

    String::from_utf8(plaintext).map_err(|_| EncryptionError::Decrypt)
}

/// Decrypt MFA secrets, accepting both current and legacy envelopes/keys.
pub fn decrypt_mfa(
    key: &[u8; 32],
    legacy_key: &[u8; 32],
    encoded: &str,
) -> EncryptionResult<String> {
    match decrypt(key, encoded) {
        Ok(plaintext) => Ok(plaintext),
        Err(_) => decrypt_legacy_v0(legacy_key, encoded),
    }
}

/// Domain-separated MFA key for new encryptions.
pub fn derive_mfa_key(raw: &str) -> [u8; 32] {
    derive_key(raw, MFA_DOMAIN)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let key = derive_key("supersecret", "mfa");
        let ciphertext = encrypt(&key, "hello world").unwrap();
        let plaintext = decrypt(&key, &ciphertext).unwrap();
        assert_eq!(plaintext, "hello world");
    }

    #[test]
    fn different_domains_different_keys() {
        let k1 = derive_key("key", "mfa");
        let k2 = derive_key("key", "pii");
        assert_ne!(k1, k2);
    }

    #[test]
    fn wrong_key_fails() {
        let key = derive_key("right_key", "mfa");
        let bad = derive_key("wrong_key", "mfa");
        let ct = encrypt(&key, "secret").unwrap();
        assert!(decrypt(&bad, &ct).is_err());
    }

    #[test]
    fn truncated_payload_fails() {
        let key = derive_key("k", "d");
        assert!(decrypt(&key, "YQ==").is_err());
    }

    #[test]
    fn mfa_key_uses_domain_separation() {
        let key = derive_mfa_key("supersecret");
        let ct = encrypt(&key, "totp-secret").unwrap();
        assert_eq!(decrypt(&key, &ct).unwrap(), "totp-secret");
    }

    #[test]
    fn decrypt_mfa_reads_legacy_v0_secrets() {
        let legacy_key = derive_key_legacy_mfa("supersecret");
        let cipher = Aes256Gcm::new_from_slice(&legacy_key).unwrap();
        let mut nonce_bytes = [0u8; NONCE_LEN];
        nonce_bytes.fill(7);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher.encrypt(nonce, &b"legacy-secret"[..]).unwrap();
        let mut payload = nonce_bytes.to_vec();
        payload.extend(ciphertext);
        let encoded = STANDARD.encode(payload);

        let current_key = derive_mfa_key("supersecret");
        assert_eq!(
            decrypt_mfa(&current_key, &legacy_key, &encoded).unwrap(),
            "legacy-secret"
        );
    }
}
