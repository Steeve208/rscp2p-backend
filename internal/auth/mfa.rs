use totp_rs::{Algorithm, Secret, TOTP};

use crate::internal::auth::error::{AuthError, AuthResult};
use crate::internal::security::encryption::{
    decrypt_mfa, derive_key_legacy_mfa, derive_mfa_key, encrypt, EncryptionError,
};

#[derive(Clone)]
pub struct MfaService {
    encryption_key: [u8; 32],
    legacy_encryption_key: [u8; 32],
    issuer: String,
}

impl MfaService {
    pub fn new(encryption_key_raw: &str, issuer: &str) -> Self {
        Self {
            encryption_key: derive_mfa_key(encryption_key_raw),
            legacy_encryption_key: derive_key_legacy_mfa(encryption_key_raw),
            issuer: issuer.to_string(),
        }
    }

    pub fn generate_secret(&self, email: &str) -> AuthResult<(String, String, String)> {
        let secret = Secret::generate_secret();
        let secret_base32 = secret.to_encoded().to_string();
        let encrypted = encrypt(&self.encryption_key, &secret_base32).map_err(map_encryption)?;

        let totp = build_totp(&secret_base32, &self.issuer, email)?;
        let otpauth_url = totp.get_url();

        Ok((encrypted, secret_base32, otpauth_url))
    }

    pub fn verify_code(&self, encrypted_secret: &str, email: &str, code: &str) -> AuthResult<bool> {
        let secret_base32 =
            decrypt_mfa(&self.encryption_key, &self.legacy_encryption_key, encrypted_secret)
                .map_err(map_encryption)?;
        let totp = build_totp(&secret_base32, &self.issuer, email)?;
        Ok(totp
            .check_current(code.trim())
            .map_err(|e| AuthError::Internal(e.into()))?)
    }
}

fn build_totp(secret_base32: &str, issuer: &str, account: &str) -> AuthResult<TOTP> {
    TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        Secret::Encoded(secret_base32.to_string())
            .to_bytes()
            .map_err(|e| AuthError::Internal(anyhow::anyhow!("{e}")))?,
        Some(issuer.to_string()),
        account.to_string(),
    )
    .map_err(|e| AuthError::Internal(e.into()))
}

fn map_encryption(err: EncryptionError) -> AuthError {
    AuthError::Internal(anyhow::anyhow!("{err}"))
}
