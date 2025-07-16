#!/bin/bash

# Test MCP create_page functionality

# First, get the MCP token
TOKEN=$(sqlite3 doxyde.db "SELECT id FROM mcp_tokens WHERE revoked_at IS NULL LIMIT 1")

if [ -z "$TOKEN" ]; then
    echo "No active MCP token found"
    exit 1
fi

echo "Using MCP token: $TOKEN"

# Test create_page with curl
echo "Testing create_page..."

curl -X POST "http://localhost:3000/.mcp/$TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/call",
    "params": {
      "name": "create_page",
      "arguments": {
        "parent_page_id": 3,
        "slug": "test-fidgets",
        "title": "Fidgets Foo Test Page",
        "template": "default"
      }
    }
  }' | jq .

echo -e "\n\nChecking if page was created..."
sqlite3 doxyde.db "SELECT id, slug, title, parent_page_id FROM pages WHERE slug = 'test-fidgets'"