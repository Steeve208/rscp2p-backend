//! SQL migration runner (`migrations/` at repo root).

use sqlx::PgPool;
use tracing::info;

use crate::internal::database::error::DatabaseResult;

pub async fn run(pool: &PgPool) -> DatabaseResult<()> {
    sqlx::migrate!("./migrations").run(pool).await?;
    info!("database migrations applied");
    Ok(())
}
