use chrono::Utc;
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use uuid::Uuid;

use crate::internal::auth::error::{AuthError, AuthResult};
use crate::internal::auth::models::SessionRecord;
use crate::internal::config::AuthConfig;

const SESSION_PREFIX: &str = "auth:session:";
const USER_SESSIONS_PREFIX: &str = "auth:user_sessions:";
const BLACKLIST_PREFIX: &str = "auth:blacklist:";
const LOGIN_ATTEMPTS_PREFIX: &str = "auth:login:";
const LOGIN_IP_PREFIX: &str = "auth:login_ip:";
const MFA_CHALLENGE_PREFIX: &str = "auth:mfa_challenge:";

#[derive(Clone)]
pub struct SessionStore {
    redis: ConnectionManager,
    config: AuthConfig,
}

impl SessionStore {
    pub fn new(redis: ConnectionManager, config: AuthConfig) -> Self {
        Self { redis, config }
    }

    pub async fn create_session(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        email: &str,
        refresh_jti: &str,
        user_agent: Option<String>,
        ip: Option<String>,
        refresh_ttl_secs: u64,
    ) -> AuthResult<SessionRecord> {
        let now = Utc::now().timestamp();

        let record = SessionRecord {
            id: session_id,
            user_id,
            email: email.to_string(),
            created_at: now,
            last_used_at: now,
            user_agent,
            ip,
            refresh_jti: refresh_jti.to_string(),
        };

        self.enforce_session_limit(user_id).await?;
        self.persist_session(&record, refresh_ttl_secs).await?;

        Ok(record)
    }

    pub async fn get_session(&self, session_id: Uuid) -> AuthResult<Option<SessionRecord>> {
        let key = format!("{SESSION_PREFIX}{session_id}");
        let mut conn = self.redis.clone();
        let raw: Option<String> = conn.get(&key).await.map_err(AuthError::Redis)?;

        match raw {
            Some(json) => {
                let record: SessionRecord =
                    serde_json::from_str(&json).map_err(|e| AuthError::Internal(e.into()))?;
                Ok(Some(record))
            }
            None => Ok(None),
        }
    }

    pub async fn touch_session(&self, session_id: Uuid, refresh_ttl_secs: u64) -> AuthResult<()> {
        let mut record = self
            .get_session(session_id)
            .await?
            .ok_or(AuthError::SessionNotFound)?;

        record.last_used_at = Utc::now().timestamp();
        self.persist_session(&record, refresh_ttl_secs).await
    }

    pub async fn update_refresh_jti(
        &self,
        session_id: Uuid,
        refresh_jti: &str,
        refresh_ttl_secs: u64,
    ) -> AuthResult<()> {
        let mut record = self
            .get_session(session_id)
            .await?
            .ok_or(AuthError::SessionNotFound)?;

        record.refresh_jti = refresh_jti.to_string();
        record.last_used_at = Utc::now().timestamp();
        self.persist_session(&record, refresh_ttl_secs).await
    }

    pub async fn delete_session(&self, session_id: Uuid) -> AuthResult<()> {
        let key = format!("{SESSION_PREFIX}{session_id}");
        let user_key = match self.get_session(session_id).await? {
            Some(record) => {
                let uk = format!("{USER_SESSIONS_PREFIX}{}", record.user_id);
                Some((record.user_id, uk))
            }
            None => None,
        };

        let mut conn = self.redis.clone();
        conn.del::<_, ()>(&key).await.map_err(AuthError::Redis)?;

        if let Some((user_id, user_key)) = user_key {
            let _: () = conn
                .srem(&user_key, session_id.to_string())
                .await
                .map_err(AuthError::Redis)?;
            let _ = user_id;
        }

        Ok(())
    }

    pub async fn list_user_sessions(&self, user_id: Uuid) -> AuthResult<Vec<SessionRecord>> {
        let user_key = format!("{USER_SESSIONS_PREFIX}{user_id}");
        let mut conn = self.redis.clone();
        let ids: Vec<String> = conn.smembers(&user_key).await.map_err(AuthError::Redis)?;

        let mut sessions = Vec::new();
        for id in ids {
            if let Ok(sid) = Uuid::parse_str(&id) {
                if let Some(session) = self.get_session(sid).await? {
                    sessions.push(session);
                } else {
                    let _: () = conn.srem(&user_key, id).await.map_err(AuthError::Redis)?;
                }
            }
        }

        sessions.sort_by(|a, b| b.last_used_at.cmp(&a.last_used_at));
        Ok(sessions)
    }

    pub async fn revoke_all_user_sessions(&self, user_id: Uuid) -> AuthResult<()> {
        let sessions = self.list_user_sessions(user_id).await?;
        for session in sessions {
            self.delete_session(session.id).await?;
        }
        Ok(())
    }

    pub async fn blacklist_jti(&self, jti: &str, ttl_secs: u64) -> AuthResult<()> {
        if ttl_secs == 0 {
            return Ok(());
        }
        let key = format!("{BLACKLIST_PREFIX}{jti}");
        let mut conn = self.redis.clone();
        conn.set_ex::<_, _, ()>(&key, "1", ttl_secs)
            .await
            .map_err(AuthError::Redis)?;
        Ok(())
    }

