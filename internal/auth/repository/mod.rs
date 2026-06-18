use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::internal::auth::error::{AuthError, AuthResult};
use crate::internal::auth::models::AuthUser;
use crate::internal::users::UserRole;

#[derive(Clone)]
pub struct AuthRepository {
    pool: PgPool,
}

impl AuthRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn find_by_id(&self, id: Uuid) -> AuthResult<Option<AuthUser>> {
        let row = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT id, email, password_hash, created_at,
                   mfa_enabled, mfa_secret, mfa_pending_secret, role
            FROM users
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn find_by_email(&self, email: &str) -> AuthResult<Option<AuthUser>> {
        let row = sqlx::query_as::<_, UserRow>(
            r#"
            SELECT id, email, password_hash, created_at,
                   mfa_enabled, mfa_secret, mfa_pending_secret, role
            FROM users
            WHERE email = $1
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(Into::into))
    }

    pub async fn create(&self, email: &str, password_hash: &str) -> AuthResult<AuthUser> {
        let row = sqlx::query_as::<_, UserRow>(
            r#"
            INSERT INTO users (email, password_hash)
            VALUES ($1, $2)
            RETURNING id, email, password_hash, created_at,
                      mfa_enabled, mfa_secret, mfa_pending_secret, role
            "#,
        )
        .bind(email)
        .bind(password_hash)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            if let sqlx::Error::Database(db) = &e {
                if db.constraint().is_some() {
                    return AuthError::EmailAlreadyExists;
                }
            }
            AuthError::Database(e)
        })?;

        Ok(row.into())
    }

    pub async fn set_mfa_pending(&self, user_id: Uuid, encrypted: &str) -> AuthResult<()> {
        sqlx::query(
            r#"
            UPDATE users
            SET mfa_pending_secret = $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .bind(encrypted)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn enable_mfa(&self, user_id: Uuid) -> AuthResult<()> {
        sqlx::query(
            r#"
            UPDATE users
            SET mfa_enabled = TRUE,
                mfa_secret = mfa_pending_secret,
                mfa_pending_secret = NULL,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn disable_mfa(&self, user_id: Uuid) -> AuthResult<()> {
        sqlx::query(
            r#"
            UPDATE users
            SET mfa_enabled = FALSE,
                mfa_secret = NULL,
                mfa_pending_secret = NULL,
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(user_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[derive(sqlx::FromRow)]
struct UserRow {
    id: Uuid,
    email: String,
    password_hash: String,
    created_at: DateTime<Utc>,
    mfa_enabled: bool,
    mfa_secret: Option<String>,
    mfa_pending_secret: Option<String>,
    role: String,
}

impl From<UserRow> for AuthUser {
    fn from(row: UserRow) -> Self {
        Self {
            id: row.id,
            email: row.email,
            password_hash: row.password_hash,
            created_at: row.created_at,
            mfa_enabled: row.mfa_enabled,
            mfa_secret: row.mfa_secret,
            mfa_pending_secret: row.mfa_pending_secret,
            role: parse_role(&row.role),
        }
    }
}

fn parse_role(s: &str) -> UserRole {
    match s {
        "support"       => UserRole::Support,
        "fraud_analyst" => UserRole::FraudAnalyst,
        "admin"         => UserRole::Admin,
        "system"        => UserRole::System,
        _               => UserRole::User,
    }
}
