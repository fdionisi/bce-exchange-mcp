use anyhow::{Ok, Result, anyhow};
use async_trait::async_trait;
use bce_exchange_provider::BceExchangeProvider;
use context_server::{Tool, ToolContent, ToolExecutor};
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Deserialize, JsonSchema, Serialize)]
struct RateConversionParams {
    #[schemars(description = "The currency to convert from (e.g., EUR, USD, JPY)")]
    from_currency: String,
    #[schemars(description = "The target currency to convert to (e.g., EUR, USD, JPY)")]
    target_currency: String,
}

pub struct RateConversion {
    ecb_exchange_provider: BceExchangeProvider,
}

impl RateConversion {
    pub fn new(ecb_exchange_provider: BceExchangeProvider) -> Self {
        Self {
            ecb_exchange_provider,
        }
    }
}

#[async_trait]
impl ToolExecutor for RateConversion {
    async fn execute(&self, arguments: Option<Value>) -> Result<Vec<ToolContent>> {
        let params = {
            let p = arguments.ok_or(anyhow!("Missing arguments"))?;
            serde_json::from_value::<RateConversionParams>(p)
                .map_err(|_| anyhow!("Invalid arguments"))?
        };

        let result = self
            .ecb_exchange_provider
            .rate_conversion(&params.from_currency, &params.target_currency)
            .await?;

        Ok(vec![ToolContent::Text {
            text: json!({
                params.from_currency: 1.0.to_string(),
                params.target_currency: result.to_string(),
            })
            .to_string(),
        }])
    }

    fn to_tool(&self) -> Tool {
        Tool {
            name: "rate_conversion".into(),
            description: Some("Convert between different currencies using ECB exchange rates. Supports major currencies including USD, JPY, BGN, CZK, DKK, GBP, HUF, PLN, RON, SEK, CHF, ISK, NOK, TRY, AUD, BRL, CAD, CNY, HKD, IDR, ILS, INR, KRW, MXN, MYR, NZD, PHP, SGD, THB, ZAR".into()),
            input_schema: schema_for!(RateConversionParams).to_value(),
        }
    }
}
