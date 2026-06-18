//! Database infrastructure errors (connection, migrations, transactions).

use thiserror::Error;

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("failed to connect to PostgreSQL: {0}")]
    Connection(#[from] sqlx::Error),
    #[error("migration failed: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("transaction failed: {0}")]
    Transaction(String),
}

pub type DatabaseResult<T> = Result<T, DatabaseError>;
