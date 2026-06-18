//! External third-party integrations — **separated from gateway core**.
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`striga`] | Striga banking — cards, KYC, IBAN, SEPA |
//! | [`transak`] | Transak — crypto buy/sell, on-ramp, off-ramp |
//! | [`service`] | Fiat invoice pay orchestration (Transak only) |
//!
//! RSC App must use [`crate::internal::core::financial_gateway`] — never call providers directly.

pub mod error;
pub mod handlers;
pub mod models;
pub mod repository;
pub mod service;
pub mod striga;
pub mod traits;
pub mod transak;

#[cfg(test)]
mod tests;

pub use error::{ProviderError, ProviderResult};
pub use models::{
    FiatConversionOrder, FiatOrderStatus, FiatProvider, FiatProviderInfo, FiatQuoteRequest,
    FiatQuoteResponse, StartFiatInvoicePayRequest, StartFiatInvoicePayResponse,
};
pub use service::FiatConversionServiceHandle;
