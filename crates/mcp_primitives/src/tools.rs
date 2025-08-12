use anyhow::{Ok, Result, anyhow};
use async_trait::async_trait;
use bce_exchange_provider::BceExchangeProvider;
use context_server::{Tool, ToolContent, ToolExecutor};
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Deserialize, JsonSchema, Serialize)]
struct CurrencyValue {
    #[schemars(description = "The currency code (e.g., EUR, USD, JPY)")]
    currency: String,
    #[schemars(description = "The amount to convert")]
    amount: f64,
}

#[derive(Deserialize, JsonSchema, Serialize)]
struct RateConversionItem {
    #[schemars(description = "Currency value to convert from")]
    from_value: CurrencyValue,
    #[schemars(description = "The target currency to convert to (e.g., EUR, USD, JPY)")]
    target_currency: String,
}

#[derive(Deserialize, JsonSchema, Serialize)]
struct RateConversionParams {
    #[schemars(description = "Array of currency conversions to perform")]
    conversions: Vec<RateConversionItem>,
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

        let mut results = Vec::new();

        for conversion in params.conversions {
            let rate = self
                .ecb_exchange_provider
                .rate_conversion(&conversion.from_value.currency, &conversion.target_currency)
                .await?;

            let converted_amount = conversion.from_value.amount * rate;

            results.push(json!({
                "rate": rate,
                "from": {
                    "currency": conversion.from_value.currency,
                    "amount": conversion.from_value.amount
                },
                "to": {
                    "currency": conversion.target_currency,
                    "amount": converted_amount
                }
            }));
        }

        Ok(vec![ToolContent::Text {
            text: json!(results).to_string(),
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
