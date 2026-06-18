//! Background workers — async tasks with Redis retry queues and dead-letter support.
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`deposit`] | Credit confirmed on-chain deposits |
//! | [`withdrawal`] | Finalize outgoing withdrawal transactions |
//! | [`queue`] | Redis retry queue + DLQ + PostgreSQL audit |
//! | [`processor`] | Polls due jobs and dispatches handlers |
//! | [`backoff`] | Exponential backoff with jitter |

pub mod backoff;
pub mod card_sync;
pub mod crypto_sync;
pub mod deposit;
pub mod error;
pub mod job;
pub mod processor;
pub mod queue;
pub mod withdrawal;

pub use card_sync::spawn_card_sync_worker;
pub use crypto_sync::spawn_crypto_sync_worker;
pub use deposit::{spawn_deposit_worker, DepositWorkerDeps};
pub use processor::spawn_queue_processor;
pub use queue::RetryQueue;
pub use withdrawal::{spawn_withdrawal_worker, WithdrawalWorkerDeps};
