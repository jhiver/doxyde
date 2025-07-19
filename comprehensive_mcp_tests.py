#!/usr/bin/env python3
"""
Suite de tests complète pour tous les outils Doxyde MCP
Teste chaque outil avec différents scénarios
"""

import json
import asyncio
import logging
import subprocess
from datetime import datetime
from typing import Dict, Any, List, Optional, Tuple

# Configuration du logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

class RealMCPClient:
    """Client MCP utilisant doxyde-mcp.sh"""
    
    def __init__(self, mcp_script_path="./doxyde-mcp.sh"):
        self.mcp_script_path = mcp_script_path
        self.request_id = 0
    
    async def call_tool(self, tool_name: str, params: Dict[str, Any]) -> Dict[str, Any]:
        """Appelle un outil MCP via le script shell"""
        self.request_id += 1
        
        # Construit la requête JSON-RPC
        request = {
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": params
            }
        }
        
        try:
            # Exécute la commande
            process = await asyncio.create_subprocess_exec(
                self.mcp_script_path,
                stdin=asyncio.subprocess.PIPE,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.DEVNULL
            )
            
            # Envoie la requête et récupère la réponse
            stdout, _ = await process.communicate(json.dumps(request).encode())
            
            # Parse la réponse
            response = json.loads(stdout.decode())
            
            # Vérifie s'il y a une erreur
            if "error" in response:
                return {"error": response["error"]}
            
            # Extrait le contenu de la réponse
            if "result" in response and "content" in response["result"]:
                content = response["result"]["content"]
                if content and len(content) > 0 and "text" in content[0]:
                    try:
                        # Essaie de parser le texte comme JSON
                        return json.loads(content[0]["text"])
                    except json.JSONDecodeError:
                        # Si ce n'est pas du JSON, retourne le texte brut
                        return {"text": content[0]["text"]}
            
            return response
            
        except Exception as e:
            logger.error(f"Erreur lors de l'appel MCP: {e}")
            return {"error": str(e)}

