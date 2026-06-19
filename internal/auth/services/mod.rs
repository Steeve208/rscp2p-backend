use std::sync::Arc;

use argon2::password_hash::{
    rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
};
use argon2::Argon2;
use redis::aio::ConnectionManager;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;
use validator::Validate;

use crate::internal::auth::audit::{AuditContext, AuditEventType, AuthAuditRepository};
use crate::internal::auth::error::{format_validation, AuthError, AuthResult};
use crate::internal::auth::jwt_keys::JwtKeySet;
use crate::internal::auth::mfa::MfaService;
use crate::internal::auth::models::{
    AuthResponse, AuthUser, JwtClaims, LoginRequest, LoginResult, LogoutRequest, MeResponse,
    MfaChallengeResponse, MfaConfirmRequest, MfaDisableRequest, MfaSetupResponse,
    MfaVerifyLoginRequest, RefreshRequest, RegisterRequest, SessionInfo, SessionsResponse,
    TokenType,
};
use crate::internal::auth::password_policy::validate_password;
use crate::internal::auth::repository::AuthRepository;
use crate::internal::auth::session_store::SessionStore;
use crate::internal::config::AuthConfig;
use crate::internal::core::financial_gateway::FinancialGatewayHandle;
use crate::internal::observability::auth as auth_metrics;

#[derive(Clone)]
pub struct AuthService {
    repo: AuthRepository,
    audit: AuthAuditRepository,
    sessions: SessionStore,
    jwt: Arc<JwtKeySet>,
    mfa: MfaService,
    financial_gateway: Option<FinancialGatewayHandle>,
}

#[derive(Clone)]
pub struct RequestContext {
    pub user_agent: Option<String>,
    pub ip: Option<String>,
}

impl RequestContext {
    pub fn audit(&self, user_id: Option<Uuid>) -> AuditContext {
        AuditContext {
            user_id,
            ip: self.ip.clone(),
            user_agent: self.user_agent.clone(),
        }
    }
}

impl AuthService {
    pub fn new(
        pool: PgPool,
        redis: ConnectionManager,
        jwt: Arc<JwtConfigWrapper>,
        auth_config: AuthConfig,
    ) -> Self {
        Self {
            repo: AuthRepository::new(pool.clone()),
            audit: AuthAuditRepository::new(pool),
            sessions: SessionStore::new(redis, auth_config.clone()),
            jwt: jwt.0.clone(),
            mfa: MfaService::new(&auth_config.mfa_encryption_key, &auth_config.mfa_issuer),
            financial_gateway: None,
        }
    }

    pub fn with_financial_gateway(mut self, gateway: FinancialGatewayHandle) -> Self {
        self.financial_gateway = Some(gateway);
        self
    }

    pub fn jwks(&self) -> serde_json::Value {
        self.jwt.public_jwks()
    }

    pub async fn register(
        &self,
        req: RegisterRequest,
        ctx: RequestContext,
    ) -> AuthResult<AuthResponse> {
        req.validate()
            .map_err(|e| AuthError::Validation(format_validation(&e)))?;

        if let Err(e) = validate_password(&req.password) {
            auth_metrics::register_failure("password_policy");
            self.audit
                .record(
                    &ctx.audit(None),
                    AuditEventType::PasswordPolicyRejected,
                    false,
                    json!({ "email": req.email }),
                )
                .await?;
            return Err(e);
        }

        let password_hash = hash_password(&req.password)?;
        let user = match self.repo.create(&req.email, &password_hash).await {
            Ok(user) => user,
            Err(AuthError::EmailAlreadyExists) => {
                auth_metrics::register_failure("email_exists");
                return Err(AuthError::EmailAlreadyExists);
            }
            Err(e) => return Err(e),
        };

        self.audit
            .record(
                &ctx.audit(Some(user.id)),
                AuditEventType::Register,
                true,
                json!({}),
            )
            .await?;

        if let Some(gateway) = &self.financial_gateway {
            if let Err(e) = gateway.provision_banking_user(user.id, &user.email).await {
                tracing::error!(
                    user_id = %user.id,
                    error = %e,
                    "banking user provisioning failed — will retry on next login"
                );
            }
        }

        auth_metrics::register_success();
        self.issue_tokens(&user, &ctx).await
    }

