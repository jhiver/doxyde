#!/bin/bash

# Test MCP error handling

TOKEN=$(sqlite3 doxyde.db "SELECT id FROM mcp_tokens WHERE revoked_at IS NULL LIMIT 1")

if [ -z "$TOKEN" ]; then
    echo "No active MCP token found"
    exit 1
fi

echo "Using MCP token: $TOKEN"

echo -e "\n1. Testing invalid tool name..."
curl -s -X POST "http://localhost:3000/.mcp/$TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/call",
    "params": {
      "name": "invalid_tool",
      "arguments": {}
    }
  }' | jq .

echo -e "\n2. Testing missing required parameter..."
curl -s -X POST "http://localhost:3000/.mcp/$TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 2,
    "method": "tools/call",
    "params": {
      "name": "get_page",
      "arguments": {}
    }
  }' | jq .

echo -e "\n3. Testing invalid page ID..."
curl -s -X POST "http://localhost:3000/.mcp/$TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 3,
    "method": "tools/call",
    "params": {
      "name": "get_page",
      "arguments": {"page_id": 99999}
    }
  }' | jq .

echo -e "\n4. Testing create_page with invalid slug..."
curl -s -X POST "http://localhost:3000/.mcp/$TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 4,
    "method": "tools/call",
    "params": {
      "name": "create_page",
      "arguments": {
        "parent_page_id": 3,
        "slug": "invalid slug with spaces!",
        "title": "Test Page",
        "template": "default"
      }
    }
  }' | jq .