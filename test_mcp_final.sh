#!/bin/bash

TOKEN=$(sqlite3 doxyde.db "SELECT id FROM mcp_tokens WHERE revoked_at IS NULL LIMIT 1")
echo "Token: $TOKEN"

echo -e "\n=== Testing MCP Error Response Format ==="

echo -e "\n1. Invalid tool name (should return JSON-RPC error):"
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

echo -e "\n2. Valid tool with missing parameter (should return JSON-RPC error):"
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

echo -e "\n3. Valid tool with invalid data (should return JSON-RPC error):"
curl -s -X POST "http://localhost:3000/.mcp/$TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 3,
    "method": "tools/call",
    "params": {
      "name": "create_page",
      "arguments": {
        "parent_page_id": 3,
        "slug": "invalid slug!",
        "title": "Test",
        "template": "default"
      }
    }
  }' | jq .

echo -e "\n4. Valid tool call (should return success):"
curl -s -X POST "http://localhost:3000/.mcp/$TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 4,
    "method": "tools/list"
  }' | jq . | head -20