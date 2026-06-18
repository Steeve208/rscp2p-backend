//! Striga Network — banking, cards, and KYC integration (isolated from gateway core).
//!
//! RSC Bank users never interact with Striga directly; all calls go through
//! [`crate::internal::core::financial_gateway`].

pub mod api;
pub mod auth;
pub mod config;
pub mod error;
pub mod handlers;
pub mod models;
pub mod repository;
pub mod webhooks;

pub use api::client::StrigaApiClient;
pub use config::StrigaProviderConfig;
