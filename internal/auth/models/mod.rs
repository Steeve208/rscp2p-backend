use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::internal::users::UserRole;

/// POST /auth/login
#[derive(Debug, Deserialize, Validate)]
pub struct LoginRequest {
    #[validate(email(message = "invalid email format"))]
    pub email: String,
    pub password: String,
}

/// POST /auth/register
#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(email(message = "invalid email format"))]
    pub email: String,
    pub password: String,
}

/// POST /auth/refresh
#[derive(Debug, Deserialize, Validate)]
pub struct RefreshRequest {
    #[validate(length(min = 1, message = "refresh_token is required"))]
    pub refresh_token: String,
}

/// POST /auth/logout (optional refresh revocation)
#[derive(Debug, Deserialize, Default)]
pub struct LogoutRequest {
    pub refresh_token: Option<String>,
}

/// POST /auth/mfa/verify-login
#[derive(Debug, Deserialize, Validate)]
pub struct MfaVerifyLoginRequest {
    #[validate(length(min = 1, message = "challenge_token is required"))]
    pub challenge_token: String,
    #[validate(length(min = 6, max = 8, message = "invalid mfa code"))]
    pub code: String,
}

/// POST /auth/mfa/confirm
#[derive(Debug, Deserialize, Validate)]
pub struct MfaConfirmRequest {
    #[validate(length(min = 6, max = 8, message = "invalid mfa code"))]
    pub code: String,
}

/// POST /auth/mfa/disable
#[derive(Debug, Deserialize, Validate)]
pub struct MfaDisableRequest {
    pub password: String,
    #[validate(length(min = 6, max = 8, message = "invalid mfa code"))]
    pub code: String,
}

/// JWT payload (access + refresh + mfa challenge).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JwtClaims {
    pub sub: Uuid,
    pub email: String,
    pub typ: TokenType,
    /// User roles embedded at token issuance (e.g. `["user"]`, `["admin"]`).
    pub roles: Vec<String>,
    pub jti: String,
    pub sid: Uuid,
    pub kid: String,
    pub exp: usize,
    pub iat: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenType {
    Access,
    Refresh,
    MfaChallenge,
}

/// API response with token pair.
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: &'static str,
    pub expires_in: u64,
    pub session_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct MfaChallengeResponse {
    pub mfa_required: bool,
    pub challenge_token: String,
    pub expires_in: u64,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum LoginResult {
    Authenticated(AuthResponse),
    MfaRequired(MfaChallengeResponse),
}

#[derive(Debug, Serialize)]
pub struct MfaSetupResponse {
    pub secret: String,
    pub otpauth_url: String,
}

/// GET /auth/me
#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub id: Uuid,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub mfa_enabled: bool,
}

/// GET /auth/sessions
#[derive(Debug, Serialize)]
pub struct SessionInfo {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub last_used_at: DateTime<Utc>,
    pub user_agent: Option<String>,
    pub ip: Option<String>,
    pub current: bool,
}

#[derive(Debug, Serialize)]
pub struct SessionsResponse {
    pub sessions: Vec<SessionInfo>,
}

/// User row mapped from PostgreSQL (repository layer).
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub mfa_enabled: bool,
    pub mfa_secret: Option<String>,
    pub mfa_pending_secret: Option<String>,
    pub role: UserRole,
}

/// Session stored in Redis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub email: String,
    pub created_at: i64,
    pub last_used_at: i64,
    pub user_agent: Option<String>,
    pub ip: Option<String>,
    pub refresh_jti: String,
}

/// Authenticated request context (from JWT middleware / extractor).
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub claims: JwtClaims,
}