    pub async fn is_blacklisted(&self, jti: &str) -> AuthResult<bool> {
        let key = format!("{BLACKLIST_PREFIX}{jti}");
        let mut conn = self.redis.clone();
        let exists: bool = conn.exists(&key).await.map_err(AuthError::Redis)?;
        Ok(exists)
    }

    pub async fn check_login_allowed(&self, email: &str, ip: Option<&str>) -> AuthResult<()> {
        self.check_key_allowed(&format!("{LOGIN_ATTEMPTS_PREFIX}{email}"))
            .await?;
        if let Some(ip) = ip {
            self.check_key_allowed(&format!("{LOGIN_IP_PREFIX}{ip}"))
                .await?;
        }
        Ok(())
    }

    pub async fn record_failed_login(&self, email: &str, ip: Option<&str>) -> AuthResult<()> {
        self.incr_key(&format!("{LOGIN_ATTEMPTS_PREFIX}{email}"))
            .await?;
        if let Some(ip) = ip {
            self.incr_key(&format!("{LOGIN_IP_PREFIX}{ip}")).await?;
        }
        Ok(())
    }

    pub async fn clear_login_attempts(&self, email: &str, ip: Option<&str>) -> AuthResult<()> {
        let mut conn = self.redis.clone();
        conn.del::<_, ()>(format!("{LOGIN_ATTEMPTS_PREFIX}{email}"))
            .await
            .map_err(AuthError::Redis)?;
        if let Some(ip) = ip {
            conn.del::<_, ()>(format!("{LOGIN_IP_PREFIX}{ip}"))
                .await
                .map_err(AuthError::Redis)?;
        }
        Ok(())
    }

    async fn check_key_allowed(&self, key: &str) -> AuthResult<()> {
        let mut conn = self.redis.clone();
        let attempts: u32 = conn
            .get::<_, Option<u32>>(key)
            .await
            .map_err(AuthError::Redis)?
            .unwrap_or(0);
        if attempts >= self.config.login_max_attempts {
            return Err(AuthError::TooManyRequests);
        }
        Ok(())
    }

    async fn incr_key(&self, key: &str) -> AuthResult<()> {
        let mut conn = self.redis.clone();
        let attempts: u32 = conn.incr(key, 1).await.map_err(AuthError::Redis)?;
        if attempts == 1 {
            let _: () = conn
                .expire(key, self.config.login_window_secs as i64)
                .await
                .map_err(AuthError::Redis)?;
        }
        Ok(())
    }

    async fn enforce_session_limit(&self, user_id: Uuid) -> AuthResult<()> {
        let mut sessions = self.list_user_sessions(user_id).await?;
        if sessions.len() < self.config.max_sessions_per_user as usize {
            return Ok(());
        }

        sessions.sort_by_key(|s| s.created_at);
        let to_remove = sessions.len() - self.config.max_sessions_per_user as usize + 1;
        for session in sessions.into_iter().take(to_remove) {
            self.delete_session(session.id).await?;
        }
        Ok(())
    }

    async fn persist_session(&self, record: &SessionRecord, ttl_secs: u64) -> AuthResult<()> {
        let key = format!("{SESSION_PREFIX}{}", record.id);
        let user_key = format!("{USER_SESSIONS_PREFIX}{}", record.user_id);
        let json = serde_json::to_string(record).map_err(|e| AuthError::Internal(e.into()))?;

        let mut conn = self.redis.clone();
        conn.set_ex::<_, _, ()>(&key, json, ttl_secs)
            .await
            .map_err(AuthError::Redis)?;
        let _: () = conn
            .sadd(&user_key, record.id.to_string())
            .await
            .map_err(AuthError::Redis)?;

        Ok(())
    }

    pub fn refresh_ttl_secs(&self, refresh_expiry_days: u64) -> u64 {
        refresh_expiry_days * 24 * 3600
    }

    pub async fn store_mfa_challenge(
        &self,
        jti: &str,
        user_id: Uuid,
        ttl_secs: u64,
    ) -> AuthResult<()> {
        let key = format!("{MFA_CHALLENGE_PREFIX}{jti}");
        let mut conn = self.redis.clone();
        conn.set_ex::<_, _, ()>(&key, user_id.to_string(), ttl_secs)
            .await
            .map_err(AuthError::Redis)?;
        Ok(())
    }

    pub async fn consume_mfa_challenge(&self, jti: &str) -> AuthResult<Option<Uuid>> {
        let key = format!("{MFA_CHALLENGE_PREFIX}{jti}");
        let mut conn = self.redis.clone();
        let raw: Option<String> = conn.get(&key).await.map_err(AuthError::Redis)?;
        if raw.is_some() {
            conn.del::<_, ()>(&key).await.map_err(AuthError::Redis)?;
            let id = raw.and_then(|s| Uuid::parse_str(&s).ok());
            return Ok(id);
        }
        Ok(None)
    }

    pub fn access_blacklist_ttl(&self, claims: &crate::internal::auth::models::JwtClaims) -> u64 {
        let now = Utc::now().timestamp() as usize;
        claims.exp.saturating_sub(now).max(1) as u64
    }
}
