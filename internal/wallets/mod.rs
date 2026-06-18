//! Wallets Module — Crypto Wallet Integration & Financial Core
//!
//! # Philosophy (Important)
//! This module owns **wallet integration and double-entry accounting**.
//! It does **NOT** contain full blockchain node logic or raw RPC clients.
//!
//! ## What lives here
//! - User wallets and multi-asset balances
//! - Deposit addresses (multi-chain)
//! - Transaction history
//! - The canonical double-entry ledger (`ledger_entries`)
//! - Business rules for deposits, withdrawals, and internal transfers
//!
//! ## What does NOT live here (by design)
//! - Raw blockchain scanning / node operation → goes in `internal/blockchain/` or dedicated workers
//! - Provider-specific signing / broadcasting logic → will live in `providers/btc`, `providers/eth`, etc.
//!
//! ## Planned Extension Structure (future)
//! ```text
//! internal/wallets/
//! ├── providers/
//! │   ├── mod.rs
//! │   ├── btc/
//! │   ├── eth/
//! │   └── rsc/
//! ├── balances/          // advanced balance reconciliation, hot/cold logic
//! ├── transactions/      // complex withdrawal flows, batching, compliance
//! └── addresses/         // address pools, HD derivation, gap limit management
//! ```
//!
//! ## Money Safety Rules (non-negotiable)
//! 1. All value movement must produce a balanced journal in `ledger_entries`
//! 2. Every external action (deposit, withdrawal) must be idempotent
//! 3. Never use `f64`/`f32` for amounts — only `rust_decimal::Decimal`
//! 4. Balances can be materialized for performance, but the ledger is the source of truth
//!
//! This foundation was created with production requirements in mind from day one.

pub mod error;
pub mod handlers;
pub mod models;
pub mod repository;
pub mod services;
// pub mod providers;

pub use error::WalletError;
pub use models::{
    Asset, BroadcastWithdrawalRequest, BroadcastWithdrawalResponse, Chain, DepositAddressResponse,
    EnsureDefaultWalletResponse, LedgerEntry, RecordDepositRequest, RecordDepositResponse,
    RequestWithdrawalRequest, RequestWithdrawalResponse, Wallet, WalletAddress, WalletBalance,
    WalletTransaction,
};
pub use services::WalletServiceHandle;