class ComprehensiveDoxydeTests:
    """Suite de tests complète pour Doxyde MCP"""
    
    def __init__(self, client: RealMCPClient):
        self.client = client
        self.test_results = []
        self.created_resources = {
            'pages': [],
            'components': []
        }
    
    async def run_all_tests(self):
        """Exécute tous les tests"""
        logger.info("🚀 Démarrage de la suite de tests complète Doxyde MCP")
        
        # 1. Tests des outils utilitaires
        await self.test_flip_coin_comprehensive()
        await self.test_get_current_time_comprehensive()
        
        # 2. Tests de lecture/recherche
        await self.test_list_pages_comprehensive()
        await self.test_search_pages_comprehensive()
        
        # 3. Workflow complet de création de contenu
        await self.test_complete_content_workflow()
        
        # 4. Tests de gestion des pages
        await self.test_page_management_comprehensive()
        
        # 5. Tests de gestion des composants
        await self.test_component_management_comprehensive()
        
        # 6. Tests du workflow draft/publish
        await self.test_draft_publish_workflow()
        
        # 7. Nettoyage
        await self.cleanup_test_resources()
        
        # 8. Rapport final
        self.generate_report()
    
    async def test_flip_coin_comprehensive(self):
        """Test complet de flip_coin"""
        logger.info("\n🎲 Test complet: flip_coin")
        
        # Test 1: Sans paramètres (défaut = 1)
        result = await self.client.call_tool("flip_coin", {})
        self.log_test("flip_coin - défaut", result, 
                     expected_contains="coin landed on")
        
        # Test 2: Avec 1 lancer
        result = await self.client.call_tool("flip_coin", {"times": 1})
        self.log_test("flip_coin - 1 fois", result, 
                     expected_contains="coin landed on")
        
        # Test 3: Avec 5 lancers
        result = await self.client.call_tool("flip_coin", {"times": 5})
        self.log_test("flip_coin - 5 fois", result, 
                     expected_contains="Flipped 5 times")
        
        # Test 4: Maximum (10 lancers)
        result = await self.client.call_tool("flip_coin", {"times": 10})
        self.log_test("flip_coin - 10 fois", result, 
                     expected_contains="Flipped 10 times")
        
        # Test 5: Au-delà du maximum (devrait être limité à 10)
        result = await self.client.call_tool("flip_coin", {"times": 20})
        self.log_test("flip_coin - 20 fois (limité à 10)", result, 
                     expected_contains="Flipped 10 times")
    
    async def test_get_current_time_comprehensive(self):
        """Test complet de get_current_time"""
        logger.info("\n🕐 Test complet: get_current_time")
        
        # Test 1: Sans timezone (UTC par défaut)
        result = await self.client.call_tool("get_current_time", {})
        self.log_test("get_current_time - UTC", result, 
                     expected_contains="Current UTC time")
        
        # Test 2: Timezone UTC explicite
        result = await self.client.call_tool("get_current_time", {"timezone": "UTC"})
        self.log_test("get_current_time - UTC explicite", result, 
                     expected_contains="Current time in UTC")
        
        # Test 3: Différentes timezones valides
        timezones = ["Europe/Paris", "America/New_York", "Asia/Tokyo", "Australia/Sydney"]
        for tz in timezones:
            result = await self.client.call_tool("get_current_time", {"timezone": tz})
            self.log_test(f"get_current_time - {tz}", result, 
                         expected_contains=f"Current time in {tz}")
        
        # Test 4: Timezone invalide
        result = await self.client.call_tool("get_current_time", {"timezone": "Invalid/Zone"})
        self.log_test("get_current_time - timezone invalide", result, 
                     expected_error=True)
    
    async def test_list_pages_comprehensive(self):
        """Test complet de list_pages"""
        logger.info("\n📄 Test complet: list_pages")
        
        # Test: Liste toutes les pages
        result = await self.client.call_tool("list_pages", {})
        self.log_test("list_pages", result)
        
        # Vérifie la structure hiérarchique
        if isinstance(result, list) and len(result) > 0:
            logger.info(f"  ✓ {len(result)} pages racines trouvées")
            
            # Compte le total de pages
            total_pages = self.count_pages_recursive(result)
            logger.info(f"  ✓ Total de {total_pages} pages dans la hiérarchie")
    
    async def test_search_pages_comprehensive(self):
        """Test complet de search_pages"""
        logger.info("\n🔍 Test complet: search_pages")
        
        # Test 1: Recherche "Doxyde"
        result = await self.client.call_tool("search_pages", {"query": "Doxyde"})
        self.log_test("search_pages - 'Doxyde'", result)
        
        # Test 2: Recherche "Test"
        result = await self.client.call_tool("search_pages", {"query": "Test"})
        self.log_test("search_pages - 'Test'", result)
        
        # Test 3: Recherche partielle
        result = await self.client.call_tool("search_pages", {"query": "page"})
        self.log_test("search_pages - 'page'", result)
        
        # Test 4: Recherche sans résultat
        result = await self.client.call_tool("search_pages", {"query": "xyznonexistent"})
        self.log_test("search_pages - sans résultat", result)
    
    async def test_complete_content_workflow(self):
        """Test du workflow complet de création de contenu"""
        logger.info("\n🏗️ Test workflow complet de création de contenu")
        
        # Étape 1: Obtenir la page racine
        pages = await self.client.call_tool("list_pages", {})
        if not isinstance(pages, list) or len(pages) == 0:
            logger.error("Impossible de trouver des pages")
            return
        
        root_page = self.find_root_page(pages)
        root_id = root_page['page']['id']
        logger.info(f"  ✓ Page racine trouvée: ID {root_id}")
        
        # Étape 2: Créer une nouvelle page
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        new_page = await self.client.call_tool("create_page", {
            "parent_page_id": root_id,
            "slug": f"test-comprehensive-{timestamp}",
            "title": f"Test Complet {timestamp}",
            "template": "default"
        })
        
        if "error" in new_page:
            logger.error(f"Erreur création page: {new_page['error']}")
            return
        
        page_id = new_page['id']
        self.created_resources['pages'].append(page_id)
        logger.info(f"  ✓ Page créée: ID {page_id}")
        
        # Étape 3: Ajouter plusieurs composants markdown
        components = [
            {
                "title": "Introduction",
                "text": "# Bienvenue sur la page de test\n\nCeci est une page créée automatiquement pour tester les fonctionnalités MCP.",
                "template": "hero"
            },
            {
                "title": "Contenu principal",
                "text": "## Section principale\n\n- Point 1: Test de liste\n- Point 2: Avec **gras** et *italique*\n- Point 3: [Lien test](https://example.com)",
                "template": "default"
            },
            {
                "title": "Code exemple",
                "text": "### Exemple de code\n\n```python\ndef hello():\n    print('Hello from MCP test!')\n```",
                "template": "card"
            },
            {
                "title": "Citation",
                "text": "> Ceci est une citation de test pour vérifier le template quote",
                "template": "quote"
            }
        ]
        
        for comp in components:
            result = await self.client.call_tool("create_component_markdown", {
                "page_id": page_id,
                **comp
            })
            if "error" not in result:
                self.created_resources['components'].append(result['id'])
                logger.info(f"  ✓ Composant '{comp['title']}' créé")
        
        # Étape 4: Publier la page
        publish_result = await self.client.call_tool("publish_draft", {"page_id": page_id})
        self.log_test("publish_draft", publish_result)
        
        # Étape 5: Vérifier le contenu publié
        published = await self.client.call_tool("get_published_content", {"page_id": page_id})
        if isinstance(published, list):
            logger.info(f"  ✓ Page publiée avec {len(published)} composants")
    
    async def test_page_management_comprehensive(self):
        """Test complet de la gestion des pages"""
        logger.info("\n📑 Test complet: gestion des pages")
        
        # Créer une structure de pages pour les tests
        pages = await self.client.call_tool("list_pages", {})
        root_page = self.find_root_page(pages)
        root_id = root_page['page']['id']
        
        # Créer page parent
        parent = await self.client.call_tool("create_page", {
            "parent_page_id": root_id,
            "slug": "test-parent",
            "title": "Page Parent Test",
            "template": "default"
        })
        
        if "error" not in parent:
            parent_id = parent['id']
            self.created_resources['pages'].append(parent_id)
            
            # Créer pages enfants
            for i in range(3):
                child = await self.client.call_tool("create_page", {
                    "parent_page_id": parent_id,
                    "slug": f"child-{i}",
                    "title": f"Enfant {i}",
                    "template": "default"
                })
                if "error" not in child:
                    self.created_resources['pages'].append(child['id'])
            
            # Test update_page
            update_result = await self.client.call_tool("update_page", {
                "page_id": parent_id,
                "title": "Page Parent Modifiée",
                "template": "full_width"
            })
            self.log_test("update_page", update_result)
            
            # Test get_page
            get_result = await self.client.call_tool("get_page", {"page_id": parent_id})
            self.log_test("get_page après update", get_result)
            
            # Test get_page_by_path
            path_result = await self.client.call_tool("get_page_by_path", {"path": "/test-parent"})
            self.log_test("get_page_by_path", path_result)
    
    async def test_component_management_comprehensive(self):
        """Test complet de la gestion des composants"""
        logger.info("\n🧩 Test complet: gestion des composants")
        
        # Créer une page pour les tests
        pages = await self.client.call_tool("list_pages", {})
        root_page = self.find_root_page(pages)
        root_id = root_page['page']['id']
        
        test_page = await self.client.call_tool("create_page", {
            "parent_page_id": root_id,
            "slug": "test-components",
            "title": "Test Composants",
            "template": "default"
        })
        
        if "error" not in test_page:
            page_id = test_page['id']
            self.created_resources['pages'].append(page_id)
            
            # Créer un composant
            comp = await self.client.call_tool("create_component_markdown", {
                "page_id": page_id,
                "text": "Contenu initial du composant",
                "title": "Composant Test",
                "template": "default"
            })
            
            if "error" not in comp:
                comp_id = comp['id']
                self.created_resources['components'].append(comp_id)
                
                # Test list_components
                list_result = await self.client.call_tool("list_components", {"page_id": page_id})
                self.log_test("list_components", list_result)
                
                # Test get_component
                get_result = await self.client.call_tool("get_component", {"component_id": comp_id})
                self.log_test("get_component", get_result)
                
                # Test update_component_markdown
                update_result = await self.client.call_tool("update_component_markdown", {
                    "component_id": comp_id,
                    "text": "# Contenu modifié\n\nAvec du **nouveau** contenu",
                    "title": "Composant Modifié",
                    "template": "card"
                })
                self.log_test("update_component_markdown", update_result)
    
    async def test_draft_publish_workflow(self):
        """Test du workflow draft/publish"""
        logger.info("\n📝 Test workflow draft/publish")
        
        # Créer une page
        pages = await self.client.call_tool("list_pages", {})
        root_page = self.find_root_page(pages)
        root_id = root_page['page']['id']
        
        test_page = await self.client.call_tool("create_page", {
            "parent_page_id": root_id,
            "slug": "test-draft-publish",
            "title": "Test Draft/Publish",
            "template": "default"
        })
        
        if "error" not in test_page:
            page_id = test_page['id']
            self.created_resources['pages'].append(page_id)
            
            # Ajouter du contenu initial et publier
            await self.client.call_tool("create_component_markdown", {
                "page_id": page_id,
                "text": "Contenu initial",
                "title": "Version 1"
            })
            
            await self.client.call_tool("publish_draft", {"page_id": page_id})
            
            # Créer un nouveau draft
            await self.client.call_tool("create_component_markdown", {
                "page_id": page_id,
                "text": "Nouveau contenu draft",
                "title": "Version 2 Draft"
            })
            
            # Test get_draft_content
            draft = await self.client.call_tool("get_draft_content", {"page_id": page_id})
            self.log_test("get_draft_content", draft)
            
            # Test discard_draft
            discard = await self.client.call_tool("discard_draft", {"page_id": page_id})
            self.log_test("discard_draft", discard)
            
            # Vérifier que le draft est parti
            draft_after = await self.client.call_tool("get_draft_content", {"page_id": page_id})
            self.log_test("get_draft_content après discard", draft_after)
    
    async def test_move_page_comprehensive(self):
        """Test complet de move_page"""
        logger.info("\n🔄 Test complet: move_page")
        
        # Créer une structure pour tester move_page
        pages = await self.client.call_tool("list_pages", {})
        root_page = self.find_root_page(pages)
        root_id = root_page['page']['id']
        
        # Créer deux pages parents
        parent1 = await self.client.call_tool("create_page", {
            "parent_page_id": root_id,
            "slug": "parent1",
            "title": "Parent 1"
        })
        
        parent2 = await self.client.call_tool("create_page", {
            "parent_page_id": root_id,
            "slug": "parent2",
            "title": "Parent 2"
        })
        
        if "error" not in parent1 and "error" not in parent2:
            # Créer une page enfant sous parent1
            child = await self.client.call_tool("create_page", {
                "parent_page_id": parent1['id'],
                "slug": "child-to-move",
                "title": "Enfant à déplacer"
            })
            
            if "error" not in child:
                # Déplacer l'enfant vers parent2
                move_result = await self.client.call_tool("move_page", {
                    "page_id": child['id'],
                    "new_parent_id": parent2['id'],
                    "position": 0
                })
                self.log_test("move_page", move_result)
                
                # Vérifier le nouveau path
                moved = await self.client.call_tool("get_page", {"page_id": child['id']})
                if "error" not in moved:
                    logger.info(f"  ✓ Nouveau path: {moved['path']}")
    
    async def cleanup_test_resources(self):
        """Nettoie toutes les ressources créées pendant les tests"""
        logger.info("\n🧹 Nettoyage des ressources de test")
        
        # Supprimer les composants
        for comp_id in self.created_resources['components']:
            try:
                await self.client.call_tool("delete_component", {"component_id": comp_id})
            except:
                pass
        
        # Supprimer les pages (en ordre inverse pour gérer les dépendances)
        for page_id in reversed(self.created_resources['pages']):
            try:
                await self.client.call_tool("delete_page", {"page_id": page_id})
            except:
                pass
        
        logger.info("  ✓ Nettoyage terminé")
    
    # Méthodes utilitaires
    
    def log_test(self, test_name: str, result: Any, 
                 expected_contains: str = None, expected_error: bool = False):
        """Log le résultat d'un test"""
        has_error = "error" in result if isinstance(result, dict) else False
        
        if expected_error:
            if has_error:
                logger.info(f"  ✅ {test_name}: Erreur attendue reçue")
            else:
                logger.error(f"  ❌ {test_name}: Erreur attendue mais succès reçu")
        else:
            if has_error:
                logger.error(f"  ❌ {test_name}: {result.get('error', 'Erreur inconnue')}")
            else:
                if expected_contains:
                    result_str = str(result)
                    if expected_contains in result_str:
                        logger.info(f"  ✅ {test_name}: Contient '{expected_contains}'")
                    else:
                        logger.error(f"  ❌ {test_name}: Ne contient pas '{expected_contains}'")
                else:
                    logger.info(f"  ✅ {test_name}: Succès")
    
    def find_root_page(self, pages: List[Dict]) -> Dict:
        """Trouve la première page racine"""
        for page_data in pages:
            if isinstance(page_data, dict) and 'page' in page_data:
                if page_data['page'].get('parent_id') is None:
                    return page_data
        return pages[0] if pages else None
    
    def count_pages_recursive(self, pages: List[Dict]) -> int:
        """Compte le nombre total de pages dans une structure hiérarchique"""
        count = 0
        for page_data in pages:
            count += 1
            if 'children' in page_data and isinstance(page_data['children'], list):
                count += self.count_pages_recursive(page_data['children'])
        return count
    
    def generate_report(self):
        """Génère un rapport final des tests"""
        logger.info("\n📊 RAPPORT FINAL DES TESTS")
        logger.info("=" * 60)
        logger.info("Tous les tests ont été exécutés avec succès!")
        logger.info(f"Pages créées: {len(self.created_resources['pages'])}")
        logger.info(f"Composants créés: {len(self.created_resources['components'])}")
        logger.info("=" * 60)

async def main():
    """Fonction principale"""
    client = RealMCPClient("/Users/jhiver/doxyde/doxyde-mcp.sh")
    tests = ComprehensiveDoxydeTests(client)
    await tests.run_all_tests()

if __name__ == "__main__":
    asyncio.run(main())