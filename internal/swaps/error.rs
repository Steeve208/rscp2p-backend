use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SwapError {
    #[error("swap order not found")]
    OrderNotFound,
    #[error("unsupported trading pair")]
    UnsupportedPair,
    #[error("no liquidity route available")]
    NoRoute,
    #[error("provider not available")]
    ProviderUnavailable,
    #[error("invalid amount")]
    InvalidAmount,
    #[error("validation error: {0}")]
    Validation(String),
    #[error("forbidden")]
    Forbidden,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("upstream error: {0}")]
    Upstream(String),
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("internal error")]
    Internal(#[from] anyhow::Error),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for SwapError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            SwapError::OrderNotFound => (StatusCode::NOT_FOUND, self.to_string()),
            SwapError::UnsupportedPair | SwapError::InvalidAmount => {
                (StatusCode::BAD_REQUEST, self.to_string())
            }
            SwapError::NoRoute | SwapError::ProviderUnavailable => {
                (StatusCode::SERVICE_UNAVAILABLE, self.to_string())
            }
            SwapError::Validation(m) => (StatusCode::BAD_REQUEST, m.clone()),
            SwapError::Conflict(m) => (StatusCode::CONFLICT, m.clone()),
            SwapError::Forbidden => (StatusCode::FORBIDDEN, self.to_string()),
            SwapError::Upstream(m) => (StatusCode::BAD_GATEWAY, m.clone()),
            SwapError::Database(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "database error".into())
            }
            SwapError::Internal(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error".into())
            }
        };

        tracing::warn!(status = %status, error = %msg, "swap error");
        (status, Json(ErrorBody { error: msg })).into_response()
    }
}

pub type SwapResult<T> = Result<T, SwapError>;
