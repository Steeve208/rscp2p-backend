use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

/// User account status (lifecycle).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum UserStatus {
    Active,
    Suspended,
    PendingDeletion,
    Deleted,
}

impl Default for UserStatus {
    fn default() -> Self {
        Self::Active
    }
}

/// Basic roles for RBAC (extend as needed).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    User,
    Support,
    FraudAnalyst,
    Admin,
    System,
}

impl UserRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Support => "support",
            Self::FraudAnalyst => "fraud_analyst",
            Self::Admin => "admin",
            Self::System => "system",
        }
    }

    /// Parse a role string — returns `None` for unknown values.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "user"          => Some(Self::User),
            "support"       => Some(Self::Support),
            "fraud_analyst" => Some(Self::FraudAnalyst),
            "admin"         => Some(Self::Admin),
            "system"        => Some(Self::System),
            _               => None,
        }
    }
}

/// Core user entity returned by the users module (never contains password_hash or MFA secrets).
#[derive(Debug, Clone, Serialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub display_name: Option<String>,
    pub timezone: String,
    pub locale: String,
    pub avatar_url: Option<String>,
    pub preferences: serde_json::Value,
    pub status: UserStatus,
    pub version: i64, // optimistic locking
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub mfa_enabled: bool,

    // Deletion lifecycle (for GDPR / right to erasure)
    pub deletion_requested_at: Option<DateTime<Utc>>,
    pub deletion_scheduled_at: Option<DateTime<Utc>>,
    pub anonymized_at: Option<DateTime<Utc>>,
}

/// PATCH /users/me — partial profile update.
#[derive(Debug, Deserialize, Validate, Default)]
pub struct UpdateProfileRequest {
    #[validate(length(min = 1, max = 120, message = "display_name must be 1-120 characters"))]
    pub display_name: Option<String>,
    #[validate(length(min = 2, max = 64))]
    pub timezone: Option<String>,
    #[validate(length(min = 2, max = 16))]
    pub locale: Option<String>,
    #[validate(url(message = "avatar_url must be a valid URL"))]
    pub avatar_url: Option<String>,
    /// Free-form preferences bag. Frontend should merge, backend replaces the provided keys.
    pub preferences: Option<serde_json::Value>,
}

/// Response for GET /users/me (rich profile) — includes ETag for optimistic locking.
#[derive(Debug, Serialize)]
pub struct UserProfileResponse {
    pub id: Uuid,
    pub email: String,
    pub display_name: Option<String>,
    pub timezone: String,
    pub locale: String,
    pub avatar_url: Option<String>,
    pub preferences: serde_json::Value,
    pub mfa_enabled: bool,
    pub status: UserStatus,
    pub version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deletion_requested_at: Option<DateTime<Utc>>,
    pub deletion_scheduled_at: Option<DateTime<Utc>>,
}

impl From<User> for UserProfileResponse {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            email: u.email,
            display_name: u.display_name,
            timezone: u.timezone,
            locale: u.locale,
            avatar_url: u.avatar_url,
            preferences: u.preferences,
            mfa_enabled: u.mfa_enabled,
            status: u.status,
            version: u.version,
            created_at: u.created_at,
            updated_at: u.updated_at,
            deletion_requested_at: u.deletion_requested_at,
            deletion_scheduled_at: u.deletion_scheduled_at,
        }
    }
}

/// POST /users/me/deletion-request — request account deletion (GDPR)
#[derive(Debug, Deserialize, Validate)]
pub struct AccountDeletionRequest {
    /// Optional reason for audit / compliance
    #[validate(length(max = 500))]
    pub reason: Option<String>,
    /// When the user wants the deletion to happen (grace period). If null, use system default.
    pub scheduled_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct AccountDeletionResponse {
    pub deletion_requested_at: DateTime<Utc>,
    pub deletion_scheduled_at: Option<DateTime<Utc>>,
    pub message: &'static str,
}

/// Admin operations (skeleton)
#[derive(Debug, Deserialize)]
pub struct AdminSuspendUserRequest {
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AdminForceDeleteRequest {
    pub reason: Option<String>,
    pub immediate: bool, // if true, anonymize right away (bypass grace)
}
