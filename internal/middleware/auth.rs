//! JWT authentication middleware — validates Bearer tokens and sets [`AuthenticatedUser`].

use axum::extract::{Request, State};
use axum::http::header::AUTHORIZATION;
use axum::middleware::Next;
use axum::response::Response;

use crate::internal::auth::{AuthError, AuthenticatedUser};
use crate::internal::state::AppState;

/// Requires a valid access JWT; rejects unauthenticated requests.
pub async fn require_auth(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AuthError> {
    let token = extract_bearer(req.headers())?;
    let claims = state.auth.verify_access_token(&token).await?;
    req.extensions_mut().insert(AuthenticatedUser { claims });
    Ok(next.run(req).await)
}

pub fn extract_bearer(headers: &axum::http::HeaderMap) -> Result<String, AuthError> {
    let header = headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(AuthError::Unauthorized)?;

    let token = header
        .strip_prefix("Bearer ")
        .or_else(|| header.strip_prefix("bearer "))
        .ok_or(AuthError::Unauthorized)?;

    if token.is_empty() {
        return Err(AuthError::Unauthorized);
    }

    Ok(token.to_string())
}
