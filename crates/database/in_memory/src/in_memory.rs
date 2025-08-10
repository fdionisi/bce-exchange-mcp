use std::{collections::HashMap, sync::Arc};

use anyhow::Result;
use async_trait::async_trait;
use bce_exchange_client::ExchangeRatesSnapshot;
use bce_exchange_database::{ExchangeRateRecord, StorageAdapter};
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

pub struct InMemoryStorageAdapter {
    pub(crate) cache: Arc<RwLock<HashMap<String, ExchangeRateRecord>>>,
}

impl InMemoryStorageAdapter {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get_exchange_rates(
        &self,
        cache_key: &str,
    ) -> Option<(ExchangeRatesSnapshot, DateTime<Utc>)> {
        let cache = self.cache.read().await;
        cache
            .get(cache_key)
            .map(|record| (record.snapshot.clone(), record.fetch_timestamp))
    }

    pub async fn store_exchange_rates(
        &self,
        cache_key: String,
        snapshot: ExchangeRatesSnapshot,
        fetch_time: DateTime<Utc>,
    ) {
        let record = ExchangeRateRecord::new(snapshot, fetch_time, cache_key.clone());
        let mut cache = self.cache.write().await;
        cache.insert(cache_key, record);
    }
}

#[async_trait]
impl StorageAdapter for InMemoryStorageAdapter {
    async fn store_exchange_rates(&self, record: ExchangeRateRecord) -> Result<()> {
        let mut cache = self.cache.write().await;
        cache.insert(record.source_identifier.clone(), record);
        Ok(())
    }

    async fn get_latest_exchange_rates(
        &self,
        source_identifier: &str,
    ) -> Result<Option<ExchangeRateRecord>> {
        let cache = self.cache.read().await;
        Ok(cache.get(source_identifier).cloned())
    }

    async fn exchange_rates_exist(&self, source_identifier: &str) -> Result<bool> {
        let cache = self.cache.read().await;
        Ok(cache.contains_key(source_identifier))
    }

    async fn health_check(&self) -> Result<()> {
        // For in-memory storage, always healthy
        Ok(())
    }
}
