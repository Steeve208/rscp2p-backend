use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

use crate::internal::users::error::{UserError, UserResult};
use crate::internal::users::models::{User, UserStatus};

#[derive(Clone)]
pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Safe projection — never returns password_hash or MFA secrets.
    pub async fn find_by_id(&self, id: Uuid) -> UserResult<Option<User>> {
        let row = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT id, email, display_name, timezone, locale, avatar_url,
                   preferences, status, version,
                   created_at, updated_at, mfa_enabled,
                   deletion_requested_at, deletion_scheduled_at, anonymized_at
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    /// Partial profile update with optimistic locking.
    /// Returns the updated user if successful, or Err(Conflict) if version mismatch.
    pub async fn update_profile_with_version(
        &self,
        id: Uuid,
        expected_version: i64,
        display_name: Option<&str>,
        timezone: Option<&str>,
        locale: Option<&str>,
        avatar_url: Option<&str>,
        preferences_patch: Option<&Value>,
    ) -> UserResult<User> {
        let current = self.find_by_id(id).await?.ok_or(UserError::NotFound)?;

        let new_preferences = if let Some(patch) = preferences_patch {
            merge_preferences(&current.preferences, patch)
        } else {
            current.preferences.clone()
        };

        // Optimistic update
        let row = sqlx::query_as::<_, UserRow>(
            r#"
            UPDATE users
            SET
                display_name = COALESCE($3, display_name),
                timezone     = COALESCE($4, timezone),
                locale       = COALESCE($5, locale),
                avatar_url   = COALESCE($6, avatar_url),
                preferences  = $7,
                updated_at   = NOW()
            WHERE id = $1 AND version = $2
            RETURNING id, email, display_name, timezone, locale, avatar_url,
                      preferences, status, version,
                      created_at, updated_at, mfa_enabled,
                      deletion_requested_at, deletion_scheduled_at, anonymized_at
            "#,
        )
        .bind(id)
        .bind(expected_version)
        .bind(display_name)
        .bind(timezone)
        .bind(locale)
        .bind(avatar_url)
        .bind(&new_preferences)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(r) => Ok(r.into()),
            None => Err(UserError::Conflict), // version was stale or user disappeared
        }
    }

    /// Request account deletion (sets deletion_requested_at and optional scheduled date)
    pub async fn request_deletion(
        &self,
        id: Uuid,
        scheduled_at: Option<DateTime<Utc>>,
    ) -> UserResult<User> {
        let row = sqlx::query_as::<_, UserRow>(
            r#"
            UPDATE users
            SET
                status = 'pending_deletion',
                deletion_requested_at = NOW(),
                deletion_scheduled_at = COALESCE($2, deletion_scheduled_at),
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, email, display_name, timezone, locale, avatar_url,
                      preferences, status, version,
                      created_at, updated_at, mfa_enabled,
                      deletion_requested_at, deletion_scheduled_at, anonymized_at
            "#,
        )
        .bind(id)
        .bind(scheduled_at)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }

    pub async fn cancel_deletion(&self, id: Uuid) -> UserResult<User> {
        let row = sqlx::query_as::<_, UserRow>(
            r#"
            UPDATE users
            SET
                status = 'active',
                deletion_requested_at = NULL,
                deletion_scheduled_at = NULL,
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, email, display_name, timezone, locale, avatar_url,
                      preferences, status, version,
                      created_at, updated_at, mfa_enabled,
                      deletion_requested_at, deletion_scheduled_at, anonymized_at
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }

    /// Admin: suspend a user
    pub async fn suspend(&self, id: Uuid) -> UserResult<User> {
        let row = sqlx::query_as::<_, UserRow>(
            r#"
            UPDATE users
            SET status = 'suspended', updated_at = NOW()
            WHERE id = $1
            RETURNING id, email, display_name, timezone, locale, avatar_url,
                      preferences, status, version,
                      created_at, updated_at, mfa_enabled,
                      deletion_requested_at, deletion_scheduled_at, anonymized_at
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }

    /// Admin: reactivate
    pub async fn reactivate(&self, id: Uuid) -> UserResult<User> {
        let row = sqlx::query_as::<_, UserRow>(
            r#"
            UPDATE users
            SET status = 'active',
                deletion_requested_at = NULL,
                deletion_scheduled_at = NULL,
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, email, display_name, timezone, locale, avatar_url,
                      preferences, status, version,
                      created_at, updated_at, mfa_enabled,
                      deletion_requested_at, deletion_scheduled_at, anonymized_at
            "#,
        )
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.into())
    }

    /// Anonymize user data (for GDPR right to be forgotten)
    pub async fn anonymize(&self, id: Uuid) -> UserResult<()> {
        sqlx::query(
            r#"
            UPDATE users
            SET
                email = concat('deleted-', id::text, '@anonymized.local'),
                display_name = NULL,
                avatar_url = NULL,
                preferences = '{}',
                status = 'deleted',
                anonymized_at = NOW(),
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Called on first profile access to ensure sane defaults
    pub async fn ensure_profile_defaults(&self, id: Uuid) -> UserResult<()> {
        sqlx::query(
            r#"
            UPDATE users
            SET timezone = COALESCE(timezone, 'UTC'),
                locale   = COALESCE(locale, 'en-US'),
                status   = COALESCE(status, 'active'),
                preferences = COALESCE(preferences, '{}'),
                version = COALESCE(version, 1)
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    email: String,
    display_name: Option<String>,
    timezone: String,
    locale: String,
    avatar_url: Option<String>,
    preferences: Value,
    status: String,
    version: i64,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    mfa_enabled: bool,
    deletion_requested_at: Option<DateTime<Utc>>,
    deletion_scheduled_at: Option<DateTime<Utc>>,
    anonymized_at: Option<DateTime<Utc>>,
}

impl From<UserRow> for User {
    fn from(row: UserRow) -> Self {
        Self {
            id: row.id,
            email: row.email,
            display_name: row.display_name,
            timezone: row.timezone,
            locale: row.locale,
            avatar_url: row.avatar_url,
            preferences: row.preferences,
            status: match row.status.as_str() {
                "suspended" => UserStatus::Suspended,
                "pending_deletion" => UserStatus::PendingDeletion,
                "deleted" => UserStatus::Deleted,
                _ => UserStatus::Active,
            },
            version: row.version,
            created_at: row.created_at,
            updated_at: row.updated_at,
            mfa_enabled: row.mfa_enabled,
            deletion_requested_at: row.deletion_requested_at,
            deletion_scheduled_at: row.deletion_scheduled_at,
            anonymized_at: row.anonymized_at,
        }
    }
}

fn merge_preferences(current: &Value, patch: &Value) -> Value {
    if !current.is_object() || !patch.is_object() {
        return patch.clone();
    }
    let mut merged = current.as_object().unwrap().clone();
    if let Some(p) = patch.as_object() {
        for (k, v) in p {
            merged.insert(k.clone(), v.clone());
        }
    }
    Value::Object(merged)
}