    pub async fn login(&self, req: LoginRequest, ctx: RequestContext) -> AuthResult<LoginResult> {
        req.validate()
            .map_err(|e| AuthError::Validation(format_validation(&e)))?;

        let ip = ctx.ip.as_deref();
        self.sessions.check_login_allowed(&req.email, ip).await?;

        let user = match self.repo.find_by_email(&req.email).await? {
            Some(user) => user,
            None => {
                self.sessions.record_failed_login(&req.email, ip).await?;
                self.audit
                    .record(
                        &ctx.audit(None),
                        AuditEventType::LoginFailed,
                        false,
                        json!({ "reason": "unknown_email" }),
                    )
                    .await?;
                auth_metrics::login_failure("unknown_email");
                return Err(AuthError::InvalidCredentials);
            }
        };

        if verify_password(&req.password, &user.password_hash).is_err() {
            self.sessions.record_failed_login(&req.email, ip).await?;
                self.audit
                    .record(
                        &ctx.audit(Some(user.id)),
                        AuditEventType::LoginFailed,
                        false,
                        json!({ "reason": "bad_password" }),
                    )
                    .await?;
                auth_metrics::login_failure("bad_password");
                return Err(AuthError::InvalidCredentials);
        }

        self.sessions.clear_login_attempts(&req.email, ip).await?;

        if user.mfa_enabled {
            let challenge = self.create_mfa_challenge(&user).await?;
            self.audit
                .record(
                    &ctx.audit(Some(user.id)),
                    AuditEventType::MfaRequired,
                    true,
                    json!({}),
                )
                .await?;
            auth_metrics::mfa_required();
            return Ok(LoginResult::MfaRequired(challenge));
        }

        let tokens = self.issue_tokens(&user, &ctx).await?;
        self.audit
            .record(
                &ctx.audit(Some(user.id)),
                AuditEventType::LoginSuccess,
                true,
                json!({ "session_id": tokens.session_id }),
            )
            .await?;
        auth_metrics::login_success();
        Ok(LoginResult::Authenticated(tokens))
    }

    pub async fn verify_mfa_login(
        &self,
        req: MfaVerifyLoginRequest,
        ctx: RequestContext,
    ) -> AuthResult<AuthResponse> {
        req.validate()
            .map_err(|e| AuthError::Validation(format_validation(&e)))?;

        let claims = self
            .jwt
            .decode_token(&req.challenge_token, TokenType::MfaChallenge)?;
        if self.sessions.is_blacklisted(&claims.jti).await? {
            return Err(AuthError::InvalidToken);
        }

        let user_id = self
            .sessions
            .consume_mfa_challenge(&claims.jti)
            .await?
            .ok_or(AuthError::InvalidToken)?;

        if user_id != claims.sub {
            return Err(AuthError::InvalidToken);
        }

        let user = self
            .repo
            .find_by_id(user_id)
            .await?
            .ok_or(AuthError::InvalidCredentials)?;

        let secret = user.mfa_secret.as_ref().ok_or(AuthError::InvalidToken)?;
        if !self.mfa.verify_code(secret, &user.email, &req.code)? {
            self.audit
                .record(
                    &ctx.audit(Some(user.id)),
                    AuditEventType::MfaVerified,
                    false,
                    json!({}),
                )
                .await?;
            return Err(AuthError::InvalidCredentials);
        }

        let ttl = self.sessions.access_blacklist_ttl(&claims);
        self.sessions.blacklist_jti(&claims.jti, ttl).await?;

        let tokens = self.issue_tokens(&user, &ctx).await?;
        self.audit
            .record(
                &ctx.audit(Some(user.id)),
                AuditEventType::MfaVerified,
                true,
                json!({ "session_id": tokens.session_id }),
            )
            .await?;
        self.audit
            .record(
                &ctx.audit(Some(user.id)),
                AuditEventType::LoginSuccess,
                true,
                json!({ "mfa": true }),
            )
            .await?;
        auth_metrics::login_success();
        Ok(tokens)
    }

