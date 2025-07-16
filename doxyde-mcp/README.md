# Doxyde MCP Proxy Server

The Doxyde MCP proxy server enables AI assistants like Claude to interact with Doxyde CMS instances through the Model Context Protocol (MCP).

## Overview

This is a lightweight proxy server that forwards MCP requests from Claude Desktop to the Doxyde web server's built-in MCP endpoint. The Doxyde web server already implements all MCP tools at `/.mcp/:token_id`, so this proxy simply forwards stdin/stdout JSON-RPC communication to HTTP requests.

## Features

- **Simple Proxy**: Forwards JSON-RPC requests from stdin to HTTP endpoint
- **Token-based Authentication**: Uses MCP tokens generated in Doxyde
- **Lightweight**: No duplicate tool implementations, just forwards requests
- **Compatible**: Works with Claude Desktop's MCP configuration

## Installation

1. Build the MCP proxy server:
   ```bash
   cargo build --release -p doxyde-mcp
   ```

2. The binary will be available at:
   ```
   ./target/release/doxyde-mcp-server
   ```

## Setup

### 1. Generate MCP Token in Doxyde

1. Login to your Doxyde instance
2. Navigate to `/.settings/mcp`
3. Click "Generate New Token"
4. Copy the generated MCP URL (e.g., `http://localhost:3000/.mcp/45a8ae64-3ad7-49b6-b9a8-0d1e2e097fa4`)

### 2. Configure Claude Desktop

Create or update your Claude Desktop configuration:

```json
{
  "mcpServers": {
    "doxyde": {
      "command": "/absolute/path/to/doxyde-mcp-server",
      "args": [],
      "env": {
        "DOXYDE_MCP_URL": "http://localhost:3000/.mcp/your-token-id",
        "RUST_LOG": "info"
      }
    }
  }
}
```

Replace:
- `/absolute/path/to/doxyde-mcp-server` with the actual path to your built binary
- `your-token-id` with the token ID from the MCP URL you generated

### 3. Test the Proxy Server

You can test the MCP proxy server manually:

```bash
export DOXYDE_MCP_URL="http://localhost:3000/.mcp/your-token-id"
./target/release/doxyde-mcp-server
```

Then send a JSON-RPC request via stdin:

```json
{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocol_version": "1.0", "capabilities": {}, "client_info": {"name": "test", "version": "1.0"}}}
```

## Available Tools

The proxy forwards requests to the Doxyde MCP endpoint, which provides:

### Information Tools
- `flip_coin` - Flip a coin one or more times
- `get_current_time` - Get the current time in UTC or a specified timezone

### Site Management
- `list_pages` - Get all pages in the site with hierarchy
- `get_page` - Get full page details by ID
- `get_page_by_path` - Find page by URL path
- `search_pages` - Search pages by title or content

### Content Access
- `get_published_content` - Get published content of a page
- `get_draft_content` - Get draft content of a page (if exists)

### Write Operations
- `create_page` - Create a new page
- `update_page` - Update page title, slug, or template

## Security

- MCP tokens are tied to specific sites for security
- Each token has limited permissions based on the site
- Tokens can be revoked at any time from the Doxyde interface
- All operations respect Doxyde's permission model

## Environment Variables

- `DOXYDE_MCP_URL` (required): The full MCP URL including token
- `RUST_LOG` (optional): Log level (debug, info, warn, error)

## Development

### Running Tests

```bash
cargo test -p doxyde-mcp
```

### Debugging

Enable debug logging:

```bash
export RUST_LOG=debug
./target/release/doxyde-mcp-server
```

Logs are written to stderr, keeping stdout clean for JSON-RPC communication.

## Architecture

The MCP proxy server is a simple forwarder:

1. Reads JSON-RPC requests from stdin
2. Forwards them to the HTTP MCP endpoint
3. Returns responses to stdout
4. All tool implementations are in the Doxyde web server