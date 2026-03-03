use sqlx::PgPool;

use crate::models::*;

use super::common::{self, serialize_auth};
use super::migrator::run_migrations_postgres;
use super::{StorageBackend, StorageError, new_id, now_iso};

pub struct PostgresBackend {
    pool: PgPool,
    encryption_key: [u8; 32],
}

impl PostgresBackend {
    pub async fn new(database_url: &str, encryption_key: [u8; 32]) -> Result<Self, StorageError> {
        let pool = PgPool::connect(database_url).await?;

        run_migrations_postgres(&pool).await?;

        Ok(Self { pool, encryption_key })
    }
}

impl_storage_backend!(PostgresBackend, true);
