use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use redis::RedisError;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("email already registered")]
    EmailAlreadyExists,
    #[error("invalid or expired token")]
    InvalidToken,
    #[error("session not found")]
    SessionNotFound,
    #[error("unauthorized")]
    Unauthorized,
    #[error("too many login attempts, try again later")]
    TooManyRequests,
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("redis error")]
    Redis(#[from] RedisError),
    #[error("internal error")]
    Internal(#[from] anyhow::Error),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            AuthError::InvalidCredentials => (StatusCode::UNAUTHORIZED, self.to_string()),
            AuthError::EmailAlreadyExists => (StatusCode::CONFLICT, self.to_string()),
            AuthError::InvalidToken => (StatusCode::UNAUTHORIZED, self.to_string()),
            AuthError::SessionNotFound => (StatusCode::NOT_FOUND, self.to_string()),
            AuthError::Unauthorized => (StatusCode::UNAUTHORIZED, self.to_string()),
            AuthError::TooManyRequests => (StatusCode::TOO_MANY_REQUESTS, self.to_string()),
            AuthError::Validation(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            AuthError::Database(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "database error".to_string(),
            ),
            AuthError::Redis(_) => (StatusCode::INTERNAL_SERVER_ERROR, "cache error".to_string()),
            AuthError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal error".to_string(),
            ),
        };

        tracing::warn!(status = %status, error = %message, "auth error");

        (status, Json(ErrorBody { error: message })).into_response()
    }
}

pub type AuthResult<T> = Result<T, AuthError>;

pub fn format_validation(err: &validator::ValidationErrors) -> String {
    err.field_errors()
        .iter()
        .flat_map(|(field, errors)| {
            errors.iter().map(move |e| {
                format!(
                    "{}: {}",
                    field,
                    e.message
                        .as_ref()
                        .map(|m| m.to_string())
                        .unwrap_or_default()
                )
            })
        })
        .collect::<Vec<_>>()
        .join(", ")
}
