//! Generic JSON-RPC 2.0 over HTTP — used by RSC node and indexer endpoints.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::internal::blockchain::error::{BlockchainError, BlockchainResult};

#[derive(Clone)]
pub struct JsonRpcClient {
    http: Client,
    endpoint: String,
    next_id: Arc<AtomicU64>,
}

impl JsonRpcClient {
    pub fn new(http: Client, endpoint: impl Into<String>) -> Self {
        Self {
            http,
            endpoint: endpoint.into(),
            next_id: Arc::new(AtomicU64::new(1)),
        }
    }

    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    pub async fn call<T: DeserializeOwned>(
        &self,
        method: &str,
        params: Value,
    ) -> BlockchainResult<T> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let body = JsonRpcRequest {
            jsonrpc: "2.0",
            id,
            method,
            params,
        };

        let response = self
            .http
            .post(&self.endpoint)
            .json(&body)
            .send()
            .await
            .map_err(|_| BlockchainError::NodeUnavailable)?;

        if !response.status().is_success() {
            return Err(BlockchainError::Rpc(format!("HTTP {}", response.status())));
        }

        let envelope: JsonRpcResponse<T> = response
            .json()
            .await
            .map_err(|e| BlockchainError::Rpc(e.to_string()))?;

        if let Some(err) = envelope.error {
            return Err(BlockchainError::Rpc(format!(
                "code {}: {}",
                err.code, err.message
            )));
        }

        envelope
            .result
            .ok_or_else(|| BlockchainError::Rpc("empty RPC result".into()))
    }

    pub async fn call_optional<T: DeserializeOwned>(
        &self,
        method: &str,
        params: Value,
    ) -> BlockchainResult<Option<T>> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let body = JsonRpcRequest {
            jsonrpc: "2.0",
            id,
            method,
            params,
        };

        let response = self
            .http
            .post(&self.endpoint)
            .json(&body)
            .send()
            .await
            .map_err(|_| BlockchainError::NodeUnavailable)?;

        let envelope: JsonRpcResponse<T> = response
            .json()
            .await
            .map_err(|e| BlockchainError::Rpc(e.to_string()))?;

        if let Some(err) = envelope.error {
            return Err(BlockchainError::Rpc(format!(
                "code {}: {}",
                err.code, err.message
            )));
        }

        Ok(envelope.result)
    }
}

#[derive(Serialize)]
struct JsonRpcRequest<'a> {
    jsonrpc: &'static str,
    id: u64,
    method: &'a str,
    params: Value,
}

#[derive(Deserialize)]
struct JsonRpcResponse<T> {
    result: Option<T>,
    error: Option<JsonRpcErrorBody>,
}

#[derive(Deserialize)]
struct JsonRpcErrorBody {
    code: i64,
    message: String,
}

/// Parse `0x`-prefixed hex quantity to u64.
pub fn hex_to_u64(hex: &str) -> BlockchainResult<u64> {
    let s = hex.strip_prefix("0x").unwrap_or(hex);
    if s.is_empty() {
        return Ok(0);
    }
    u64::from_str_radix(s, 16).map_err(|_| BlockchainError::Rpc(format!("invalid hex: {hex}")))
}

/// Parse `0x`-prefixed hex quantity to decimal string (wei), arbitrary precision.
pub fn hex_wei_to_string(hex: &str) -> BlockchainResult<String> {
    let s = hex.strip_prefix("0x").unwrap_or(hex);
    if s.is_empty() || s == "0" {
        return Ok("0".into());
    }

    let mut digits: Vec<u8> = Vec::new();
    for ch in s.chars() {
        let nibble = ch
            .to_digit(16)
            .ok_or_else(|| BlockchainError::Rpc(format!("invalid wei hex: {hex}")))?;

        let mut carry = nibble as u32;
        for d in digits.iter_mut() {
            let value = (*d as u32) * 16 + carry;
            *d = (value % 10) as u8;
            carry = value / 10;
        }
        while carry > 0 {
            digits.push((carry % 10) as u8);
            carry /= 10;
        }
    }

    if digits.is_empty() {
        return Ok("0".into());
    }

    Ok(digits.iter().rev().map(|d| char::from(b'0' + *d)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_wei_hex_to_decimal() {
        assert_eq!(hex_wei_to_string("0x0").unwrap(), "0");
        assert_eq!(hex_wei_to_string("0x1").unwrap(), "1");
        assert_eq!(hex_wei_to_string("0x10").unwrap(), "16");
    }
}
