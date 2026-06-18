//! Transaction lifecycle helpers for repositories.

use sqlx::{PgPool, Postgres, Transaction};

use crate::internal::database::error::{DatabaseError, DatabaseResult};

/// Entry point for beginning transactions from a pool handle.
#[derive(Debug, Clone, Copy)]
pub struct TransactionManager;

impl TransactionManager {
    pub async fn begin(pool: &PgPool) -> DatabaseResult<Transaction<'_, Postgres>> {
        pool.begin().await.map_err(DatabaseError::from)
    }
}

/// Commits an open transaction.
pub async fn commit(tx: Transaction<'_, Postgres>) -> DatabaseResult<()> {
    tx.commit().await.map_err(DatabaseError::from)
}

/// Rolls back an open transaction (no-op if already finished).
pub async fn rollback(tx: Transaction<'_, Postgres>) -> DatabaseResult<()> {
    tx.rollback().await.map_err(DatabaseError::from)
}
