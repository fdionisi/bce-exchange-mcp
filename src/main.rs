use std::{env, path::PathBuf, sync::Arc};

use anyhow::Result;
use bce_exchange_database_sqlite::SqliteStorageAdapter;
use bce_exchange_mcp_primitives::tools::RateConversion;
use bce_exchange_provider::BceExchangeProvider;
use context_server::{ContextServer, ContextServerRpcRequest, ContextServerRpcResponse};
use context_server_utils::tool_registry::ToolRegistry;
use http_client::HttpClient;
use http_client_reqwest::HttpClientReqwest;
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};

fn get_database_directory() -> Result<PathBuf> {
    let home_dir = env::var("HOME")
        .map_err(|_| anyhow::anyhow!("Could not find HOME environment variable"))?;

    let config_dir = PathBuf::from(home_dir)
        .join(".config")
        .join("bce-exchange-mcp")
        .join("database");

    std::fs::create_dir_all(&config_dir)
        .map_err(|e| anyhow::anyhow!("Failed to create database directory: {}", e))?;

    Ok(config_dir)
}

struct ContextServerState {
    rpc: ContextServer,
}

impl ContextServerState {
    async fn new(http_client: Arc<dyn HttpClient>) -> Result<Self> {
        let tool_registry = Arc::new(ToolRegistry::default());

        let db_path = get_database_directory()?.join("exchange.db");

        tool_registry.register(Arc::new(RateConversion::new(BceExchangeProvider::new(
            http_client.clone(),
            SqliteStorageAdapter::new(&db_path.to_string_lossy()).await?,
        ))));

        Ok(Self {
            rpc: ContextServer::builder()
                .with_server_info((env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")))
                .with_tools(tool_registry)
                .build()?,
        })
    }

    async fn process_request(
        &self,
        request: ContextServerRpcRequest,
    ) -> Result<Option<ContextServerRpcResponse>> {
        self.rpc.handle_incoming_message(request).await
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let http_client = Arc::new(HttpClientReqwest::default());

    let state = ContextServerState::new(http_client).await?;

    let mut stdin = BufReader::new(io::stdin()).lines();
    let mut stdout = io::stdout();

    while let Some(line) = stdin.next_line().await? {
        let request: ContextServerRpcRequest = match serde_json::from_str(&line) {
            Ok(req) => req,
            Err(e) => {
                eprintln!("Error parsing request: {}", e);
                continue;
            }
        };

        if let Some(response) = state.process_request(request).await? {
            let response_json = serde_json::to_string(&response)?;
            stdout.write_all(response_json.as_bytes()).await?;
            stdout.write_all(b"\n").await?;
            stdout.flush().await?;
        }
    }

    Ok(())
}
