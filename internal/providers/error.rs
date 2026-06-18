use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use thiserror::Error;

use crate::internal::core::financial_gateway::GatewayError;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("order not found")]
    OrderNotFound,
    #[error("provider not configured")]
    ProviderNotConfigured,
    #[error("provider unavailable: {0}")]
    ProviderUnavailable(String),
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

impl IntoResponse for ProviderError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            ProviderError::OrderNotFound => (StatusCode::NOT_FOUND, self.to_string()),
            ProviderError::ProviderNotConfigured | ProviderError::ProviderUnavailable(_) => {
                (StatusCode::SERVICE_UNAVAILABLE, self.to_string())
            }
            ProviderError::Validation(m) => (StatusCode::BAD_REQUEST, m.clone()),
            ProviderError::Conflict(m) => (StatusCode::CONFLICT, m.clone()),
            ProviderError::Forbidden => (StatusCode::FORBIDDEN, self.to_string()),
            ProviderError::Upstream(m) => (StatusCode::BAD_GATEWAY, m.clone()),
            ProviderError::Database(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "database error".into())
            }
            ProviderError::Internal(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error".into())
            }
        };

        tracing::warn!(status = %status, error = %msg, "provider error");
        (status, Json(ErrorBody { error: msg })).into_response()
    }
}

impl From<GatewayError> for ProviderError {
    fn from(err: GatewayError) -> Self {
        match err {
            GatewayError::NotFound => Self::OrderNotFound,
            GatewayError::Forbidden => Self::Forbidden,
            GatewayError::Validation(m) => Self::Validation(m),
            GatewayError::NotConfigured => Self::ProviderNotConfigured,
            GatewayError::Upstream(m) => Self::Upstream(m),
            GatewayError::Database(e) => Self::Database(e),
            GatewayError::Internal(e) => Self::Internal(e),
        }
    }
}

pub type ProviderResult<T> = Result<T, ProviderError>;
