use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum UserError {
    #[error("user not found")]
    NotFound,
    #[error("email already in use")]
    EmailInUse,
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("forbidden")]
    Forbidden,
    #[error("conflict (stale version or concurrent modification)")]
    Conflict,
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("internal error")]
    Internal(#[from] anyhow::Error),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for UserError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            UserError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            UserError::EmailInUse => (StatusCode::CONFLICT, self.to_string()),
            UserError::Conflict => (StatusCode::CONFLICT, self.to_string()),
            UserError::Validation(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            UserError::Forbidden => (StatusCode::FORBIDDEN, self.to_string()),
            UserError::Database(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "database error".to_string(),
            ),
            UserError::Internal(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal error".to_string(),
            ),
        };

        tracing::warn!(status = %status, error = %message, "user error");
        (status, Json(ErrorBody { error: message })).into_response()
    }
}

pub type UserResult<T> = Result<T, UserError>;

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
