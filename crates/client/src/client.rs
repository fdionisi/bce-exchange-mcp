use std::{collections::HashMap, sync::Arc};

use anyhow::{Result, anyhow};
use http_client::{HttpClient, Request, RequestBuilderExt, ResponseAsyncBodyExt};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExchangeRate {
    pub currency: String,
    pub rate: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExchangeRatesSnapshot {
    pub rates: Vec<ExchangeRate>,
    pub timestamp: String,
}

#[derive(Deserialize)]
struct EcbDataResponse {
    #[serde(rename = "dataSets")]
    data_sets: Vec<DataSet>,
    structure: Structure,
}

#[derive(Deserialize)]
struct DataSet {
    series: HashMap<String, Series>,
}

#[derive(Deserialize)]
struct Series {
    observations: HashMap<String, Vec<Option<f64>>>,
}

#[derive(Deserialize)]
struct Structure {
    dimensions: Dimensions,
}

#[derive(Deserialize)]
struct Dimensions {
    series: Vec<Dimension>,
}

#[derive(Deserialize)]
struct Dimension {
    id: String,
    values: Vec<DimensionValue>,
}

#[derive(Deserialize)]
struct DimensionValue {
    id: String,
}

pub struct BceClient {
    http_client: Arc<dyn HttpClient>,
}

impl BceClient {
    pub fn new(http_client: Arc<dyn HttpClient>) -> Self {
        Self { http_client }
    }

    pub async fn fetch_all_exchange_rates(&self) -> Result<ExchangeRatesSnapshot> {
        let url = "https://data-api.ecb.europa.eu/service/data/EXR/D..EUR.SP00.A?format=jsondata&lastNObservations=1";

        let response = self
            .http_client
            .send(
                Request::builder()
                    .method("GET")
                    .uri(url)
                    .header("User-Agent", "ecb-exchange-mcp/0.1.0-alpha.1")
                    .end()?,
            )
            .await?;

        let ecb_response: EcbDataResponse = response.json().await?;

        self.parse_exchange_rates(ecb_response)
    }

    fn parse_exchange_rates(&self, response: EcbDataResponse) -> Result<ExchangeRatesSnapshot> {
        let data_set = response
            .data_sets
            .first()
            .ok_or_else(|| anyhow!("No data sets found in response"))?;

        let currency_dimension = response
            .structure
            .dimensions
            .series
            .iter()
            .find(|d| d.id == "CURRENCY")
            .ok_or_else(|| anyhow!("CURRENCY dimension not found"))?;

        let mut rates = Vec::new();

        for (series_key, series) in &data_set.series {
            let parts: Vec<&str> = series_key.split(':').collect();
            if parts.len() >= 2 {
                if let Ok(currency_index) = parts[1].parse::<usize>() {
                    if let Some(currency_value) = currency_dimension.values.get(currency_index) {
                        if let Some(rate_value) = series
                            .observations
                            .values()
                            .next()
                            .and_then(|obs| obs.first())
                            .and_then(|val| val.to_owned())
                        {
                            rates.push(ExchangeRate {
                                currency: currency_value.id.clone(),
                                rate: rate_value,
                            });
                        }
                    }
                }
            }
        }

        Ok(ExchangeRatesSnapshot {
            rates,
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    }
}
