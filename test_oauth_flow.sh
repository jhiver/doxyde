#!/bin/bash

echo "Testing OAuth flow locally..."

# Test 1: Register a client
echo -e "\n1. Registering OAuth client:"
REGISTER_RESPONSE=$(curl -s -X POST http://localhost:3000/.oauth/register \
  -H "Content-Type: application/json" \
  -d '{
    "client_name": "Test OAuth Client",
    "redirect_uris": ["http://localhost:6274/oauth/callback/debug"]
  }')

echo "$REGISTER_RESPONSE" | jq .

CLIENT_ID=$(echo "$REGISTER_RESPONSE" | jq -r .client_id)
CLIENT_SECRET=$(echo "$REGISTER_RESPONSE" | jq -r .client_secret)

echo -e "\nClient ID: $CLIENT_ID"

# Test 2: Authorization URL
echo -e "\n2. Authorization URL (open in browser when logged in):"
AUTH_URL="http://localhost:3000/.oauth/authorize?response_type=code&client_id=$CLIENT_ID&redirect_uri=http://localhost:6274/oauth/callback/debug&scope=read+write+admin&code_challenge=test_challenge&code_challenge_method=plain"
echo "$AUTH_URL"

echo -e "\n3. After authorizing, use the code from the redirect URL to get a token:"
echo "curl -X POST http://localhost:3000/.oauth/token \\"
echo "  -H 'Content-Type: application/json' \\"
echo "  -d '{"
echo '    "grant_type": "authorization_code",'
echo '    "code": "YOUR_AUTH_CODE",'
echo "    \"code_verifier\": \"test_challenge\""
echo "  }'"