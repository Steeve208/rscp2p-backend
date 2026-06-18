//! PostgreSQL connection pool manager.

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use tracing::info;

use crate::internal::database::error::{DatabaseError, DatabaseResult};

/// Owns the shared `PgPool` for the application.
#[derive(Debug, Clone)]
pub struct ConnectionManager {
    pool: PgPool,
}

impl ConnectionManager {
    pub async fn connect(database_url: &str, max_connections: u32) -> DatabaseResult<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .connect(database_url)
            .await?;

        sqlx::query("SELECT 1").execute(&pool).await?;

        info!(max_connections, "PostgreSQL connected successfully");

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub fn into_pool(self) -> PgPool {
        self.pool
    }
}

impl AsRef<PgPool> for ConnectionManager {
    fn as_ref(&self) -> &PgPool {
        &self.pool
    }
}

impl std::ops::Deref for ConnectionManager {
    type Target = PgPool;

    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}
