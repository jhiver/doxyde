#!/bin/bash

# Replace with your actual token ID
TOKEN_ID="${1:-45a8ae64-3ad7-49b6-b9a8-0d1e2e097fa4}"
BASE_URL="http://localhost:3000/.mcp/$TOKEN_ID"

echo "Testing Phase 1 MCP Tools"
echo "========================"
echo "MCP Server: $BASE_URL"
echo ""

# Test 1: List pages to get the root page info
echo "1. Testing list_pages..."
echo "Expected: Should return root page with path '/' (not '/home')"
LIST_RESPONSE=$(curl -s -X POST "$BASE_URL" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"list_pages","arguments":{}}}')

echo "$LIST_RESPONSE" | jq .
echo ""

# Extract page ID from response
PAGE_ID=$(echo "$LIST_RESPONSE" | jq -r '.result.content[0].text' | jq -r '.[0].page.id')
echo "Found root page ID: $PAGE_ID"
echo ""

# Test 2: Get page details by ID
echo "2. Testing get_page with ID $PAGE_ID..."
echo "Expected: Should return full page model data"
curl -s -X POST "$BASE_URL" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -d "{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/call\",\"params\":{\"name\":\"get_page\",\"arguments\":{\"page_id\":$PAGE_ID}}}" | jq .
echo ""

# Test 3: Get page by path
echo "3. Testing get_page_by_path with path '/'..."
echo "Expected: Should return the root page"
curl -s -X POST "$BASE_URL" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -d '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"get_page_by_path","arguments":{"path":"/"}}}' | jq .
echo ""

# Test 4: Get published content
echo "4. Testing get_published_content for page $PAGE_ID..."
echo "Expected: Should return array of components with ALL fields as JSON"
CONTENT_RESPONSE=$(curl -s -X POST "$BASE_URL" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -d "{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"tools/call\",\"params\":{\"name\":\"get_published_content\",\"arguments\":{\"page_id\":$PAGE_ID}}}")

echo "$CONTENT_RESPONSE" | jq .

# Parse and display component structure
echo ""
echo "Component structure analysis:"
echo "$CONTENT_RESPONSE" | jq -r '.result.content[0].text' | jq '.' 2>/dev/null || echo "No components found"
echo ""

# Test 5: Get draft content
echo "5. Testing get_draft_content for page $PAGE_ID..."
echo "Expected: Either draft components or 'No draft version exists' message"
curl -s -X POST "$BASE_URL" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -d "{\"jsonrpc\":\"2.0\",\"id\":5,\"method\":\"tools/call\",\"params\":{\"name\":\"get_draft_content\",\"arguments\":{\"page_id\":$PAGE_ID}}}" | jq .
echo ""

# Test 6: Search pages
echo "6. Testing search_pages..."
echo "a) Searching for 'Doxyde'..."
curl -s -X POST "$BASE_URL" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -d '{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"search_pages","arguments":{"query":"Doxyde"}}}' | jq .
echo ""

echo "b) Searching for 'test'..."
curl -s -X POST "$BASE_URL" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json" \
  -d '{"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"search_pages","arguments":{"query":"test"}}}' | jq .
echo ""

echo "Phase 1 MCP Tools Test Complete!"
echo ""
echo "Summary of findings:"
echo "- Root page path should be '/' not '/home'"
echo "- Components should expose ALL fields as JSON (title, content, template, timestamps)"
echo "- All component data should be accessible, not simplified"