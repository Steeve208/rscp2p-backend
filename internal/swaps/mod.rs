//! Asset swap engine — provider-agnostic pricing, routing, and execution.
//!
//! # Responsibility
//! - Cross-asset conversion (e.g. BTC↔USDT, RSC↔BTC, ETH↔BRL)
//! - Multi-provider quotes (DEX / CEX adapters from config)
//! - Liquidity routing (best net output)
//! - Fee breakdown (platform + provider + network)
//!
//! # Design
//! - **No hardcoded providers** in orchestration: only `SwapProviderRegistry` + config `SWAP_PROVIDERS`
//! - New venues are added via config + `ConfiguredLiquidityAdapter` (or future dedicated adapters in factory)

pub mod config;
pub mod error;
pub mod fees;
pub mod handlers;
pub mod models;
pub mod pricing;
pub mod providers;
pub mod registry;
pub mod repository;
pub mod routing;
pub mod service;
pub mod traits;

#[cfg(test)]
mod tests;

pub use config::SwapsConfig;
pub use error::{SwapError, SwapResult};
pub use models::{
    CreateSwapOrderRequest, CreateSwapOrderResponse, SwapOrder, SwapPair, SwapProviderInfo,
    SwapQuoteRequest, SwapQuoteResponse,
};
pub use service::SwapServiceHandle;
