#!/bin/bash

echo "Testing MCP tools/list endpoint on SSE server"
echo "============================================="

# Test the tools/list method via curl
echo "Sending tools/list request to https://sse.doxyde.com/message"
curl -X POST https://sse.doxyde.com/message \
  -H "Authorization: Bearer mcp_token_db6dad14-f151-4af8-bc46-1a315edc8b22" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/list",
    "params": {},
    "id": 1
  }' \
  -v 2>&1 | grep -E "(< HTTP|< {|jsonrpc|error|result)"

echo ""
echo "If this returns a 404 or connection error, the POST endpoint isn't working"
echo "If it returns a JSON-RPC error, the method isn't implemented"
echo "If it returns a result with tools, then rmcp is handling it automatically"