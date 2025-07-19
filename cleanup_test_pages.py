#!/usr/bin/env python3
"""Clean up all test pages from database"""

import json
import asyncio
import subprocess
import logging

logging.basicConfig(level=logging.INFO)
logger = logging.getLogger(__name__)

async def cleanup():
    """Delete all test-related pages"""
    cmd = ["./doxyde-mcp.sh"]
    
    # List all pages
    request = {
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "list_pages",
            "arguments": {}
        },
        "id": 1
    }
    
    proc = await asyncio.create_subprocess_exec(
        *cmd,
        stdin=asyncio.subprocess.PIPE,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE
    )
    
    stdout, stderr = await proc.communicate(json.dumps(request).encode())
    response = json.loads(stdout.decode())
    
    if "result" in response and "content" in response["result"]:
        pages = json.loads(response["result"]["content"][0]["text"])
        
        deleted = 0
        for page_hierarchy in pages:
            page = page_hierarchy.get("page", {})
            title = page.get("title", "")
            slug = page.get("slug", "")
            
            # Delete any test-related pages
            if (title.startswith('Test') or 
                title.startswith('Parent') or 
                title.startswith('What') or
                'long title' in title.lower() or 
                title == '' or
                slug.startswith('test-') or
                slug.startswith('custom-') or
                slug.startswith('parent') or
                slug in ['untitled', 'about-us', 'what-s-new-price-99-99']):
                
                # Delete the page
                delete_request = {
                    "jsonrpc": "2.0",
                    "method": "tools/call",
                    "params": {
                        "name": "delete_page",
                        "arguments": {"page_id": page['id']}
                    },
                    "id": 2
                }
                
                proc = await asyncio.create_subprocess_exec(
                    *cmd,
                    stdin=asyncio.subprocess.PIPE,
                    stdout=asyncio.subprocess.PIPE,
                    stderr=asyncio.subprocess.PIPE
                )
                
                await proc.communicate(json.dumps(delete_request).encode())
                logger.info(f"Deleted: {title or slug}")
                deleted += 1
        
        logger.info(f"\nTotal pages deleted: {deleted}")

if __name__ == "__main__":
    asyncio.run(cleanup())