    pub async fn mfa_setup(
        &self,
        user_id: Uuid,
        ctx: RequestContext,
    ) -> AuthResult<MfaSetupResponse> {
        let user = self
            .repo
            .find_by_id(user_id)
            .await?
            .ok_or(AuthError::Unauthorized)?;

        let (encrypted, secret, otpauth_url) = self.mfa.generate_secret(&user.email)?;
        self.repo.set_mfa_pending(user.id, &encrypted).await?;

        self.audit
            .record(
                &ctx.audit(Some(user.id)),
                AuditEventType::MfaSetup,
                true,
                json!({}),
            )
            .await?;

        Ok(MfaSetupResponse {
            secret,
            otpauth_url,
        })
    }

    pub async fn mfa_confirm(
        &self,
        user_id: Uuid,
        req: MfaConfirmRequest,
        ctx: RequestContext,
    ) -> AuthResult<()> {
        req.validate()
            .map_err(|e| AuthError::Validation(format_validation(&e)))?;

        let user = self
            .repo
            .find_by_id(user_id)
            .await?
            .ok_or(AuthError::Unauthorized)?;

        let pending = user
            .mfa_pending_secret
            .as_ref()
            .ok_or(AuthError::Validation("mfa setup not started".into()))?;

        if !self.mfa.verify_code(pending, &user.email, &req.code)? {
            return Err(AuthError::InvalidCredentials);
        }

        self.repo.enable_mfa(user.id).await?;
        self.audit
            .record(
                &ctx.audit(Some(user.id)),
                AuditEventType::MfaEnabled,
                true,
                json!({}),
            )
            .await?;
        Ok(())
    }

    pub async fn mfa_disable(
        &self,
        user_id: Uuid,
        req: MfaDisableRequest,
        ctx: RequestContext,
    ) -> AuthResult<()> {
        req.validate()
            .map_err(|e| AuthError::Validation(format_validation(&e)))?;

        let user = self
            .repo
            .find_by_id(user_id)
            .await?
            .ok_or(AuthError::Unauthorized)?;

        verify_password(&req.password, &user.password_hash)?;
        let secret = user
            .mfa_secret
            .as_ref()
            .ok_or(AuthError::Validation("mfa is not enabled".into()))?;
        if !self.mfa.verify_code(secret, &user.email, &req.code)? {
            return Err(AuthError::InvalidCredentials);
        }

        self.repo.disable_mfa(user.id).await?;
        self.audit
            .record(
                &ctx.audit(Some(user.id)),
                AuditEventType::MfaDisabled,
                true,
                json!({}),
            )
            .await?;
        Ok(())
    }

    pub async fn refresh(
        &self,
        req: RefreshRequest,
        ctx: RequestContext,
    ) -> AuthResult<AuthResponse> {
        req.validate()
            .map_err(|e| AuthError::Validation(format_validation(&e)))?;

        let claims = self
            .jwt
            .decode_token(&req.refresh_token, TokenType::Refresh)?;
        self.ensure_token_active(&claims).await?;

        let session = self
            .sessions
            .get_session(claims.sid)
            .await?
            .ok_or(AuthError::InvalidToken)?;

        if session.refresh_jti != claims.jti {
            return Err(AuthError::InvalidToken);
        }

        let user = self
            .repo
            .find_by_id(claims.sub)
            .await?
            .ok_or(AuthError::InvalidCredentials)?;

        let ttl = self
            .sessions
            .refresh_ttl_secs(self.jwt.config.refresh_expiry_days);
        self.sessions.blacklist_jti(&claims.jti, ttl).await?;

        let (refresh_token, refresh_claims) =
            self.jwt
                .encode_token(&user, TokenType::Refresh, session.id)?;
        self.sessions
            .update_refresh_jti(session.id, &refresh_claims.jti, ttl)
            .await?;

        let (access_token, _) = self
            .jwt
            .encode_token(&user, TokenType::Access, session.id)?;
        self.sessions.touch_session(session.id, ttl).await?;

        self.audit
            .record(
                &ctx.audit(Some(user.id)),
                AuditEventType::Refresh,
                true,
                json!({ "session_id": session.id }),
            )
            .await?;

        Ok(AuthResponse {
            access_token,
            refresh_token,
            token_type: "Bearer",
            expires_in: self.jwt.config.expiry_hours * 3600,
            session_id: session.id,
        })
    }

