use async_trait::async_trait;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use crate::internal::auth::error::AuthError;
use crate::internal::auth::models::AuthenticatedUser;
use crate::internal::middleware::auth::extract_bearer;
use crate::internal::state::AppState;

/// Authenticated user from middleware extension or Bearer header validation.
#[async_trait]
impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        if let Some(user) = parts.extensions.get::<AuthenticatedUser>() {
            return Ok(user.clone());
        }

        let token = extract_bearer(&parts.headers)?;
        let claims = state.auth.verify_access_token(&token).await?;
        Ok(AuthenticatedUser { claims })
    }
}
