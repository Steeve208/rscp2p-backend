//! PostgreSQL infrastructure — connection, migrations, pools, transactions.
//!
//! Domain SQL (SELECT/INSERT per feature) belongs in each module's `repository/`,
//! not here.
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`connection`] | Pool creation and [`ConnectionManager`] |
//! | [`migrations`] | Apply `migrations/*.sql` |
//! | [`helpers`] | Ping, constraint / unique-violation helpers |
//! | [`transaction`] | Begin / commit / `run` transaction wrapper |
//! | [`error`] | Infrastructure-level errors |

mod connection;
mod error;
mod helpers;
mod migrations;
mod transaction;

pub use connection::ConnectionManager;
pub use error::{DatabaseError, DatabaseResult};
pub use helpers::{constraint_name, is_unique_on_constraint, is_unique_violation, ping};
pub use migrations::run as migrate;
pub use transaction::{commit, TransactionManager};

use sqlx::PgPool;

/// Connect to PostgreSQL and return the shared pool (used by [`crate::internal::state::AppState`]).
pub async fn connect(database_url: &str, max_connections: u32) -> DatabaseResult<PgPool> {
    Ok(ConnectionManager::connect(database_url, max_connections)
        .await?
        .into_pool())
}
