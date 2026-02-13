use crate::core::config::AppConfig;
use crate::core::error::StorageError;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn open(config: &AppConfig) -> Result<Self, StorageError> {
        let db_dir = config.data_path();
        std::fs::create_dir_all(&db_dir).map_err(|e| StorageError::Database(e.to_string()))?;

        let db_path = db_dir.join("octo-code.db");
        let options = SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .pragma("foreign_keys", "ON");

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await
            .map_err(|e| StorageError::Database(e.to_string()))?;

        Ok(Self { pool })
    }

    pub async fn run_migrations(&self) -> Result<(), StorageError> {
        // Run the initial migration manually
        sqlx::query(include_str!("../../migrations/001_initial.sql"))
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Migration(e.to_string()))?;
        Ok(())
    }

    pub fn sessions(&self) -> super::SessionRepo {
        super::SessionRepo::new(self.pool.clone())
    }

    pub fn messages(&self) -> super::MessageRepo {
        super::MessageRepo::new(self.pool.clone())
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}
