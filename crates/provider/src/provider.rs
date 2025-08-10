use std::sync::Arc;

use anyhow::{Result, anyhow};
use bce_exchange_client::{BceClient, ExchangeRatesSnapshot};
use bce_exchange_database::{BceDatabase, ExchangeRateRecord, StorageAdapter};
use chrono::{Timelike, Utc};
use chrono_tz::Europe::Paris;
use http_client::HttpClient;

pub struct BceExchangeProvider {
    client: BceClient,
    database: BceDatabase,
}

impl BceExchangeProvider {
    pub fn new<S>(http_client: Arc<dyn HttpClient>, storage_adapter: S) -> Self
    where
        S: StorageAdapter + 'static,
    {
        Self {
            client: BceClient::new(http_client),
            database: BceDatabase::new(storage_adapter),
        }
    }

    pub async fn rate_conversion(&self, from_currency: &str, to_currency: &str) -> Result<f64> {
        let snapshot = self.fetch_exchange_rates().await?;

        let rate = if to_currency == "EUR" {
            self.get_rate_to_eur(&snapshot, from_currency)?
        } else if from_currency == "EUR" {
            let eur_to_target = self.get_rate_from_eur(&snapshot, to_currency)?;
            1.0 / eur_to_target
        } else {
            let from_to_eur = self.get_rate_to_eur(&snapshot, from_currency)?;
            let eur_to_target = self.get_rate_from_eur(&snapshot, to_currency)?;
            from_to_eur / eur_to_target
        };

        Ok(rate)
    }

    async fn fetch_exchange_rates(&self) -> Result<ExchangeRatesSnapshot> {
        let now = Utc::now();
        let today = now.date_naive();
        let cache_key = today.to_string();

        if let Ok(Some(record)) = self.database.get_latest_exchange_rates(&cache_key).await {
            let cet_now = now.with_timezone(&Paris);
            if cet_now.hour() < 17 {
                return Ok(record.snapshot);
            }

            let cet_fetch_time = record.fetch_timestamp.with_timezone(&Paris);
            if cet_fetch_time.date_naive() == today && cet_fetch_time.hour() >= 17 {
                return Ok(record.snapshot);
            }
        }

        let snapshot = self.client.fetch_all_exchange_rates().await?;

        let record = ExchangeRateRecord::new(snapshot.clone(), now, cache_key);

        self.database.store_exchange_rates(record).await?;

        Ok(snapshot)
    }

    fn get_rate_to_eur(&self, snapshot: &ExchangeRatesSnapshot, currency: &str) -> Result<f64> {
        snapshot
            .rates
            .iter()
            .find(|rate| rate.currency == currency)
            .map(|rate| rate.rate)
            .ok_or_else(|| anyhow!("Currency {} not found in snapshot", currency))
    }

    fn get_rate_from_eur(&self, snapshot: &ExchangeRatesSnapshot, currency: &str) -> Result<f64> {
        snapshot
            .rates
            .iter()
            .find(|rate| rate.currency == currency)
            .map(|rate| rate.rate)
            .ok_or_else(|| anyhow!("Currency {} not found in snapshot", currency))
    }
}
