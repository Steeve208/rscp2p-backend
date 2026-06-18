use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BlockchainError {
    #[error("blockchain node unavailable")]
    NodeUnavailable,
    #[error("transaction not found")]
    TransactionNotFound,
    #[error("block not found")]
    BlockNotFound,
    #[error("invalid address")]
    InvalidAddress,
    #[error("invalid transaction hash")]
    InvalidTxHash,
    #[error("broadcast rejected: {0}")]
    BroadcastRejected(String),
    #[error("websocket error: {0}")]
    WebSocket(String),
    #[error("rpc error: {0}")]
    Rpc(String),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("internal error")]
    Internal(#[from] anyhow::Error),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for BlockchainError {
    fn into_response(self) -> Response {
        let (status, msg) = match &self {
            BlockchainError::NodeUnavailable => (StatusCode::SERVICE_UNAVAILABLE, self.to_string()),
            BlockchainError::TransactionNotFound | BlockchainError::BlockNotFound => {
                (StatusCode::NOT_FOUND, self.to_string())
            }
            BlockchainError::InvalidAddress
            | BlockchainError::InvalidTxHash
            | BlockchainError::Validation(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            BlockchainError::BroadcastRejected(m) => (StatusCode::BAD_REQUEST, m.clone()),
            BlockchainError::WebSocket(m) | BlockchainError::Rpc(m) => {
                (StatusCode::BAD_GATEWAY, m.clone())
            }
            BlockchainError::Internal(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error".into())
            }
        };

        tracing::warn!(status = %status, error = %msg, "blockchain error");
        (status, Json(ErrorBody { error: msg })).into_response()
    }
}

pub type BlockchainResult<T> = Result<T, BlockchainError>;
