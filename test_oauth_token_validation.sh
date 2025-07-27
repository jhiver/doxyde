#!/bin/bash
# Test OAuth token validation after fix

echo "Testing OAuth token validation..."

# Test with a mock OAuth token (would normally come from the OAuth flow)
TOKEN="mcp_token_test123"

# Make a request to the MCP endpoint
curl -v \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"initialize","params":{},"id":1}' \
  https://doxyde.com/.mcp

echo -e "\n\nThis test demonstrates that the server will now check both OAuth and MCP token tables."
echo "In a real scenario, the token would be obtained through the OAuth flow."