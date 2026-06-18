//! WebSocket client for RSC node subscriptions (newHeads, pending txs, logs).

use std::time::Duration;

use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::sync::mpsc;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use crate::internal::blockchain::error::{BlockchainError, BlockchainResult};
use crate::internal::blockchain::models::{BlockchainEvent, BlockchainEventType};

/// Subscribe to `newHeads` and forward translated events to the gateway channel.
pub async fn subscribe_new_heads(
    ws_url: &str,
    network: &str,
    tx: mpsc::Sender<BlockchainEvent>,
) -> BlockchainResult<()> {
    let (mut ws, _) = connect_async(ws_url)
        .await
        .map_err(|e| BlockchainError::WebSocket(e.to_string()))?;

    let subscribe = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_subscribe",
        "params": ["newHeads"]
    });

    ws.send(Message::Text(subscribe.to_string().into()))
        .await
        .map_err(|e| BlockchainError::WebSocket(e.to_string()))?;

    while let Some(msg) = ws.next().await {
        let msg = msg.map_err(|e| BlockchainError::WebSocket(e.to_string()))?;
        if let Message::Text(text) = msg {
            if let Ok(value) = serde_json::from_str::<Value>(&text) {
                if let Some(params) = value.get("params") {
                    let event = BlockchainEvent {
                        network: network.to_string(),
                        event_type: BlockchainEventType::NewHead,
                        payload: params.clone(),
                        received_at: Utc::now(),
                    };
                    if tx.send(event).await.is_err() {
                        tracing::info!("blockchain event channel closed, stopping WS listener");
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

/// Spawn a resilient listener with reconnect backoff.
pub fn spawn_ws_listener(
    ws_url: String,
    network: String,
    tx: mpsc::Sender<BlockchainEvent>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut backoff = Duration::from_secs(1);
        loop {
            tracing::info!(%ws_url, "connecting blockchain websocket");
            match subscribe_new_heads(&ws_url, &network, tx.clone()).await {
                Ok(()) => tracing::warn!("blockchain websocket closed"),
                Err(e) => tracing::error!(error = %e, "blockchain websocket error"),
            }
            tokio::time::sleep(backoff).await;
            backoff = (backoff * 2).min(Duration::from_secs(60));
        }
    })
}
