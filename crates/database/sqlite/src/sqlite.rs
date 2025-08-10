mod migration_manager;

use std::collections::HashMap;

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use bce_exchange_database::{ExchangeRateRecord, StorageAdapter};
use chrono::DateTime;
use sqlx::{
    Row, Sqlite,
    pool::PoolConnection,
    sqlite::{SqliteConnectOptions, SqlitePool},
};

use crate::migration_manager::MigrationManager;

pub struct SqliteStorageAdapter {
    pool: SqlitePool,
}

impl SqliteStorageAdapter {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = SqlitePool::connect_with(
            SqliteConnectOptions::new()
                .filename(database_url)
                .create_if_missing(true),
        )
        .await
        .map_err(|e| anyhow!("Connection error: {}", e))?;

        let mut conn = pool
            .acquire()
            .await
            .map_err(|e| anyhow!("Connection error: {}", e))?;

        Self::apply_pragma_optimisations(&mut conn).await?;
        MigrationManager::ensure_current_schema(&pool).await?;

        Ok(Self { pool })
    }

    async fn apply_pragma_optimisations(conn: &mut PoolConnection<Sqlite>) -> Result<()> {
        sqlx::query("PRAGMA journal_mode = WAL")
            .execute(&mut **conn)
            .await
            .map_err(|e| anyhow!("Operation failed: {}", e))?;

        let pragmas = [
            "PRAGMA synchronous = NORMAL",
            "PRAGMA busy_timeout = 5000",
            "PRAGMA cache_size = -20000",
            "PRAGMA temp_store = MEMORY",
            "PRAGMA mmap_size = 268435456",
            "PRAGMA foreign_keys = ON",
        ];

        for pragma in &pragmas {
            sqlx::query(pragma)
                .execute(&mut **conn)
                .await
                .map_err(|e| anyhow!("Operation failed: {}", e))?;
        }

        Ok(())
    }
}

#[async_trait]
impl StorageAdapter for SqliteStorageAdapter {
    async fn store_exchange_rates(&self, record: ExchangeRateRecord) -> Result<()> {
        let snapshot_json = serde_json::to_string(&record.snapshot)
            .map_err(|e| anyhow!("Serialization error: {}", e))?;

        let metadata_json = if record.metadata.is_empty() {
            None
        } else {
            Some(
                serde_json::to_string(&record.metadata)
                    .map_err(|e| anyhow!("Serialization error: {}", e))?,
            )
        };

        let fetch_timestamp = record.fetch_timestamp.timestamp();

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO exchange_rates
            (source_identifier, fetch_time, snapshot_json, metadata_json)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(&record.source_identifier)
        .bind(fetch_timestamp)
        .bind(snapshot_json)
        .bind(metadata_json)
        .execute(&self.pool)
        .await
        .map_err(|e| anyhow!("Operation failed: {}", e))?;

        Ok(())
    }

    async fn get_latest_exchange_rates(
        &self,
        source_identifier: &str,
    ) -> Result<Option<ExchangeRateRecord>> {
        let row = sqlx::query(
            r#"
            SELECT source_identifier, fetch_time, snapshot_json, metadata_json
            FROM exchange_rates
            WHERE source_identifier = ?
            ORDER BY fetch_time DESC
            LIMIT 1
            "#,
        )
        .bind(source_identifier)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| anyhow!("Operation failed: {}", e))?;

        match row {
            Some(row) => {
                let snapshot_json: String = row.get("snapshot_json");
                let metadata_json: Option<String> = row.get("metadata_json");
                let fetch_timestamp: i64 = row.get("fetch_time");

                let snapshot = serde_json::from_str(&snapshot_json)
                    .map_err(|e| anyhow!("Serialization error: {}", e))?;

                let metadata = if let Some(json) = metadata_json {
                    serde_json::from_str(&json)
                        .map_err(|e| anyhow!("Serialization error: {}", e))?
                } else {
                    HashMap::new()
                };

                let fetch_time = DateTime::from_timestamp(fetch_timestamp, 0)
                    .ok_or_else(|| anyhow!("Invalid timestamp"))?;

                Ok(Some(ExchangeRateRecord {
                    snapshot,
                    fetch_timestamp: fetch_time,
                    source_identifier: source_identifier.to_string(),
                    metadata,
                }))
            }
            None => Ok(None),
        }
    }

    async fn exchange_rates_exist(&self, source_identifier: &str) -> Result<bool> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(1) FROM exchange_rates WHERE source_identifier = ? LIMIT 1",
        )
        .bind(source_identifier)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| anyhow!("Operation failed: {}", e))?;

        Ok(count > 0)
    }

    async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| anyhow!("Connection error: {}", e))?;
        Ok(())
    }
}
