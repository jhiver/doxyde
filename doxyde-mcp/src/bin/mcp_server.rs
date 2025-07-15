use anyhow::{Context, Result};
use clap::Parser;
use doxyde_mcp::{
    auth::{AuthClient, AuthCredentials},
    server::McpServer,
    tools::ToolHandler,
};
use reqwest::Url;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "doxyde-mcp-server")]
#[command(about = "MCP server for Doxyde CMS", long_about = None)]
struct Args {
    /// Base URL of the Doxyde server
    #[arg(long, env = "DOXYDE_BASE_URL", default_value = "http://localhost:3000")]
    base_url: String,

    /// Username for authentication
    #[arg(long, env = "DOXYDE_USERNAME")]
    username: String,

    /// Password for authentication
    #[arg(long, env = "DOXYDE_PASSWORD")]
    password: String,

    /// Log level
    #[arg(long, env = "RUST_LOG", default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Initialize logging
    let filter = EnvFilter::try_new(&args.log_level).unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr) // Log to stderr so stdout is clean for JSON-RPC
        .init();

    tracing::info!("Starting Doxyde MCP server");
    tracing::info!("Connecting to Doxyde at: {}", args.base_url);

    // Parse base URL
    let base_url = Url::parse(&args.base_url).context("Invalid base URL")?;

    // Create authentication client
    let credentials = AuthCredentials {
        username: args.username,
        password: args.password,
    };
    let auth_client = AuthClient::new(base_url, credentials)?;

    // Create tool handler
    let tool_handler = ToolHandler::new(auth_client.clone());

    // Create and run MCP server
    let server = McpServer::new(auth_client, tool_handler);
    server.run().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_args_parsing() {
        let args = vec![
            "doxyde-mcp-server",
            "--username",
            "test",
            "--password",
            "pass",
            "--base-url",
            "http://localhost:8080",
        ];

        let parsed = Args::try_parse_from(args);
        assert!(parsed.is_ok());

        let args = parsed.unwrap();
        assert_eq!(args.username, "test");
        assert_eq!(args.base_url, "http://localhost:8080");
    }
}
