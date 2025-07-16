#!/bin/bash

TOKEN=$(sqlite3 doxyde.db "SELECT id FROM mcp_tokens WHERE revoked_at IS NULL LIMIT 1")
echo "Token: $TOKEN"

echo -e "\nTesting error response format..."

# Test with invalid tool name - should get JSON-RPC error
curl -s -X POST "http://localhost:3000/.mcp/$TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 123,
    "method": "tools/call",
    "params": {
      "name": "invalid_tool_name",
      "arguments": {}
    }
  }' | jq .