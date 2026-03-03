use sqlx::SqlitePool;

use super::StorageError;

pub async fn run_migrations_sqlite(pool: &SqlitePool) -> Result<(), StorageError> {
    sqlx::migrate!("./migrations/sqlite")
        .run(pool)
        .await
        .map_err(|e| StorageError::Database(Box::new(e)))?;
    Ok(())
}

#[cfg(feature = "postgres")]
pub async fn run_migrations_postgres(pool: &sqlx::PgPool) -> Result<(), StorageError> {
    sqlx::migrate!("./migrations/postgres")
        .run(pool)
        .await
        .map_err(|e| StorageError::Database(Box::new(e)))?;
    Ok(())
}
