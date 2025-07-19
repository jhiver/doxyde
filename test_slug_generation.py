#!/usr/bin/env python3
"""
Comprehensive tests for slug generation feature in Doxyde
"""

import json
import asyncio
import logging
import subprocess
from datetime import datetime

# Set up logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(message)s')
logger = logging.getLogger(__name__)

class MCPClient:
    """Client to interact with MCP server"""
    
    async def call_tool(self, tool_name: str, arguments: dict):
        """Call an MCP tool with the given arguments"""
        cmd = ["./doxyde-mcp.sh"]
        request = {
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": arguments
            },
            "id": 1
        }
        
        try:
            proc = await asyncio.create_subprocess_exec(
                *cmd,
                stdin=asyncio.subprocess.PIPE,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE
            )
            
            stdout, stderr = await proc.communicate(json.dumps(request).encode())
            
            if proc.returncode != 0:
                logger.error(f"Command failed: {stderr.decode()}")
                return None
            
            response = json.loads(stdout.decode())
            
            if "error" in response:
                return response["error"]
            
            if "result" in response and "content" in response["result"]:
                for item in response["result"]["content"]:
                    if item["type"] == "text":
                        return json.loads(item["text"])
            
            return None
            
        except Exception as e:
            logger.error(f"Failed to call tool: {e}")
            return None

