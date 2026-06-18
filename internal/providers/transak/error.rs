use thiserror::Error;

use crate::internal::providers::error::ProviderError;

#[derive(Debug, Error)]
pub enum TransakError {
    #[error("validation error: {0}")]
    Validation(String),
    #[error("not configured")]
    NotConfigured,
    #[error("webhook forbidden")]
    WebhookForbidden,
    #[error("upstream error: {0}")]
    Upstream(String),
    #[error("parse error: {0}")]
    Parse(String),
}

pub type TransakResult<T> = Result<T, TransakError>;

impl From<TransakError> for ProviderError {
    fn from(err: TransakError) -> Self {
        match err {
            TransakError::Validation(m) => ProviderError::Validation(m),
            TransakError::NotConfigured => ProviderError::ProviderNotConfigured,
            TransakError::WebhookForbidden => ProviderError::Forbidden,
            TransakError::Upstream(m) => ProviderError::Upstream(m),
            TransakError::Parse(m) => ProviderError::Validation(m),
        }
    }
}
