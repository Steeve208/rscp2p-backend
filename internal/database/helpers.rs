//! Cross-cutting DB utilities (no domain queries).

use sqlx::PgPool;

/// Liveness probe for health checks.
pub async fn ping(pool: &PgPool) -> bool {
    sqlx::query("SELECT 1").execute(pool).await.is_ok()
}

/// Returns true when `err` is a PostgreSQL unique violation (SQLSTATE 23505).
pub fn is_unique_violation(err: &sqlx::Error) -> bool {
    err.as_database_error()
        .and_then(|db| db.code())
        .map(|code| code == "23505")
        .unwrap_or(false)
}

/// Returns true when `err` is a unique violation on the given constraint name.
pub fn is_unique_on_constraint(err: &sqlx::Error, constraint: &str) -> bool {
    if !is_unique_violation(err) {
        return false;
    }
    err.as_database_error()
        .and_then(|db| db.constraint())
        .map(|name| name == constraint)
        .unwrap_or(false)
}

/// Returns the PostgreSQL constraint name when present.
pub fn constraint_name(err: &sqlx::Error) -> Option<&str> {
    err.as_database_error().and_then(|db| db.constraint())
}
