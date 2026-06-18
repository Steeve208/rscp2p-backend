use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GatewayError {
    #[error("not found")]
    NotFound,
    #[error("forbidden")]
    Forbidden,
    #[error("validation error: {0}")]
    Validation(String),
    #[error("provider not configured")]
    NotConfigured,
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

impl IntoResponse for GatewayError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            GatewayError::NotFound => (StatusCode::NOT_FOUND, self.to_string()),
            GatewayError::Forbidden => (StatusCode::FORBIDDEN, self.to_string()),
            GatewayError::Validation(m) => (StatusCode::BAD_REQUEST, m.clone()),
            GatewayError::NotConfigured => (StatusCode::SERVICE_UNAVAILABLE, self.to_string()),
            GatewayError::Upstream(m) => (StatusCode::BAD_GATEWAY, m.clone()),
            GatewayError::Database(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "database error".into())
            }
            GatewayError::Internal(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error".into())
            }
        };
        (status, Json(ErrorBody { error: msg })).into_response()
    }
}

pub type GatewayResult<T> = Result<T, GatewayError>;

impl From<crate::internal::providers::striga::error::StrigaError> for GatewayError {
    fn from(err: crate::internal::providers::striga::error::StrigaError) -> Self {
        match err {
            crate::internal::providers::striga::error::StrigaError::NotConfigured => {
                Self::NotConfigured
            }
            crate::internal::providers::striga::error::StrigaError::Validation(m) => {
                Self::Validation(m)
            }
            crate::internal::providers::striga::error::StrigaError::Upstream(m) => Self::Upstream(m),
            crate::internal::providers::striga::error::StrigaError::WebhookForbidden => {
                Self::Forbidden
            }
            crate::internal::providers::striga::error::StrigaError::Parse(m) => Self::Upstream(m),
            crate::internal::providers::striga::error::StrigaError::Database(e) => Self::Database(e),
            crate::internal::providers::striga::error::StrigaError::Internal(e) => Self::Internal(e),
        }
    }
}
