//! Authentication domain — production-hardened (MFA, audit, JWT rotation).

pub mod audit;
mod error;
pub mod extractor;
pub mod handlers;
mod jwt_keys;
mod mfa;
mod models;
mod password_policy;
mod repository;
mod services;
mod session_store;

pub use error::AuthError;
pub use models::AuthenticatedUser;
pub(crate) use services::{AuthService, JwtConfigWrapper};
