//! Blockchain Connector — translates RSC node / indexer ↔ Gateway
//!
//! # Responsibility
//! This module talks to external chain infrastructure. It does **not** own accounting.
//!
//! ## What lives here
//! - JSON-RPC queries: balance, blocks, transaction status, broadcast
//! - WebSocket subscriptions: new heads, pending txs, logs (via `ws/`)
//! - Translation of raw node payloads into gateway models
//!
//! ## What does NOT live here
//! - User balances, ledger, deposits/withdrawals → `internal/wallets/`
//! - Signing keys / HD wallets → future `internal/wallets/providers/` or HSM integration
//!
//! ## Architecture
//! ```text
//! internal/blockchain/
//! ├── rpc/          # generic JSON-RPC 2.0 HTTP client
//! ├── rsc/          # RSC-specific methods (eth_* compatible)
//! ├── ws/           # WebSocket subscriptions + reconnect
//! ├── services/     # gateway-facing API
//! └── handlers/     # HTTP routes for ops / internal tools
//! ```
//!
//! Deposit workers should consume `BlockchainEvent` from the WS channel and call
//! `WalletService::record_confirmed_deposit` when confirmations are sufficient.

pub mod error;
pub mod handlers;
pub mod models;
pub mod rpc;
pub mod rsc;
pub mod services;
pub mod ws;

pub use error::{BlockchainError, BlockchainResult};
pub use models::{
    BlockTransfer, BlockchainEvent, BlockchainEventType, BroadcastTxRequest, BroadcastTxResponse,
    NodeHealth, OnChainBalance, OnChainBlock, OnChainTransaction, OnChainTxState,
};
pub use services::{BlockchainService, BlockchainServiceHandle};
