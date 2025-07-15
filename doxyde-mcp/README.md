# Doxyde MCP Server

The Doxyde MCP (Model Context Protocol) server enables AI assistants like Claude to interact directly with Doxyde CMS instances.

## Features

- **Authentication**: Secure session-based authentication with Doxyde
- **Site Management**: List and manage sites
- **Page Operations**: Create, read, update pages
- **Component Management**: Add and modify page components
- **Content Search**: Search across sites and pages

## Installation

1. Build the MCP server:
   ```bash
   cargo build --release -p doxyde-mcp
   ```

2. The binary will be available at:
   ```
   ./target/release/doxyde-mcp-server
   ```

## Setup

### 1. Create MCP User

First, create a dedicated user for the MCP server in your Doxyde instance:

```bash
# Create the MCP user
./target/release/doxyde user create mcp@system.local mcp_agent --password secure-password

# Grant permissions (replace localhost:3000 with your domain)
./target/release/doxyde user grant mcp_agent localhost:3000 owner
```

### 2. Configure Claude Desktop

Copy `claude_desktop_config.example.json` and update it with your settings:

```json
{
  "mcpServers": {
    "doxyde": {
      "command": "/absolute/path/to/doxyde-mcp-server",
      "args": [],
      "env": {
        "DOXYDE_BASE_URL": "http://localhost:3000",
        "DOXYDE_USERNAME": "mcp_agent",
        "DOXYDE_PASSWORD": "your-secure-password",
        "RUST_LOG": "info"
      }
    }
  }
}
```

Add this configuration to Claude Desktop's settings.

### 3. Test the Server

You can test the MCP server manually:

```bash
export DOXYDE_BASE_URL="http://localhost:3000"
export DOXYDE_USERNAME="mcp_agent"
export DOXYDE_PASSWORD="your-password"
./target/release/doxyde-mcp-server
```

Then send a JSON-RPC request via stdin:

```json
{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocol_version": "1.0", "capabilities": {}, "client_info": {"name": "test", "version": "1.0"}}}
```

## Available Tools

The MCP server provides the following tools:

### Site Management
- `list_sites` - List all accessible sites
- `get_site` - Get details about a specific site

### Page Management
- `list_pages` - List all pages in a site
- `get_page` - Get page details including components
- `create_page` - Create a new page
- `update_page` - Update page properties

### Content Management
- `add_markdown_component` - Add markdown content to a page
  - Supports full Markdown syntax
  - Template options: default, card, highlight, quote, hero, with_title
  - Optional title parameter for components
- `search_content` - Search across all content

## Security

- The MCP server uses session-based authentication
- Sessions expire after 24 hours
- All operations respect Doxyde's permission model
- Credentials should be stored securely (environment variables)

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

The MCP server consists of:

1. **Authentication Module** (`auth.rs`) - Handles login and session management
2. **Message Types** (`messages.rs`) - MCP protocol message definitions
3. **Server Core** (`server.rs`) - JSON-RPC server implementation
4. **Tools** (`tools.rs`) - Doxyde-specific tool implementations

## Future Enhancements

- [ ] Real API integration (currently using mock responses)
- [ ] Image upload support
- [ ] Component reordering
- [ ] Bulk operations
- [ ] WebSocket transport option
- [ ] API token authentication