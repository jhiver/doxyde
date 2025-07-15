use anyhow::{Context, Result};
use jsonrpc_core::{IoHandler, Params};
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Write};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::auth::AuthClient;
use crate::messages::*;
use crate::tools::ToolHandler;

pub struct McpServer {
    auth_client: Arc<AuthClient>,
    _tool_handler: Arc<ToolHandler>,
    io_handler: IoHandler,
    _initialized: Arc<RwLock<bool>>,
}

impl McpServer {
    pub fn new(auth_client: AuthClient, tool_handler: ToolHandler) -> Self {
        let auth_client = Arc::new(auth_client);
        let tool_handler = Arc::new(tool_handler);
        let initialized = Arc::new(RwLock::new(false));

        let mut io_handler = IoHandler::new();

        // Initialize method
        {
            let initialized = initialized.clone();
            io_handler.add_method("initialize", move |params: Params| {
                let initialized = initialized.clone();
                async move {
                    let _params: InitializeParams = params
                        .parse()
                        .map_err(|e| jsonrpc_core::Error::invalid_params(e.to_string()))?;

                    let mut init = initialized.write().await;
                    *init = true;

                    let response = InitializeResponse {
                        protocol_version: "1.0".to_string(),
                        capabilities: ServerCapabilities {
                            tools: Some(HashMap::new()),
                            resources: Some(HashMap::new()),
                            prompts: None,
                            logging: Some(HashMap::new()),
                        },
                        server_info: ServerInfo {
                            name: "doxyde-mcp".to_string(),
                            version: env!("CARGO_PKG_VERSION").to_string(),
                        },
                    };

                    Ok(serde_json::to_value(response).unwrap())
                }
            });
        }

        // List tools method
        {
            let tool_handler = tool_handler.clone();
            let initialized = initialized.clone();
            io_handler.add_method("tools/list", move |_params: Params| {
                let tool_handler = tool_handler.clone();
                let initialized = initialized.clone();
                async move {
                    // Check if initialized
                    let init = initialized.read().await;
                    if !*init {
                        return Err(jsonrpc_core::Error::invalid_request());
                    }
                    drop(init);

                    let tools = tool_handler.list_tools();
                    let response = ListToolsResponse { tools };
                    Ok(serde_json::to_value(response).unwrap())
                }
            });
        }

        // Call tool method
        {
            let tool_handler = tool_handler.clone();
            let initialized = initialized.clone();
            io_handler.add_method("tools/call", move |params: Params| {
                let tool_handler = tool_handler.clone();
                let initialized = initialized.clone();
                async move {
                    // Check if initialized
                    let init = initialized.read().await;
                    if !*init {
                        return Err(jsonrpc_core::Error::invalid_request());
                    }
                    drop(init);

                    let params: CallToolParams = params
                        .parse()
                        .map_err(|e| jsonrpc_core::Error::invalid_params(e.to_string()))?;

                    match tool_handler.call_tool(&params.name, params.arguments).await {
                        Ok(content) => {
                            let response = CallToolResponse { content };
                            Ok(serde_json::to_value(response).unwrap())
                        }
                        Err(e) => Err(jsonrpc_core::Error::invalid_params(e.to_string())),
                    }
                }
            });
        }

        // List resources method
        {
            let initialized = initialized.clone();
            io_handler.add_method("resources/list", move |_params: Params| {
                let initialized = initialized.clone();
                async move {
                    // Check if initialized
                    let init = initialized.read().await;
                    if !*init {
                        return Err(jsonrpc_core::Error::invalid_request());
                    }

                    // For now, return empty resources
                    let response = ListResourcesResponse { resources: vec![] };
                    Ok(serde_json::to_value(response).unwrap())
                }
            });
        }

        Self {
            auth_client,
            _tool_handler: tool_handler,
            io_handler,
            _initialized: initialized,
        }
    }

    pub async fn run(&self) -> Result<()> {
        tracing::info!("Starting MCP server on stdio");

        // Authenticate with Doxyde
        self.auth_client
            .login()
            .await
            .context("Failed to authenticate with Doxyde")?;

        // Read from stdin
        let stdin = io::stdin();
        let reader = BufReader::new(stdin);

        for line in reader.lines() {
            let line = line.context("Failed to read from stdin")?;
            if line.trim().is_empty() {
                continue;
            }

            tracing::debug!("Received: {}", line);

            // Process the JSON-RPC request
            let response = self.io_handler.handle_request(&line).await;

            if let Some(response) = response {
                // Write response to stdout
                println!("{}", response);
                io::stdout().flush().context("Failed to flush stdout")?;
                tracing::debug!("Sent: {}", response);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthCredentials;
    use reqwest::Url;

    #[tokio::test]
    async fn test_server_creation() {
        let base_url = Url::parse("http://localhost:3000").unwrap();
        let creds = AuthCredentials {
            username: "test".to_string(),
            password: "test".to_string(),
        };
        let auth_client = AuthClient::new(base_url, creds).unwrap();
        let tool_handler = ToolHandler::new(auth_client.clone());

        let server = McpServer::new(auth_client, tool_handler);
        assert!(!*server._initialized.read().await);
    }
}