    pub async fn logout(
        &self,
        access_claims: &JwtClaims,
        body: LogoutRequest,
        ctx: RequestContext,
    ) -> AuthResult<()> {
        self.revoke_session(access_claims.sid).await?;

        if let Some(refresh) = body.refresh_token {
            if let Ok(claims) = self.jwt.decode_token(&refresh, TokenType::Refresh) {
                let ttl = self
                    .sessions
                    .refresh_ttl_secs(self.jwt.config.refresh_expiry_days);
                self.sessions.blacklist_jti(&claims.jti, ttl).await?;
            }
        }

        let access_ttl = self.sessions.access_blacklist_ttl(access_claims);
        self.sessions
            .blacklist_jti(&access_claims.jti, access_ttl)
            .await?;

        self.audit
            .record(
                &ctx.audit(Some(access_claims.sub)),
                AuditEventType::Logout,
                true,
                json!({ "session_id": access_claims.sid }),
            )
            .await?;
        Ok(())
    }

    pub async fn logout_all(
        &self,
        user_id: Uuid,
        access_claims: &JwtClaims,
        ctx: RequestContext,
    ) -> AuthResult<()> {
        let sessions = self.sessions.list_user_sessions(user_id).await?;
        let ttl = self
            .sessions
            .refresh_ttl_secs(self.jwt.config.refresh_expiry_days);
        for session in &sessions {
            self.sessions
                .blacklist_jti(&session.refresh_jti, ttl)
                .await?;
        }
        let access_ttl = self.sessions.access_blacklist_ttl(access_claims);
        self.sessions
            .blacklist_jti(&access_claims.jti, access_ttl)
            .await?;
        self.sessions.revoke_all_user_sessions(user_id).await?;

        self.audit
            .record(
                &ctx.audit(Some(user_id)),
                AuditEventType::LogoutAll,
                true,
                json!({}),
            )
            .await?;
        Ok(())
    }

    pub async fn me(&self, claims: &JwtClaims) -> AuthResult<MeResponse> {
        let user = self
            .repo
            .find_by_id(claims.sub)
            .await?
            .ok_or(AuthError::Unauthorized)?;

        Ok(MeResponse {
            id: user.id,
            email: user.email,
            created_at: user.created_at,
            mfa_enabled: user.mfa_enabled,
        })
    }

    pub async fn list_sessions(&self, claims: &JwtClaims) -> AuthResult<SessionsResponse> {
        let sessions = self.sessions.list_user_sessions(claims.sub).await?;
        let mapped = sessions
            .into_iter()
            .map(|s| SessionInfo {
                id: s.id,
                created_at: chrono::DateTime::from_timestamp(s.created_at, 0)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(chrono::Utc::now),
                last_used_at: chrono::DateTime::from_timestamp(s.last_used_at, 0)
                    .map(|dt| dt.with_timezone(&chrono::Utc))
                    .unwrap_or_else(chrono::Utc::now),
                user_agent: s.user_agent,
                ip: s.ip,
                current: s.id == claims.sid,
            })
            .collect();

        Ok(SessionsResponse { sessions: mapped })
    }

    pub async fn revoke_session_for_user(
        &self,
        user_id: Uuid,
        session_id: Uuid,
        current_sid: Uuid,
        ctx: RequestContext,
    ) -> AuthResult<()> {
        let session = self
            .sessions
            .get_session(session_id)
            .await?
            .ok_or(AuthError::SessionNotFound)?;

        if session.user_id != user_id {
            return Err(AuthError::Unauthorized);
        }

        if session_id == current_sid {
            return Err(AuthError::Validation(
                "cannot revoke the current session; use logout instead".into(),
            ));
        }

        self.revoke_session(session_id).await?;
        self.audit
            .record(
                &ctx.audit(Some(user_id)),
                AuditEventType::SessionRevoked,
                true,
                json!({ "session_id": session_id }),
            )
            .await?;
        Ok(())
    }

