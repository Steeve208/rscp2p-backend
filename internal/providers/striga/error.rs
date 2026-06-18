use thiserror::Error;

#[derive(Debug, Error)]
pub enum StrigaError {
    #[error("striga not configured")]
    NotConfigured,
    #[error("validation error: {0}")]
    Validation(String),
    #[error("upstream error: {0}")]
    Upstream(String),
    #[error("webhook forbidden")]
    WebhookForbidden,
    #[error("parse error: {0}")]
    Parse(String),
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("internal error")]
    Internal(#[from] anyhow::Error),
}

pub type StrigaResult<T> = Result<T, StrigaError>;

impl From<StrigaError> for crate::internal::providers::error::ProviderError {
    fn from(err: StrigaError) -> Self {
        match err {
            StrigaError::NotConfigured => Self::ProviderNotConfigured,
            StrigaError::Validation(m) => Self::Validation(m),
            StrigaError::Upstream(m) => Self::Upstream(m),
            StrigaError::WebhookForbidden => Self::Forbidden,
            StrigaError::Parse(m) => Self::Upstream(m),
            StrigaError::Database(e) => Self::Database(e),
            StrigaError::Internal(e) => Self::Internal(e),
        }
    }
}
