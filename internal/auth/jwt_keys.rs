use std::collections::HashMap;

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, decode_header, encode, DecodingKey, EncodingKey, Header, Validation};
use uuid::Uuid;

use crate::internal::auth::error::{AuthError, AuthResult};
use crate::internal::auth::models::{AuthUser, JwtClaims, TokenType};
use crate::internal::config::JwtConfig;

#[derive(Clone)]
pub struct JwtKeySet {
    current_kid: String,
    keys: HashMap<String, String>,
    pub config: JwtConfig,
}

impl JwtKeySet {
    pub fn from_config(config: &JwtConfig) -> Self {
        let mut keys = HashMap::new();
        keys.insert(config.kid_current.clone(), config.secret.clone());
        if let (Some(kid), Some(secret)) = (&config.kid_previous, &config.secret_previous) {
            if !kid.is_empty() && !secret.is_empty() {
                keys.insert(kid.clone(), secret.clone());
            }
        }

        Self {
            current_kid: config.kid_current.clone(),
            keys,
            config: config.clone(),
        }
    }

    pub fn encode_token(
        &self,
        user: &AuthUser,
        typ: TokenType,
        sid: Uuid,
    ) -> AuthResult<(String, JwtClaims)> {
        let now = Utc::now();
        let exp = match typ {
            TokenType::Access => now + Duration::hours(self.config.expiry_hours as i64),
            TokenType::Refresh => now + Duration::days(self.config.refresh_expiry_days as i64),
            TokenType::MfaChallenge => {
                now + Duration::minutes(self.config.mfa_challenge_minutes as i64)
            }
        };

        let claims = JwtClaims {
            sub: user.id,
            email: user.email.clone(),
            typ,
            roles: vec![user.role.as_str().to_string()],
            jti: Uuid::new_v4().to_string(),
            sid,
            kid: self.current_kid.clone(),
            exp: exp.timestamp() as usize,
            iat: now.timestamp() as usize,
        };

        let secret =
            self.keys
                .get(&self.current_kid)
                .ok_or(AuthError::Internal(anyhow::anyhow!(
                    "missing current jwt key"
                )))?;

        let mut header = Header::default();
        header.kid = Some(self.current_kid.clone());

        let token = encode(
            &header,
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .map_err(|e| AuthError::Internal(e.into()))?;

        Ok((token, claims))
    }

    pub fn decode_token(&self, token: &str, expected_typ: TokenType) -> AuthResult<JwtClaims> {
        let header = decode_header(token).map_err(|_| AuthError::InvalidToken)?;
        let kid = header.kid.ok_or(AuthError::InvalidToken)?;
        let secret = self.keys.get(&kid).ok_or(AuthError::InvalidToken)?;

        let mut validation = Validation::default();
        validation.validate_exp = true;

        let token_data = decode::<JwtClaims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &validation,
        )
        .map_err(|_| AuthError::InvalidToken)?;

        if token_data.claims.typ != expected_typ {
            return Err(AuthError::InvalidToken);
        }

        Ok(token_data.claims)
    }

    pub fn public_jwks(&self) -> serde_json::Value {
        let keys: Vec<_> = self
            .keys
            .keys()
            .map(|kid| {
                serde_json::json!({
                    "kid": kid,
                    "alg": "HS256",
                    "use": "sig"
                })
            })
            .collect();
        serde_json::json!({ "keys": keys })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn config_with_rotation() -> JwtConfig {
        JwtConfig {
            secret: "current-secret-at-least-32-characters-long".into(),
            secret_previous: Some("previous-secret-at-least-32-characters".into()),
            kid_current: "v2".into(),
            kid_previous: Some("v1".into()),
            expiry_hours: 1,
            refresh_expiry_days: 7,
            mfa_challenge_minutes: 5,
        }
    }

    #[test]
    fn decodes_with_previous_key_after_rotation() {
        let cfg = config_with_rotation();
        let old_set = JwtKeySet {
            current_kid: "v1".into(),
            keys: HashMap::from([("v1".into(), cfg.secret_previous.clone().unwrap())]),
            config: cfg.clone(),
        };
        let user = AuthUser {
            id: Uuid::new_v4(),
            email: "u@test.com".into(),
            password_hash: "x".into(),
            created_at: Utc::now(),
            mfa_enabled: false,
            mfa_secret: None,
            mfa_pending_secret: None,
            role: crate::internal::users::UserRole::User,
        };
        let sid = Uuid::new_v4();
        let (token, _) = old_set.encode_token(&user, TokenType::Access, sid).unwrap();

        let new_set = JwtKeySet::from_config(&cfg);
        let claims = new_set.decode_token(&token, TokenType::Access).unwrap();
        assert_eq!(claims.kid, "v1");
    }
}
