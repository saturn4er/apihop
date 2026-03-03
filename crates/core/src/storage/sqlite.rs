use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;

use crate::models::*;

use super::common::{self, serialize_auth};
use super::migrator::run_migrations_sqlite;
use super::{StorageBackend, StorageError, new_id, now_iso};

pub struct SqliteBackend {
    pool: SqlitePool,
    encryption_key: [u8; 32],
}

impl SqliteBackend {
    pub async fn new(db_path: &str, encryption_key: [u8; 32]) -> Result<Self, StorageError> {
        let options = SqliteConnectOptions::new()
            .filename(db_path)
            .create_if_missing(true)
            .foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        run_migrations_sqlite(&pool).await?;

        Ok(Self { pool, encryption_key })
    }
}

impl_storage_backend!(SqliteBackend, false);
