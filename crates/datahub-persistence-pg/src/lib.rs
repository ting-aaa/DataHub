use std::time::Duration;

use sqlx::{PgPool, postgres::PgPoolOptions};

/// Opens the `PostgreSQL` pool used by a `DataHub` process.
///
/// # Errors
///
/// Returns the underlying `SQLx` connection error when `PostgreSQL` cannot be
/// reached or the connection string is invalid.
pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .connect(database_url)
        .await
}

/// Verifies that the pool can execute a minimal query.
///
/// # Errors
///
/// Returns the underlying `SQLx` error when no healthy connection is available.
pub async fn check(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT 1").execute(pool).await?;
    Ok(())
}

/// Applies every embedded `DataHub` migration in order.
///
/// # Errors
///
/// Returns a migration error when the migration history is inconsistent or a
/// statement cannot be applied.
pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("../../migrations").run(pool).await
}
