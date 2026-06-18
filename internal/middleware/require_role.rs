//! Role-based authorization middleware — runs after [`super::auth::require_auth`].
//!
//! Roles are embedded in the JWT at issuance time (`claims.roles`).
//! No database call is made here — authorization is O(1).

use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;

use crate::internal::auth::AuthenticatedUser;
use crate::internal::users::UserRole;

/// Require the authenticated user to hold at least one of `allowed` roles.
///
/// Must run after [`super::auth::require_auth`] (which sets the `AuthenticatedUser` extension).
pub async fn require_role(
    req: Request,
    next: Next,
    allowed: &[UserRole],
) -> Result<Response, StatusCode> {
    let Some(user) = req.extensions().get::<AuthenticatedUser>() else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    let has_role = user
        .claims
        .roles
        .iter()
        .any(|r| UserRole::from_str(r).map(|role| allowed.contains(&role)).unwrap_or(false));

    if has_role {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}

// ── Convenience factories ──────────────────────────────────────────────────────

pub async fn require_admin(req: Request, next: Next) -> Result<Response, StatusCode> {
    require_role(req, next, &[UserRole::Admin, UserRole::System]).await
}

pub async fn require_support_or_admin(req: Request, next: Next) -> Result<Response, StatusCode> {
    require_role(
        req,
        next,
        &[UserRole::Support, UserRole::Admin, UserRole::System, UserRole::FraudAnalyst],
    )
    .await
}

pub async fn require_fraud_analyst(req: Request, next: Next) -> Result<Response, StatusCode> {
    require_role(
        req,
        next,
        &[UserRole::FraudAnalyst, UserRole::Admin, UserRole::System],
    )
    .await
}
