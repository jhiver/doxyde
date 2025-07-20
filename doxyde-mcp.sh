#!/bin/bash

# Check if DOXYDE_MCP_URL is set
if [ -z "$DOXYDE_MCP_URL" ]; then
    echo "Error: DOXYDE_MCP_URL environment variable is not set."
    echo "Please set it before running this script:"
    echo "  export DOXYDE_MCP_URL='https://your-site.com/.mcp/your-token'"
    exit 1
fi

exec /Users/jhiver/doxyde/target/release/doxyde-mcp-server
