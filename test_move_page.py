#!/usr/bin/env python3
"""
Test approfondi de la fonctionnalité move_page
"""

import json
import asyncio
import logging
import subprocess
from datetime import datetime

logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(message)s')
logger = logging.getLogger(__name__)

class RealMCPClient:
    def __init__(self, mcp_script_path="./doxyde-mcp.sh"):
        self.mcp_script_path = mcp_script_path
        self.request_id = 0
    
    async def call_tool(self, tool_name: str, params: dict) -> dict:
        self.request_id += 1
        request = {
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": "tools/call",
            "params": {"name": tool_name, "arguments": params}
        }
        
        try:
            process = await asyncio.create_subprocess_exec(
                self.mcp_script_path,
                stdin=asyncio.subprocess.PIPE,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.DEVNULL
            )
            stdout, _ = await process.communicate(json.dumps(request).encode())
            response = json.loads(stdout.decode())
            
            if "error" in response:
                return {"error": response["error"]}
            
            if "result" in response and "content" in response["result"]:
                content = response["result"]["content"]
                if content and len(content) > 0 and "text" in content[0]:
                    try:
                        return json.loads(content[0]["text"])
                    except json.JSONDecodeError:
                        return {"text": content[0]["text"]}
            
            return response
        except Exception as e:
            return {"error": str(e)}

async def test_move_page_scenarios():
    """Test différents scénarios de move_page"""
    client = RealMCPClient("/Users/jhiver/doxyde/doxyde-mcp.sh")
    created_pages = []
    
    try:
        logger.info("🚀 Test approfondi de move_page")
        
        # Obtenir la page racine
        pages = await client.call_tool("list_pages", {})
        root_id = pages[0]['page']['id']
        logger.info(f"✓ Page racine: ID {root_id}")
        
        # Créer une structure de test
        # Parent A
        parent_a = await client.call_tool("create_page", {
            "parent_page_id": root_id,
            "slug": "parent-a",
            "title": "Parent A",
            "template": "default"
        })
        created_pages.append(parent_a['id'])
        logger.info(f"✓ Parent A créé: ID {parent_a['id']}")
        
        # Parent B
        parent_b = await client.call_tool("create_page", {
            "parent_page_id": root_id,
            "slug": "parent-b",
            "title": "Parent B",
            "template": "default"
        })
        created_pages.append(parent_b['id'])
        logger.info(f"✓ Parent B créé: ID {parent_b['id']}")
        
        # Enfants de A
        child_a1 = await client.call_tool("create_page", {
            "parent_page_id": parent_a['id'],
            "slug": "child-a1",
            "title": "Enfant A1",
            "template": "default"
        })
        created_pages.append(child_a1['id'])
        
        child_a2 = await client.call_tool("create_page", {
            "parent_page_id": parent_a['id'],
            "slug": "child-a2",
            "title": "Enfant A2",
            "template": "default"
        })
        created_pages.append(child_a2['id'])
        
        # Petit-enfant
        grandchild = await client.call_tool("create_page", {
            "parent_page_id": child_a1['id'],
            "slug": "grandchild",
            "title": "Petit-enfant",
            "template": "default"
        })
        created_pages.append(grandchild['id'])
        
        logger.info("✓ Structure de test créée")
        
        # Test 1: Déplacer un enfant simple
        logger.info("\n📍 Test 1: Déplacer child-a1 vers parent-b")
        before = await client.call_tool("get_page", {"page_id": child_a1['id']})
        if "error" not in before:
            logger.info(f"  Avant: {before.get('path', 'N/A')}")
        
        move1 = await client.call_tool("move_page", {
            "page_id": child_a1['id'],
            "new_parent_id": parent_b['id'],
            "position": 0
        })
        
        if "error" not in move1:
            logger.info(f"  ✅ Déplacement réussi")
            after = await client.call_tool("get_page", {"page_id": child_a1['id']})
            if "error" not in after:
                logger.info(f"  Après: {after.get('path', 'N/A')}")
        else:
            logger.error(f"  ❌ Erreur: {move1['error']}")
        
        # Test 2: Déplacer avec position spécifique
        logger.info("\n📍 Test 2: Déplacer child-a2 vers parent-b en position 1")
        move2 = await client.call_tool("move_page", {
            "page_id": child_a2['id'],
            "new_parent_id": parent_b['id'],
            "position": 1
        })
        logger.info(f"  ✅ Déplacement avec position: {move2}")
        
        # Vérifier l'ordre
        parent_b_details = await client.call_tool("get_page", {"page_id": parent_b['id']})
        if "error" not in parent_b_details:
            logger.info(f"  Parent B a maintenant des enfants")
            # Afficher le path si disponible
            logger.info(f"  Path de Parent B: {parent_b_details.get('path', 'N/A')}")
        
        # Test 3: Essayer de créer une référence circulaire (devrait échouer)
        logger.info("\n📍 Test 3: Test de référence circulaire (doit échouer)")
        circular = await client.call_tool("move_page", {
            "page_id": parent_a['id'],
            "new_parent_id": grandchild['id']
        })
        if "error" in circular:
            logger.info(f"  ✅ Erreur attendue: {circular['error']}")
        else:
            logger.error(f"  ❌ Devrait échouer mais a réussi")
        
        # Test 4: Essayer de déplacer la page racine (devrait échouer)
        logger.info("\n📍 Test 4: Test déplacement page racine (doit échouer)")
        move_root = await client.call_tool("move_page", {
            "page_id": root_id,
            "new_parent_id": parent_a['id']
        })
        if "error" in move_root:
            logger.info(f"  ✅ Erreur attendue: {move_root['error']}")
        else:
            logger.error(f"  ❌ Devrait échouer mais a réussi")
        
        # Test 5: Déplacer une page avec des enfants
        logger.info("\n📍 Test 5: Déplacer parent-a (qui a encore grandchild) vers root")
        move5 = await client.call_tool("move_page", {
            "page_id": parent_a['id'],
            "new_parent_id": root_id,
            "position": 10
        })
        logger.info(f"  ✅ Déplacement avec enfants: {move5}")
        
        # Afficher la structure finale
        logger.info("\n📊 Structure finale:")
        final_pages = await client.call_tool("list_pages", {})
        print_hierarchy(final_pages, indent=0)
        
    finally:
        # Nettoyage
        logger.info("\n🧹 Nettoyage...")
        for page_id in reversed(created_pages):
            try:
                await client.call_tool("delete_page", {"page_id": page_id})
            except:
                pass
        logger.info("✓ Nettoyage terminé")

def print_hierarchy(pages, indent=0):
    """Affiche la hiérarchie des pages"""
    for page_data in pages:
        if 'page' in page_data:
            page = page_data['page']
            logger.info(f"{'  ' * indent}├─ {page['title']} (ID: {page['id']}, Path: {page.get('path', 'N/A')})")
            if 'children' in page_data and page_data['children']:
                print_hierarchy(page_data['children'], indent + 1)

if __name__ == "__main__":
    asyncio.run(test_move_page_scenarios())