async def test_slug_generation():
    """Test comprehensive slug generation scenarios"""
    client = MCPClient()
    
    logger.info("🧪 Testing Slug Generation Feature\n")
    
    # Get the root page to use as parent
    pages = await client.call_tool("list_pages", {})
    if not pages or isinstance(pages, dict) and "error" in pages:
        logger.error("Failed to list pages")
        return
    
    root_page = pages[0]["page"]
    root_id = root_page["id"]
    
    # Clean up any leftover test pages first
    logger.info("🧹 Cleaning up any existing test pages...")
    all_pages = await client.call_tool("list_pages", {})
    if all_pages and isinstance(all_pages, list):
        for page_hierarchy in all_pages:
            page = page_hierarchy.get("page", {})
            title = page.get("title", "")
            slug = page.get("slug", "")
            if (title.startswith('Test') or title.startswith('Parent') or 
                'long title' in title.lower() or title == '' or
                slug in ['custom-slug', 'custom-slug-2', 'untitled']):
                await client.call_tool("delete_page", {"page_id": page['id']})
                logger.info(f"  Cleaned up: {title or slug}")
    
    logger.info("")
    
    # Test 1: Create page without slug
    logger.info("📍 Test 1: Create page without slug (should auto-generate)")
    result = await client.call_tool("create_page", {
        "parent_page_id": root_id,
        "title": "Test Page Without Slug"
    })
    
    if result and isinstance(result, dict) and "error" not in result:
        logger.info(f"  ✅ Created page with auto-generated slug: {result.get('slug', 'N/A')}")
        # Check that slug starts with expected base
        assert result['slug'].startswith('test-page-without-slug'), f"Expected slug to start with 'test-page-without-slug', got {result['slug']}"
        page1_id = result.get('id')
        page1_slug = result.get('slug')
    else:
        logger.error(f"  ❌ Failed to create page: {result}")
        return
    
    # Test 2: Create another page with same title (should get suffix)
    logger.info("\n📍 Test 2: Create another page with same title (should get suffix)")
    result = await client.call_tool("create_page", {
        "parent_page_id": root_id,
        "title": "Test Page Without Slug"
    })
    
    if result and "error" not in result:
        logger.info(f"  ✅ Created page with suffix: {result['slug']}")
        # Should have a different slug than the first one
        assert result['slug'] != page1_slug, f"Expected different slug than {page1_slug}, got {result['slug']}"
        assert result['slug'].startswith('test-page-without-slug'), f"Expected slug to start with 'test-page-without-slug', got {result['slug']}"
        page2_id = result['id']
    else:
        logger.error(f"  ❌ Failed to create page: {result}")
    
    # Test 3: Create page with explicit slug
    logger.info("\n📍 Test 3: Create page with explicit slug")
    result = await client.call_tool("create_page", {
        "parent_page_id": root_id,
        "slug": "custom-slug",
        "title": "Page with Custom Slug"
    })
    
    if result and "error" not in result:
        logger.info(f"  ✅ Created page with custom slug: {result['slug']}")
        # If custom-slug already exists, it will get a suffix
        assert result['slug'].startswith('custom-slug'), f"Expected slug to start with 'custom-slug', got {result['slug']}"
        page3_id = result['id']
    else:
        logger.error(f"  ❌ Failed to create page: {result}")
    
    # Test 4: Create page with special characters in title
    logger.info("\n📍 Test 4: Create page with special characters in title")
    result = await client.call_tool("create_page", {
        "parent_page_id": root_id,
        "title": "What's New? Price: $99.99!"
    })
    
    if result and "error" not in result:
        logger.info(f"  ✅ Created page with cleaned slug: {result['slug']}")
        assert result['slug'] == 'what-s-new-price-99-99', f"Expected 'what-s-new-price-99-99', got {result['slug']}"
        page4_id = result['id']
    else:
        logger.error(f"  ❌ Failed to create page: {result}")
    
    # Test 5: Create pages under different parents with same title
    logger.info("\n📍 Test 5: Create pages under different parents with same title")
    
    # Create first parent
    parent1 = await client.call_tool("create_page", {
        "parent_page_id": root_id,
        "title": "Parent One"
    })
    
    # Create second parent
    parent2 = await client.call_tool("create_page", {
        "parent_page_id": root_id,
        "title": "Parent Two"
    })
    
    if parent1 and parent2:
        # Create child with same title under parent1
        child1 = await client.call_tool("create_page", {
            "parent_page_id": parent1['id'],
            "title": "About Us"
        })
        
        # Create child with same title under parent2
        child2 = await client.call_tool("create_page", {
            "parent_page_id": parent2['id'],
            "title": "About Us"
        })
        
        if child1 and child2:
            logger.info(f"  ✅ Created '{child1['slug']}' under Parent One")
            logger.info(f"  ✅ Created '{child2['slug']}' under Parent Two")
            assert child1['slug'] == child2['slug'] == 'about-us', "Both should have 'about-us' slug"
        else:
            logger.error("  ❌ Failed to create child pages")
    else:
        logger.error("  ❌ Failed to create parent pages")
    
    # Test 6: Very long title
    logger.info("\n📍 Test 6: Create page with very long title")
    long_title = "This is a very long title that exceeds one hundred characters and should be truncated to ensure reasonable URL length for better usability and SEO optimization"
    result = await client.call_tool("create_page", {
        "parent_page_id": root_id,
        "title": long_title
    })
    
    if result and "error" not in result:
        logger.info(f"  ✅ Created page with truncated slug: {result['slug']}")
        logger.info(f"  Slug length: {len(result['slug'])} characters")
        assert len(result['slug']) <= 100, f"Slug should be <= 100 chars, got {len(result['slug'])}"
        assert not result['slug'].endswith('-'), "Slug should not end with hyphen"
    else:
        logger.error(f"  ❌ Failed to create page: {result}")
    
    # Test 7: Empty title edge case
    logger.info("\n📍 Test 7: Create page with empty title (should generate 'untitled' slug)")
    result = await client.call_tool("create_page", {
        "parent_page_id": root_id,
        "title": ""
    })
    
    if result and "error" not in result:
        logger.info(f"  ✅ Created page with auto-generated slug: {result['slug']}")
        assert result['slug'] == 'untitled', f"Expected 'untitled', got {result['slug']}"
    else:
        logger.error(f"  ❌ Failed: {result}")
    
    # Cleanup
    logger.info("\n🧹 Cleaning up test pages...")
    test_pages = await client.call_tool("search_pages", {"query": "test"})
    if test_pages and isinstance(test_pages, list):
        for page in test_pages:
            if page['title'].startswith('Test') or page['title'].startswith('Parent') or 'long title' in page['title'].lower() or page['title'] == '':
                result = await client.call_tool("delete_page", {"page_id": page['id']})
                if result and "error" not in result:
                    logger.info(f"  Deleted: {page['title'] or 'untitled'}")
                else:
                    logger.debug(f"  Could not delete: {page['title']}")

async def test_web_form_behavior():
    """Test that web form also handles slug generation correctly"""
    logger.info("\n🌐 Testing Web Form Slug Generation")
    logger.info("  Note: Manual test required - visit web UI and:")
    logger.info("  1. Create new page with only title filled")
    logger.info("  2. Verify slug is auto-generated on save")
    logger.info("  3. Create page with custom slug")
    logger.info("  4. Verify custom slug is preserved")

if __name__ == "__main__":
    asyncio.run(test_slug_generation())