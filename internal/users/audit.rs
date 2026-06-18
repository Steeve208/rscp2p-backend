use chrono::Utc;
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::internal::users::error::{UserError, UserResult};

#[derive(Clone, Copy, Debug)]
pub enum UserAuditEventType {
    ProfileUpdated,
    AccountDeletionRequested,
    AccountDeletionCancelled,
    AccountAnonymized,
    AccountSuspended,
    AccountReactivated,
    AdminViewedProfile,
}

impl UserAuditEventType {
    fn as_str(self) -> &'static str {
        match self {
            Self::ProfileUpdated => "profile_updated",
            Self::AccountDeletionRequested => "account_deletion_requested",
            Self::AccountDeletionCancelled => "account_deletion_cancelled",
            Self::AccountAnonymized => "account_anonymized",
            Self::AccountSuspended => "account_suspended",
            Self::AccountReactivated => "account_reactivated",
            Self::AdminViewedProfile => "admin_viewed_profile",
        }
    }
}

#[derive(Clone)]
pub struct UserAuditContext {
    pub user_id: Uuid,
    pub actor_user_id: Option<Uuid>, // self or admin
    pub ip: Option<String>,
    pub user_agent: Option<String>,
}

#[derive(Clone)]
pub struct UserAuditRepository {
    pool: PgPool,
}

impl UserAuditRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn record(
        &self,
        ctx: &UserAuditContext,
        event: UserAuditEventType,
        success: bool,
        old_values: Option<Value>,
        new_values: Option<Value>,
        metadata: Value,
    ) -> UserResult<()> {
        sqlx::query(
            r#"
            INSERT INTO user_audit_events
                (user_id, actor_user_id, event_type, success, ip, user_agent, old_values, new_values, metadata)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
        )
        .bind(ctx.user_id)
        .bind(ctx.actor_user_id)
        .bind(event.as_str())
        .bind(success)
        .bind(&ctx.ip)
        .bind(&ctx.user_agent)
        .bind(old_values)
        .bind(new_values)
        .bind(metadata)
        .execute(&self.pool)
        .await
        .map_err(UserError::Database)?;

        tracing::info!(
            event = event.as_str(),
            user_id = %ctx.user_id,
            actor = ?ctx.actor_user_id,
            success,
            "user audit"
        );

        Ok(())
    }
}
