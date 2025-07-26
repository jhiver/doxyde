#!/bin/bash

echo "Testing RMCP integration with Doxyde"
echo "===================================="

# Base URL
BASE_URL="http://localhost:3000"

# First, create an admin user if needed
echo "Creating admin user..."
./target/debug/doxyde user create admin@example.com admin --admin --password admin123 || echo "Admin user might already exist"

echo ""
echo "Starting Doxyde server..."
echo "Please run: cargo run --bin doxyde-web"
echo "Press Enter when the server is running..."
read

# Login as admin to get session
echo "Logging in as admin..."
COOKIE_JAR=$(mktemp)
curl -c "$COOKIE_JAR" -X POST "$BASE_URL/.login" \
  -H "Content-Type: application/x-www-form-urlencoded" \
  -d "email=admin@example.com&password=admin123" \
  -v

echo ""
echo "Creating MCP token..."
TOKEN_RESPONSE=$(curl -b "$COOKIE_JAR" -X POST "$BASE_URL/.mcp/token" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Test RMCP Token",
    "scopes": "read write",
    "expires_in_days": 30
  }' \
  -s)

echo "Token response: $TOKEN_RESPONSE"

# Extract token from response
TOKEN=$(echo "$TOKEN_RESPONSE" | jq -r '.token // empty')

if [ -z "$TOKEN" ]; then
  echo "Failed to create token. Response: $TOKEN_RESPONSE"
  rm "$COOKIE_JAR"
  exit 1
fi

echo ""
echo "Token created successfully: $TOKEN"

echo ""
echo "Testing MCP endpoint with token..."
curl -X POST "$BASE_URL/.mcp" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "initialize",
    "params": {
      "protocolVersion": "1.0.0",
      "capabilities": {},
      "clientInfo": {
        "name": "test-client",
        "version": "1.0.0"
      }
    },
    "id": 1
  }' \
  -v

echo ""
echo "Testing list_tools..."
curl -X POST "$BASE_URL/.mcp" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/list",
    "params": {},
    "id": 2
  }' \
  -v

echo ""
echo "Testing time tool..."
curl -X POST "$BASE_URL/.mcp" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "tools/call",
    "params": {
      "name": "time",
      "arguments": {
        "timezone": "America/New_York"
      }
    },
    "id": 3
  }' \
  -v

rm "$COOKIE_JAR"
echo ""
echo "Test complete!"