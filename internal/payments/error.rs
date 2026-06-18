use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PaymentError {
    #[error("merchant not found")]
    MerchantNotFound,
    #[error("invoice not found")]
    InvoiceNotFound,
    #[error("payment not found")]
    PaymentNotFound,
    #[error("settlement not found")]
    SettlementNotFound,
    #[error("invoice expired")]
    InvoiceExpired,
    #[error("invoice already paid")]
    InvoiceAlreadyPaid,
    #[error("invalid amount")]
    InvalidAmount,
    #[error("insufficient balance")]
    InsufficientBalance,
    #[error("validation error: {0}")]
    Validation(String),
    #[error("idempotency conflict")]
    IdempotencyConflict,
    #[error("forbidden")]
    Forbidden,
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("internal error")]
    Internal(#[from] anyhow::Error),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for PaymentError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            PaymentError::MerchantNotFound
            | PaymentError::InvoiceNotFound
            | PaymentError::PaymentNotFound
            | PaymentError::SettlementNotFound => (StatusCode::NOT_FOUND, self.to_string()),
            PaymentError::InvoiceExpired | PaymentError::InvoiceAlreadyPaid => {
                (StatusCode::BAD_REQUEST, self.to_string())
            }
            PaymentError::InvalidAmount | PaymentError::InsufficientBalance => {
                (StatusCode::BAD_REQUEST, self.to_string())
            }
            PaymentError::Validation(m) => (StatusCode::BAD_REQUEST, m.clone()),
            PaymentError::IdempotencyConflict => (StatusCode::CONFLICT, self.to_string()),
            PaymentError::Conflict(m) => (StatusCode::CONFLICT, m.clone()),
            PaymentError::Forbidden => (StatusCode::FORBIDDEN, self.to_string()),
            PaymentError::Database(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "database error".into())
            }
            PaymentError::Internal(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error".into())
            }
        };

        tracing::warn!(status = %status, error = %msg, "payment error");
        (status, Json(ErrorBody { error: msg })).into_response()
    }
}

pub type PaymentResult<T> = Result<T, PaymentError>;
