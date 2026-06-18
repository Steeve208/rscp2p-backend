use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WalletError {
    #[error("wallet not found")]
    WalletNotFound,
    #[error("insufficient balance")]
    InsufficientBalance,
    #[error("invalid amount")]
    InvalidAmount,
    #[error("address already exists")]
    AddressExists,
    #[error("validation error: {0}")]
    Validation(String),
    #[error("idempotency conflict")]
    IdempotencyConflict,
    #[error("forbidden")]
    Forbidden,
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("internal error")]
    Internal(#[from] anyhow::Error),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for WalletError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            WalletError::WalletNotFound => (StatusCode::NOT_FOUND, self.to_string()),
            WalletError::InsufficientBalance => (StatusCode::BAD_REQUEST, self.to_string()),
            WalletError::InvalidAmount => (StatusCode::BAD_REQUEST, self.to_string()),
            WalletError::AddressExists => (StatusCode::CONFLICT, self.to_string()),
            WalletError::Validation(m) => (StatusCode::BAD_REQUEST, m.clone()),
            WalletError::IdempotencyConflict => (StatusCode::CONFLICT, self.to_string()),
            WalletError::Forbidden => (StatusCode::FORBIDDEN, self.to_string()),
            WalletError::Database(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "database error".into())
            }
            WalletError::Internal(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error".into())
            }
        };

        tracing::warn!(status = %status, error = %msg, "wallet error");
        (status, Json(ErrorBody { error: msg })).into_response()
    }
}

pub type WalletResult<T> = Result<T, WalletError>;
