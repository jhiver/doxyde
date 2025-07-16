use anyhow::{Context, Result};
use clap::Parser;
use doxyde_mcp::server::McpProxyServer;
use reqwest::Url;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "doxyde-mcp-server")]
#[command(about = "MCP proxy server for Doxyde CMS", long_about = None)]
struct Args {
    /// MCP URL including token (e.g., http://localhost:3000/.mcp/token-id)
    #[arg(long, env = "DOXYDE_MCP_URL")]
    mcp_url: String,

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

    tracing::info!("Starting Doxyde MCP proxy server");
    tracing::info!("Proxying to: {}", args.mcp_url);

    // Parse MCP URL
    let mcp_url = Url::parse(&args.mcp_url).context("Invalid MCP URL")?;

    // Create and run MCP proxy server
    let server = McpProxyServer::new(mcp_url);
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
            "--mcp-url",
            "http://localhost:3000/.mcp/test-token",
        ];

        let parsed = Args::try_parse_from(args);
        assert!(parsed.is_ok());

        let args = parsed.unwrap();
        assert_eq!(args.mcp_url, "http://localhost:3000/.mcp/test-token");
    }
}
