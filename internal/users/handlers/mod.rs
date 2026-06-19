use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router};

use crate::internal::auth::AuthenticatedUser;
use crate::internal::state::AppState;
use crate::internal::users::models::{
    AccountDeletionRequest, AccountDeletionResponse, UpdateProfileRequest, UserProfileResponse,
};
use crate::internal::users::{UserAuditContext, UserResult};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/me", get(get_me).patch(update_me))
        .route("/me/deletion-request", post(request_deletion))
        .route("/me/deletion-cancel", post(cancel_deletion))
}

/// Admin user management (mounted at `/admin` in `routes/users.rs`).
pub fn admin_router() -> Router<AppState> {
    Router::new()
        .route("/users/:user_id/suspend", post(admin_suspend))
        .route("/users/:user_id/reactivate", post(admin_reactivate))
        .route("/users/:user_id/anonymize", post(admin_anonymize))
}

async fn get_me(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    _headers: HeaderMap,
) -> UserResult<(HeaderMap, Json<UserProfileResponse>)> {
    let profile = state.users.get_profile(user.claims.sub).await?;

    let mut response_headers = HeaderMap::new();
    response_headers.insert(
        "ETag",
        format!("\"version-{}\"", profile.version).parse().unwrap(),
    );

    // Optional: record admin view if caller is not the owner (future)
    Ok((response_headers, Json(profile)))
}

async fn update_me(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    headers: HeaderMap,
    Json(body): Json<UpdateProfileRequest>,
) -> UserResult<Json<UserProfileResponse>> {
    // Optimistic locking via If-Match or from body (we prefer header for REST semantics)
    let expected_version = extract_version_from_if_match(&headers)
        .or_else(|| extract_version_from_body(&body))
        .ok_or_else(|| {
            crate::internal::users::UserError::Validation(
                "Missing If-Match header or version in body for optimistic update".into(),
            )
        })?;

    let ctx = build_audit_context(&state, &user, &headers);

    let updated = state
        .users
        .update_profile(
            user.claims.sub,
            expected_version,
            body,
            Some(user.claims.sub),
            ctx,
        )
        .await?;

    Ok(Json(updated))
}

async fn request_deletion(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    headers: HeaderMap,
    Json(body): Json<AccountDeletionRequest>,
) -> UserResult<Json<AccountDeletionResponse>> {
    let ctx = build_audit_context(&state, &user, &headers);

    let resp = state
        .users
        .request_account_deletion(user.claims.sub, body, ctx)
        .await?;

    Ok(Json(resp))
}

async fn cancel_deletion(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    headers: HeaderMap,
) -> UserResult<StatusCode> {
    let ctx = build_audit_context(&state, &user, &headers);

    state
        .users
        .cancel_account_deletion(user.claims.sub, ctx)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

// ==================== Admin handlers (protected by role middleware) ====================

async fn admin_suspend(
    State(state): State<AppState>,
    user: AuthenticatedUser, // the admin performing the action
    axum::extract::Path(target_id): axum::extract::Path<uuid::Uuid>,
    headers: HeaderMap,
) -> UserResult<StatusCode> {
    let ctx = build_audit_context(&state, &user, &headers);

    state
        .users
        .admin_suspend_user(target_id, user.claims.sub, None, ctx)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

async fn admin_reactivate(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    axum::extract::Path(target_id): axum::extract::Path<uuid::Uuid>,
    headers: HeaderMap,
) -> UserResult<StatusCode> {
    let ctx = build_audit_context(&state, &user, &headers);

    state
        .users
        .admin_reactivate_user(target_id, user.claims.sub, ctx)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

async fn admin_anonymize(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    axum::extract::Path(target_id): axum::extract::Path<uuid::Uuid>,
    headers: HeaderMap,
) -> UserResult<StatusCode> {
    let ctx = build_audit_context(&state, &user, &headers);

    state
        .users
        .admin_force_anonymize(target_id, user.claims.sub, true, ctx)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

// ==================== Helpers ====================

fn build_audit_context(
    _state: &AppState,
    user: &AuthenticatedUser,
    headers: &HeaderMap,
) -> UserAuditContext {
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);

    let ua = headers
        .get(axum::http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);

    UserAuditContext {
        user_id: user.claims.sub,
        actor_user_id: Some(user.claims.sub),
        ip,
        user_agent: ua,
    }
}

fn extract_version_from_if_match(headers: &HeaderMap) -> Option<i64> {
    headers
        .get("if-match")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.trim_matches('"').strip_prefix("version-"))
        .and_then(|n| n.parse::<i64>().ok())
}

fn extract_version_from_body(_body: &UpdateProfileRequest) -> Option<i64> {
    // For convenience we also accept a top-level "version" field in the future if needed.
    // Currently we rely on If-Match header (REST best practice).
    None
}
