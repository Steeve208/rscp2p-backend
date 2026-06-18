//! Payments Module — RSC Pay foundation
//!
//! # Responsibility
//! Process QR payments, merchants, invoices, instant payments.
//! Future: fiat conversions, full RSC Pay product surface.
//!
//! ## Components
//! ```text
//! internal/payments/
//! ├── services/     # merchant system + payment engine + settlement logic
//! ├── repository/   # persistence
//! ├── handlers/     # HTTP API
//! └── models/       # gateway-facing types
//! ```
//!
//! ## What lives here
//! - **Merchant system**: register comercios, link wallet (optional)
//! - **Payment engine**: invoices, QR payloads, instant pay completion
//! - **Settlement logic**: batch unsettled payments into merchant settlements
//!
//! ## What does NOT live here
//! - On-chain RPC / node → `internal/blockchain/`
//! - User balances / ledger → `internal/wallets/`
//! - Fiat on-ramp APIs → `internal/providers/transak` (banking → Striga via Financial Gateway)
//!
//! ## Money flow
//! - **Pay**: payer → settlement clearing wallet.
//! - **Settlement**: clearing → merchant wallet (liquidación explícita).
//! - **Fiat pay**: on-ramp → deposit al payer → pay → clearing (mismo flujo).

pub mod error;
pub mod handlers;
pub mod models;
pub mod repository;
pub mod services;

pub use error::{PaymentError, PaymentResult};
pub use models::{
    CreateInvoiceRequest, CreateInvoiceResponse, InvoicePublicView, Merchant, MerchantResponse,
    PayInvoiceRequest, PayInvoiceResponse, Payment, PaymentInvoice, PaymentMethod,
    QrPaymentPayload, RegisterMerchantRequest, Settlement, SettlementResponse,
};
pub use services::PaymentServiceHandle;