    pub async fn verify_access_token(&self, token: &str) -> AuthResult<JwtClaims> {
        let claims = self.jwt.decode_token(token, TokenType::Access)?;
        if let Err(e) = self.ensure_token_active(&claims).await {
            return Err(e);
        }
        Ok(claims)
    }

    pub async fn audit_events(
        &self,
        user_id: Uuid,
    ) -> AuthResult<Vec<crate::internal::auth::audit::AuditEventRow>> {
        self.audit.recent_for_user(user_id, 50).await
    }

    async fn create_mfa_challenge(&self, user: &AuthUser) -> AuthResult<MfaChallengeResponse> {
        let sid = Uuid::nil();
        let (challenge_token, claims) =
            self.jwt.encode_token(user, TokenType::MfaChallenge, sid)?;
        let ttl = self.jwt.config.mfa_challenge_minutes * 60;
        self.sessions
            .store_mfa_challenge(&claims.jti, user.id, ttl)
            .await?;
        Ok(MfaChallengeResponse {
            mfa_required: true,
            challenge_token,
            expires_in: ttl,
        })
    }

    async fn ensure_token_active(&self, claims: &JwtClaims) -> AuthResult<()> {
        if self.sessions.is_blacklisted(&claims.jti).await? {
            return Err(AuthError::InvalidToken);
        }

        if claims.typ == TokenType::MfaChallenge {
            return Ok(());
        }

        let session = self
            .sessions
            .get_session(claims.sid)
            .await?
            .ok_or(AuthError::InvalidToken)?;

        if session.user_id != claims.sub {
            return Err(AuthError::InvalidToken);
        }

        Ok(())
    }

    async fn issue_tokens(
        &self,
        user: &AuthUser,
        ctx: &RequestContext,
    ) -> AuthResult<AuthResponse> {
        let ttl = self
            .sessions
            .refresh_ttl_secs(self.jwt.config.refresh_expiry_days);
        let session_id = Uuid::new_v4();

        let (refresh_token, refresh_claims) =
            self.jwt
                .encode_token(user, TokenType::Refresh, session_id)?;

        self.sessions
            .create_session(
                session_id,
                user.id,
                &user.email,
                &refresh_claims.jti,
                ctx.user_agent.clone(),
                ctx.ip.clone(),
                ttl,
            )
            .await?;

        let (access_token, _) = self.jwt.encode_token(user, TokenType::Access, session_id)?;

        Ok(AuthResponse {
            access_token,
            refresh_token,
            token_type: "Bearer",
            expires_in: self.jwt.config.expiry_hours * 3600,
            session_id,
        })
    }

    async fn revoke_session(&self, session_id: Uuid) -> AuthResult<()> {
        if let Some(session) = self.sessions.get_session(session_id).await? {
            let ttl = self
                .sessions
                .refresh_ttl_secs(self.jwt.config.refresh_expiry_days);
            self.sessions
                .blacklist_jti(&session.refresh_jti, ttl)
                .await?;
            self.sessions.delete_session(session_id).await?;
        }
        Ok(())
    }
}

/// Wrapper so AppState can hold Arc<JwtKeySet> built from config.
pub struct JwtConfigWrapper(pub Arc<JwtKeySet>);

impl JwtConfigWrapper {
    pub fn new(config: &crate::internal::config::JwtConfig) -> Self {
        Self(Arc::new(JwtKeySet::from_config(config)))
    }
}

fn hash_password(password: &str) -> AuthResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AuthError::Internal(anyhow::anyhow!("{e}")))?;
    Ok(hash.to_string())
}

fn verify_password(password: &str, password_hash: &str) -> AuthResult<()> {
    let parsed = PasswordHash::new(password_hash).map_err(|_| AuthError::InvalidCredentials)?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .map_err(|_| AuthError::InvalidCredentials)
}
