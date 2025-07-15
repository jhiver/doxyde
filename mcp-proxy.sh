#!/bin/bash

# MCP stdio to HTTP proxy for Doxyde
# This script acts as a bridge between Claude Code's stdio transport and the HTTP MCP server

MCP_URL="http://localhost:3000/.mcp/45a8ae64-3ad7-49b6-b9a8-0d1e2e097fa4"

while IFS= read -r line; do
    # Send the request to the HTTP server
    response=$(echo "$line" | curl -s -X POST "$MCP_URL" \
        -H "Content-Type: application/json" \
        -d @-)
    
    # Output the response
    echo "$response"
done