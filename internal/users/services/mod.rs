use std::sync::Arc;

use chrono::{Duration, Utc};
use redis::aio::ConnectionManager;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use crate::internal::users::audit::{UserAuditContext, UserAuditEventType, UserAuditRepository};
use crate::internal::users::error::{format_validation, UserError, UserResult};
use crate::internal::users::models::{
    AccountDeletionRequest, AccountDeletionResponse, UpdateProfileRequest, UserProfileResponse,
    UserRole,
};
use crate::internal::users::repository::UserRepository;

/// Production-grade user service with optimistic locking, audit, deletion lifecycle and RBAC.
#[derive(Clone)]
pub struct UserService {
    repo: UserRepository,
    audit: UserAuditRepository,
    #[allow(dead_code)]
    redis: ConnectionManager,
}

impl UserService {
    pub fn new(pool: PgPool, redis: ConnectionManager) -> Self {
        Self {
            repo: UserRepository::new(pool.clone()),
            audit: UserAuditRepository::new(pool),
            redis,
        }
    }

    // ==================== Public profile operations ====================

    pub async fn get_profile(&self, user_id: Uuid) -> UserResult<UserProfileResponse> {
        self.repo.ensure_profile_defaults(user_id).await?;
        let user = self
            .repo
            .find_by_id(user_id)
            .await?
            .ok_or(UserError::NotFound)?;
        Ok(user.into())
    }

    /// Update profile with optimistic locking.
    /// The caller must pass the `version` they last saw (from GET or previous response).
    pub async fn update_profile(
        &self,
        user_id: Uuid,
        expected_version: i64,
        req: UpdateProfileRequest,
        actor: Option<Uuid>, // usually the same as user_id, or an admin
        ctx: UserAuditContext,
    ) -> UserResult<UserProfileResponse> {
        req.validate()
            .map_err(|e| UserError::Validation(format_validation(&e)))?;

        let old_user = self
            .repo
            .find_by_id(user_id)
            .await?
            .ok_or(UserError::NotFound)?;

        let updated = self
            .repo
            .update_profile_with_version(
                user_id,
                expected_version,
                req.display_name.as_deref(),
                req.timezone.as_deref(),
                req.locale.as_deref(),
                req.avatar_url.as_deref(),
                req.preferences.as_ref(),
            )
            .await?;

        // Audit the change
        let _ = self
            .audit
            .record(
                &ctx,
                UserAuditEventType::ProfileUpdated,
                true,
                Some(json!({
                    "display_name": old_user.display_name,
                    "timezone": old_user.timezone,
                    "locale": old_user.locale,
                    "avatar_url": old_user.avatar_url,
                })),
                Some(json!({
                    "display_name": updated.display_name,
                    "timezone": updated.timezone,
                    "locale": updated.locale,
                    "avatar_url": updated.avatar_url,
                    "preferences": updated.preferences,
                })),
                json!({ "actor": actor }),
            )
            .await;

        Ok(updated.into())
    }

    // ==================== Account deletion (GDPR) ====================

    pub async fn request_account_deletion(
        &self,
        user_id: Uuid,
        req: AccountDeletionRequest,
        ctx: UserAuditContext,
    ) -> UserResult<AccountDeletionResponse> {
        req.validate()
            .map_err(|e| UserError::Validation(format_validation(&e)))?;

        let default_grace = Duration::days(30);
        let scheduled = req
            .scheduled_at
            .unwrap_or_else(|| Utc::now() + default_grace);

        let user = self.repo.request_deletion(user_id, Some(scheduled)).await?;

        let _ = self
            .audit
            .record(
                &ctx,
                UserAuditEventType::AccountDeletionRequested,
                true,
                None,
                Some(json!({ "scheduled_at": scheduled })),
                json!({ "reason": req.reason }),
            )
            .await;

        Ok(AccountDeletionResponse {
            deletion_requested_at: user.deletion_requested_at.unwrap_or_else(Utc::now),
            deletion_scheduled_at: user.deletion_scheduled_at,
            message: "Account deletion scheduled. You can cancel it before the scheduled date.",
        })
    }

    pub async fn cancel_account_deletion(
        &self,
        user_id: Uuid,
        ctx: UserAuditContext,
    ) -> UserResult<UserProfileResponse> {
        let user = self.repo.cancel_deletion(user_id).await?;

        let _ = self
            .audit
            .record(
                &ctx,
                UserAuditEventType::AccountDeletionCancelled,
                true,
                None,
                None,
                json!({}),
            )
            .await;

        Ok(user.into())
    }

    // ==================== Admin operations (require elevated role) ====================

    pub async fn admin_suspend_user(
        &self,
        target_user_id: Uuid,
        actor_user_id: Uuid,
        reason: Option<String>,
        ctx: UserAuditContext,
    ) -> UserResult<UserProfileResponse> {
        let user = self.repo.suspend(target_user_id).await?;

        let _ = self
            .audit
            .record(
                &ctx,
                UserAuditEventType::AccountSuspended,
                true,
                None,
                Some(json!({ "reason": reason })),
                json!({ "actor": actor_user_id }),
            )
            .await;

        Ok(user.into())
    }

    pub async fn admin_reactivate_user(
        &self,
        target_user_id: Uuid,
        actor_user_id: Uuid,
        ctx: UserAuditContext,
    ) -> UserResult<UserProfileResponse> {
        let user = self.repo.reactivate(target_user_id).await?;

        let _ = self
            .audit
            .record(
                &ctx,
                UserAuditEventType::AccountReactivated,
                true,
                None,
                None,
                json!({ "actor": actor_user_id }),
            )
            .await;

        Ok(user.into())
    }

    /// Force anonymization (right to be forgotten / compliance)
    pub async fn admin_force_anonymize(
        &self,
        target_user_id: Uuid,
        actor_user_id: Uuid,
        immediate: bool,
        ctx: UserAuditContext,
    ) -> UserResult<()> {
        self.repo.anonymize(target_user_id).await?;

        let _ = self
            .audit
            .record(
                &ctx,
                UserAuditEventType::AccountAnonymized,
                true,
                None,
                Some(json!({ "immediate": immediate })),
                json!({ "actor": actor_user_id }),
            )
            .await;

        Ok(())
    }

    // ==================== Helpers ====================

    pub async fn ensure_defaults(&self, user_id: Uuid) -> UserResult<()> {
        self.repo.ensure_profile_defaults(user_id).await
    }

    pub fn new_audit_context(
        &self,
        user_id: Uuid,
        actor_user_id: Option<Uuid>,
        ip: Option<String>,
        user_agent: Option<String>,
    ) -> UserAuditContext {
        UserAuditContext {
            user_id,
            actor_user_id,
            ip,
            user_agent,
        }
    }
}

/// Wrapper for AppState
#[derive(Clone)]
pub struct UserServiceHandle(pub Arc<UserService>);

impl UserServiceHandle {
    pub fn new(pool: PgPool, redis: ConnectionManager) -> Self {
        Self(Arc::new(UserService::new(pool, redis)))
    }
}

impl std::ops::Deref for UserServiceHandle {
    type Target = UserService;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
