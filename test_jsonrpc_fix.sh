#!/bin/bash

echo "Testing JSON-RPC ID handling in MCP endpoint"
echo "============================================"

# Test with valid token (replace with an actual token if you have one)
TOKEN="test-token-12345"

echo -e "\n1. Test with numeric ID (should return same ID in error):"
curl -X POST http://localhost:3000/.mcp \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"jsonrpc":"2.0","method":"initialize","id":123}' \
  -s | jq .

echo -e "\n2. Test with string ID (should return same ID in error):"
curl -X POST http://localhost:3000/.mcp \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"jsonrpc":"2.0","method":"initialize","id":"test-123"}' \
  -s | jq .

echo -e "\n3. Test with no ID (notification - should have no ID in response):"
curl -X POST http://localhost:3000/.mcp \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"jsonrpc":"2.0","method":"initialize"}' \
  -s | jq .

echo -e "\n4. Test with null ID (should have no ID in response):"
curl -X POST http://localhost:3000/.mcp \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $TOKEN" \
  -d '{"jsonrpc":"2.0","method":"initialize","id":null}' \
  -s | jq .

echo -e "\n5. Test without Authorization header (should return error with proper ID):"
curl -X POST http://localhost:3000/.mcp \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"initialize","id":"no-auth-test"}' \
  -s | jq .

echo -e "\nDone!"