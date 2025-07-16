use anyhow::{Context, Result};
use reqwest::{Client, Url};
use serde_json::Value;
use std::io::{self, BufRead, BufReader, Write};

pub struct McpProxyServer {
    mcp_url: Url,
    client: Client,
}

impl McpProxyServer {
    pub fn new(mcp_url: Url) -> Self {
        Self {
            mcp_url,
            client: Client::new(),
        }
    }

    pub async fn run(&self) -> Result<()> {
        tracing::info!("Starting MCP proxy server on stdio");
        tracing::info!("Forwarding requests to: {}", self.mcp_url);

        // Read from stdin
        let stdin = io::stdin();
        let reader = BufReader::new(stdin);

        for line in reader.lines() {
            let line = line.context("Failed to read from stdin")?;
            if line.trim().is_empty() {
                continue;
            }

            tracing::debug!("Received: {}", line);

            // Forward the JSON-RPC request to the HTTP endpoint
            let response = self.forward_request(&line).await?;

            // Write response to stdout
            println!("{}", response);
            io::stdout().flush().context("Failed to flush stdout")?;
            tracing::debug!("Sent: {}", response);
        }

        Ok(())
    }

    async fn forward_request(&self, request: &str) -> Result<String> {
        // Parse the request to ensure it's valid JSON
        let _json: Value = serde_json::from_str(request).context("Invalid JSON-RPC request")?;

        // Forward the request to the HTTP MCP endpoint
        let response = self
            .client
            .post(self.mcp_url.clone())
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .body(request.to_string())
            .send()
            .await
            .context("Failed to forward request to MCP endpoint")?;

        // Check if the response is successful
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!("MCP endpoint returned error: {} - {}", status, body);

            // Return a JSON-RPC error response
            let error_response = serde_json::json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": {
                    "code": -32603,
                    "message": format!("MCP endpoint error: {}", status)
                }
            });
            return Ok(error_response.to_string());
        }

        // Get the response body
        let response_body = response
            .text()
            .await
            .context("Failed to read response body")?;

        Ok(response_body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_server_creation() {
        let mcp_url = Url::parse("http://localhost:3000/.mcp/test-token").unwrap();
        let server = McpProxyServer::new(mcp_url);
        assert_eq!(
            server.mcp_url.as_str(),
            "http://localhost:3000/.mcp/test-token"
        );
    }
}

