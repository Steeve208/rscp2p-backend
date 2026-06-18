use axum::extract::{ConnectInfo, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use std::net::SocketAddr;
use validator::Validate;

use crate::internal::auth::error::{format_validation, AuthError, AuthResult};
use crate::internal::auth::models::{
    AuthenticatedUser, LoginRequest, LogoutRequest, MfaConfirmRequest, MfaDisableRequest,
    MfaVerifyLoginRequest, RefreshRequest, RegisterRequest,
};
use crate::internal::auth::services::RequestContext;
use crate::internal::security::client_ip::resolve_client_ip;
use crate::internal::state::AppState;

/// Public auth routes (no JWT required).
pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/refresh", post(refresh))
        .route("/mfa/verify-login", post(verify_mfa_login))
        .route("/jwks", get(jwks))
}

/// Protected auth routes (JWT required).
pub fn protected_router() -> Router<AppState> {
    Router::new()
        .route("/me", get(me))
        .route("/logout", post(logout))
        .route("/logout-all", post(logout_all))
        .route("/sessions", get(list_sessions))
        .route("/sessions/:session_id", delete(revoke_session))
        .route("/mfa/setup", post(mfa_setup))
        .route("/mfa/confirm", post(mfa_confirm))
        .route("/mfa/disable", post(mfa_disable))
        .route("/audit", get(audit_trail))
}

async fn register(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<RegisterRequest>,
) -> AuthResult<Json<crate::internal::auth::models::AuthResponse>> {
    body.validate()
        .map_err(|e| AuthError::Validation(format_validation(&e)))?;
    let ctx = request_context(&state, &headers, None);
    let response = state.auth.register(body, ctx).await?;
    // Profile defaults (timezone, locale, etc.) are ensured on first profile read
    // via UserService::get_profile for simplicity and to avoid token parsing here.
    Ok(Json(response))
}

async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Json(body): Json<LoginRequest>,
) -> AuthResult<Json<crate::internal::auth::models::LoginResult>> {
    body.validate()
        .map_err(|e| AuthError::Validation(format_validation(&e)))?;
    let ctx = request_context(&state, &headers, Some(peer));
    let response = state.auth.login(body, ctx).await?;
    Ok(Json(response))
}

async fn verify_mfa_login(
    State(state): State<AppState>,
    headers: HeaderMap,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Json(body): Json<MfaVerifyLoginRequest>,
) -> AuthResult<Json<crate::internal::auth::models::AuthResponse>> {
    body.validate()
        .map_err(|e| AuthError::Validation(format_validation(&e)))?;
    let ctx = request_context(&state, &headers, Some(peer));
    let response = state.auth.verify_mfa_login(body, ctx).await?;
    Ok(Json(response))
}

async fn refresh(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<RefreshRequest>,
) -> AuthResult<Json<crate::internal::auth::models::AuthResponse>> {
    body.validate()
        .map_err(|e| AuthError::Validation(format_validation(&e)))?;
    let ctx = request_context(&state, &headers, None);
    let response = state.auth.refresh(body, ctx).await?;
    Ok(Json(response))
}

async fn jwks(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(state.auth.jwks())
}

async fn me(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> AuthResult<Json<crate::internal::auth::models::MeResponse>> {
    let profile = state.auth.me(&user.claims).await?;
    Ok(Json(profile))
}

async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
    user: AuthenticatedUser,
    Json(body): Json<LogoutRequest>,
) -> AuthResult<StatusCode> {
    let ctx = request_context(&state, &headers, None);
    state.auth.logout(&user.claims, body, ctx).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn logout_all(
    State(state): State<AppState>,
    headers: HeaderMap,
    user: AuthenticatedUser,
) -> AuthResult<StatusCode> {
    let ctx = request_context(&state, &headers, None);
    state
        .auth
        .logout_all(user.claims.sub, &user.claims, ctx)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_sessions(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> AuthResult<Json<crate::internal::auth::models::SessionsResponse>> {
    let sessions = state.auth.list_sessions(&user.claims).await?;
    Ok(Json(sessions))
}

async fn revoke_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    user: AuthenticatedUser,
    axum::extract::Path(session_id): axum::extract::Path<uuid::Uuid>,
) -> AuthResult<StatusCode> {
    let ctx = request_context(&state, &headers, None);
    state
        .auth
        .revoke_session_for_user(user.claims.sub, session_id, user.claims.sid, ctx)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn mfa_setup(
    State(state): State<AppState>,
    headers: HeaderMap,
    user: AuthenticatedUser,
) -> AuthResult<Json<crate::internal::auth::models::MfaSetupResponse>> {
    let ctx = request_context(&state, &headers, None);
    let response = state.auth.mfa_setup(user.claims.sub, ctx).await?;
    Ok(Json(response))
}

async fn mfa_confirm(
    State(state): State<AppState>,
    headers: HeaderMap,
    user: AuthenticatedUser,
    Json(body): Json<MfaConfirmRequest>,
) -> AuthResult<StatusCode> {
    body.validate()
        .map_err(|e| AuthError::Validation(format_validation(&e)))?;
    let ctx = request_context(&state, &headers, None);
    state.auth.mfa_confirm(user.claims.sub, body, ctx).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn mfa_disable(
    State(state): State<AppState>,
    headers: HeaderMap,
    user: AuthenticatedUser,
    Json(body): Json<MfaDisableRequest>,
) -> AuthResult<StatusCode> {
    body.validate()
        .map_err(|e| AuthError::Validation(format_validation(&e)))?;
    let ctx = request_context(&state, &headers, None);
    state.auth.mfa_disable(user.claims.sub, body, ctx).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn audit_trail(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> AuthResult<Json<Vec<crate::internal::auth::audit::AuditEventRow>>> {
    let events = state.auth.audit_events(user.claims.sub).await?;
    Ok(Json(events))
}

fn request_context(
    state: &AppState,
    headers: &HeaderMap,
    peer: Option<SocketAddr>,
) -> RequestContext {
    let ip = resolve_client_ip(peer, headers, &state.config.auth.trusted_proxies);
    let user_agent = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);

    RequestContext { user_agent, ip }
}
