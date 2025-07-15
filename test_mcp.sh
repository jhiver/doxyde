#!/bin/bash

# Replace with your actual token ID
TOKEN_ID="${1:-YOUR_TOKEN_ID}"
BASE_URL="http://localhost:3000/.mcp/$TOKEN_ID"

echo "Testing MCP server at: $BASE_URL"
echo ""

# Test 1: Initialize
echo "1. Testing initialize..."
curl -X POST "$BASE_URL" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}' \
  2>/dev/null | jq .

echo ""

# Test 2: List tools
echo "2. Testing tools/list..."
curl -X POST "$BASE_URL" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  2>/dev/null | jq .

echo ""

# Test 3: Call flip_coin
echo "3. Testing flip_coin tool..."
curl -X POST "$BASE_URL" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -d '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"flip_coin","arguments":{"times":3}}}' \
  2>/dev/null | jq .

echo ""

# Test 4: Call get_current_time
echo "4. Testing get_current_time tool..."
curl -X POST "$BASE_URL" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -d '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"get_current_time","arguments":{}}}' \
  2>/dev/null | jq .

echo ""

# Test 5: Test with SSE Accept header
echo "5. Testing SSE transport..."
curl -X POST "$BASE_URL" \
  -H "Content-Type: application/json" \
  -H "Accept: text/event-stream" \
  -d '{"jsonrpc":"2.0","id":5,"method":"tools/list"}' \
  --no-buffer \
  2>/dev/null

echo ""
echo "Done!"