#!/bin/bash

# Test the MCP endpoint to see what it returns
echo "Testing MCP endpoint without auth (should return error):"
curl -X POST https://doxyde.com/.mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc": "2.0", "method": "initialize", "id": 1}' \
  -s | jq .

echo -e "\n\nTesting with a dummy Bearer token:"
curl -X POST https://doxyde.com/.mcp \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer test-token" \
  -d '{"jsonrpc": "2.0", "method": "initialize", "id": 1}' \
  -s | jq .