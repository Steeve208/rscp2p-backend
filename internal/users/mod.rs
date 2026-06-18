//! User management and profiles (distinct from auth).
//!
//! # Separation of concerns
//! - `auth/`: identity verification, passwords, MFA secrets, tokens, sessions
//! - `users/`: profile data, preferences, account status, public profile
//!
//! The users module **never** handles password_hash or raw MFA secrets.

mod audit;
mod error;
pub mod handlers;
mod models;
mod repository;
mod services;

pub use audit::{UserAuditContext, UserAuditEventType, UserAuditRepository};
pub use error::{UserError, UserResult};
pub use models::{UpdateProfileRequest, User, UserProfileResponse, UserRole, UserStatus};
pub use services::{UserService, UserServiceHandle};
