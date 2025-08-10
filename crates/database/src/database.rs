use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use async_trait::async_trait;
use bce_exchange_client::ExchangeRatesSnapshot;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExchangeRateRecord {
    pub snapshot: ExchangeRatesSnapshot,
    pub fetch_timestamp: DateTime<Utc>,
    pub source_identifier: String,
    pub metadata: HashMap<String, String>,
}

impl ExchangeRateRecord {
    pub fn new(
        snapshot: ExchangeRatesSnapshot,
        fetch_timestamp: DateTime<Utc>,
        source_identifier: String,
    ) -> Self {
        Self {
            snapshot,
            fetch_timestamp,
            source_identifier,
            metadata: HashMap::new(),
        }
    }
}

#[async_trait]
pub trait StorageAdapter: Send + Sync {
    async fn store_exchange_rates(&self, record: ExchangeRateRecord) -> Result<()>;
    async fn get_latest_exchange_rates(
        &self,
        source_identifier: &str,
    ) -> Result<Option<ExchangeRateRecord>>;
    async fn exchange_rates_exist(&self, source_identifier: &str) -> Result<bool>;
    async fn health_check(&self) -> Result<()>;
}

pub struct BceDatabase {
    storage: Arc<dyn StorageAdapter>,
}

impl BceDatabase {
    pub fn new<S>(storage: S) -> Self
    where
        S: StorageAdapter + 'static,
    {
        Self {
            storage: Arc::new(storage),
        }
    }

    pub async fn get_latest_exchange_rates(
        &self,
        source_identifier: &str,
    ) -> Result<Option<ExchangeRateRecord>> {
        self.storage
            .get_latest_exchange_rates(source_identifier)
            .await
    }

    pub async fn store_exchange_rates(&self, record: ExchangeRateRecord) -> Result<()> {
        self.storage.store_exchange_rates(record).await
    }
}
