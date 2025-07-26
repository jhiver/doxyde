#!/bin/bash

echo "Testing OAuth CORS endpoints locally..."

# Test OPTIONS preflight for /.oauth/register
echo -e "\n1. Testing OPTIONS /.oauth/register:"
curl -i -X OPTIONS http://localhost:3000/.oauth/register \
  -H "Origin: http://localhost:6274" \
  -H "Access-Control-Request-Method: POST" \
  -H "Access-Control-Request-Headers: Content-Type"

# Test POST to /.oauth/register
echo -e "\n\n2. Testing POST /.oauth/register:"
curl -i -X POST http://localhost:3000/.oauth/register \
  -H "Origin: http://localhost:6274" \
  -H "Content-Type: application/json" \
  -d '{
    "client_name": "MCP Inspector Test",
    "redirect_uris": ["http://localhost:6274/callback"]
  }'

# Test OPTIONS preflight for /.oauth/authorize
echo -e "\n\n3. Testing OPTIONS /.oauth/authorize:"
curl -i -X OPTIONS http://localhost:3000/.oauth/authorize \
  -H "Origin: http://localhost:6274" \
  -H "Access-Control-Request-Method: GET" \
  -H "Access-Control-Request-Headers: Authorization"

# Test OPTIONS preflight for /.oauth/token
echo -e "\n\n4. Testing OPTIONS /.oauth/token:"
curl -i -X OPTIONS http://localhost:3000/.oauth/token \
  -H "Origin: http://localhost:6274" \
  -H "Access-Control-Request-Method: POST" \
  -H "Access-Control-Request-Headers: Content-Type, Authorization"