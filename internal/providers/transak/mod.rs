//! Transak — external integration (isolated from gateway core).
//!
//! - **api** — REST partner API (price-quote, orders)
//! - **onramp** — quotes & checkout
//! - **webhooks** — order notifications
//! - **kyc** — user verification
//! - **cards** — payment method discovery
//!
//! The core fiat service only depends on [`TransakProvider`] + [`webhooks`] types.

pub mod api;
pub mod cards;
pub mod config;
pub mod error;
pub mod handlers;
pub mod kyc;
pub mod models;
pub mod onramp;
pub mod provider;
pub mod webhooks;
pub mod widget;

pub use config::{TransakEnvironment, TransakProviderConfig};
pub use provider::TransakProvider;
pub use webhooks::OrderNotification;
