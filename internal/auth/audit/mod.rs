use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::internal::auth::error::AuthResult;

#[derive(Clone, Copy, Debug)]
pub enum AuditEventType {
    Register,
    LoginSuccess,
    LoginFailed,
    MfaRequired,
    MfaVerified,
    MfaSetup,
    MfaEnabled,
    MfaDisabled,
    Refresh,
    Logout,
    LogoutAll,
    SessionRevoked,
    TokenRejected,
    PasswordPolicyRejected,
}

impl AuditEventType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Register => "register",
            Self::LoginSuccess => "login_success",
            Self::LoginFailed => "login_failed",
            Self::MfaRequired => "mfa_required",
            Self::MfaVerified => "mfa_verified",
            Self::MfaSetup => "mfa_setup",
            Self::MfaEnabled => "mfa_enabled",
            Self::MfaDisabled => "mfa_disabled",
            Self::Refresh => "refresh",
            Self::Logout => "logout",
            Self::LogoutAll => "logout_all",
            Self::SessionRevoked => "session_revoked",
            Self::TokenRejected => "token_rejected",
            Self::PasswordPolicyRejected => "password_policy_rejected",
        }
    }
}

#[derive(Clone)]
pub struct AuditContext {
    pub user_id: Option<Uuid>,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Clone)]
pub struct AuthAuditRepository {
    pool: PgPool,
}

impl AuthAuditRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn record(
        &self,
        ctx: &AuditContext,
        event: AuditEventType,
        success: bool,
        metadata: Value,
    ) -> AuthResult<()> {
        sqlx::query(
            r#"
            INSERT INTO auth_audit_events (user_id, event_type, success, ip, user_agent, metadata)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(ctx.user_id)
        .bind(event.as_str())
        .bind(success)
        .bind(&ctx.ip)
        .bind(&ctx.user_agent)
        .bind(metadata)
        .execute(&self.pool)
        .await?;

        tracing::info!(
            event = event.as_str(),
            success,
            user_id = ?ctx.user_id,
            ip = ?ctx.ip,
            "auth audit"
        );

        Ok(())
    }

    pub async fn recent_for_user(
        &self,
        user_id: Uuid,
        limit: i64,
    ) -> AuthResult<Vec<AuditEventRow>> {
        let rows = sqlx::query_as::<_, AuditEventRow>(
            r#"
            SELECT id, user_id, event_type, success, ip, user_agent, metadata, created_at
            FROM auth_audit_events
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct AuditEventRow {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub event_type: String,
    pub success: bool,
    pub ip: Option<String>,
    pub user_agent: Option<String>,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
}
