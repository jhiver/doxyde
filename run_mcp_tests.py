#!/usr/bin/env python3
"""
Suite de tests pour les outils Doxyde MCP
Tests tous les outils disponibles de manière systématique
"""

import json
import asyncio
import logging
from datetime import datetime
from typing import Dict, Any, List, Optional

# Configuration du logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(levelname)s - %(message)s'
)
logger = logging.getLogger(__name__)

class DoxydeTestSuite:
    """Suite de tests pour les outils Doxyde MCP"""
    
    def __init__(self, mcp_client):
        self.client = mcp_client
        self.test_results = []
        self.test_data = {
            'created_pages': [],
            'created_components': []
        }
    
    async def run_all_tests(self):
        """Execute tous les tests dans l'ordre approprié"""
        logger.info("🚀 Démarrage de la suite de tests Doxyde")
        
        # Tests des outils utilitaires
        await self.test_flip_coin()
        await self.test_get_current_time()
        
        # Tests de lecture des pages
        await self.test_list_pages()
        await self.test_get_page()
        await self.test_get_page_by_path()
        await self.test_search_pages()
        
        # Tests de gestion du contenu
        await self.test_get_published_content()
        await self.test_get_draft_content()
        
        # Tests CRUD des pages
        await self.test_create_page()
        await self.test_update_page()
        await self.test_move_page()
        
        # Tests des composants
        await self.test_create_component_markdown()
        await self.test_list_components()
        await self.test_get_component()
        await self.test_update_component_markdown()
        
        # Tests de publication
        await self.test_publish_draft()
        await self.test_discard_draft()
        
        # Nettoyage
        await self.cleanup_test_data()
        
        # Rapport final
        self.generate_report()
    
    async def test_tool(self, tool_name: str, params: Dict[str, Any], expected_success: bool = True):
        """Test générique pour un outil MCP"""
        test_start = datetime.now()
        
        try:
            logger.info(f"🧪 Test: {tool_name} avec {params}")
            
            # Mesure du temps d'exécution
            start_time = datetime.now()
            result = await self.client.call_tool(tool_name, params)
            end_time = datetime.now()
            
            execution_time = (end_time - start_time).total_seconds()
            
            # Validation de la réponse
            success = self.validate_response(result, expected_success)
            
            test_result = {
                'tool': tool_name,
                'params': params,
                'success': success,
                'execution_time': execution_time,
                'response_size': len(json.dumps(result)) if result else 0,
                'timestamp': test_start.isoformat(),
                'error': None if success else str(result)
            }
            
            self.test_results.append(test_result)
            
            if success:
                logger.info(f"✅ {tool_name} - OK ({execution_time:.2f}s)")
            else:
                logger.error(f"❌ {tool_name} - FAILED: {result}")
                
            return result
            
        except Exception as e:
            logger.error(f"💥 {tool_name} - EXCEPTION: {e}")
            self.test_results.append({
                'tool': tool_name,
                'params': params,
                'success': False,
                'execution_time': None,
                'response_size': 0,
                'timestamp': test_start.isoformat(),
                'error': str(e)
            })
            return None
    
    def validate_response(self, response: Any, expected_success: bool) -> bool:
        """Valide qu'une réponse est correcte"""
        if response is None:
            return not expected_success
        
        # Vérifie si c'est une erreur
        if isinstance(response, dict) and 'error' in response:
            return not expected_success
        
        # Vérifie la taille de la réponse (max 1MB)
        try:
            response_size = len(json.dumps(response))
            if response_size > 1024 * 1024:
                logger.warning(f"⚠️ Réponse très volumineuse: {response_size} bytes")
                return False
        except (TypeError, ValueError):
            logger.warning("⚠️ Réponse non sérialisable en JSON")
            return False
        
        return expected_success
    
    # Tests des outils utilitaires
    async def test_flip_coin(self):
        """Test de l'outil flip_coin"""
        await self.test_tool("doxyde:flip_coin", {})
        await self.test_tool("doxyde:flip_coin", {"times": 5})
        await self.test_tool("doxyde:flip_coin", {"times": 10})
        await self.test_tool("doxyde:flip_coin", {"times": 15}, False)  # Devrait échouer (max 10)
    
    async def test_get_current_time(self):
        """Test de l'outil get_current_time"""
        await self.test_tool("doxyde:get_current_time", {})
        await self.test_tool("doxyde:get_current_time", {"timezone": "UTC"})
        await self.test_tool("doxyde:get_current_time", {"timezone": "Europe/Paris"})
        await self.test_tool("doxyde:get_current_time", {"timezone": "America/New_York"})
        await self.test_tool("doxyde:get_current_time", {"timezone": "Invalid/Timezone"}, False)
    
    # Tests de lecture des pages
    async def test_list_pages(self):
        """Test de l'outil list_pages"""
        result = await self.test_tool("doxyde:list_pages", {})
        
        # Debug: affiche la structure des données retournées
        if result:
            logger.info(f"🔍 Structure de list_pages: {type(result)}")
            logger.info(f"🔍 Contenu: {json.dumps(result, indent=2)[:500]}...")
        
        return result
    
    def extract_page_ids(self, pages_data):
        """Extrait les IDs de pages de la structure retournée par list_pages"""
        page_ids = []
        
        if not pages_data:
            return page_ids
        
        try:
            # Si c'est une liste
            if isinstance(pages_data, list):
                for item in pages_data:
                    if isinstance(item, dict):
                        # Structure avec 'page' et 'children'
                        if 'page' in item and 'id' in item['page']:
                            page_ids.append(item['page']['id'])
                        # Structure directe
                        elif 'id' in item:
                            page_ids.append(item['id'])
                        
                        # Récursif pour les enfants
                        if 'children' in item and isinstance(item['children'], list):
                            for child in item['children']:
                                if 'page' in child and 'id' in child['page']:
                                    page_ids.append(child['page']['id'])
            
            # Si c'est un dictionnaire
            elif isinstance(pages_data, dict):
                if 'page' in pages_data and 'id' in pages_data['page']:
                    page_ids.append(pages_data['page']['id'])
                elif 'id' in pages_data:
                    page_ids.append(pages_data['id'])
        
        except Exception as e:
            logger.error(f"Erreur lors de l'extraction des IDs: {e}")
        
        logger.info(f"🔍 IDs de pages extraits: {page_ids}")
        return page_ids
    
    async def test_get_page(self):
        """Test de l'outil get_page"""
        # Test avec une page existante (basé sur list_pages)
        pages = await self.test_list_pages()
        page_ids = self.extract_page_ids(pages)
        
        if page_ids:
            page_id = page_ids[0]
            logger.info(f"🔍 Test get_page avec page_id: {page_id}")
            await self.test_tool("doxyde:get_page", {"page_id": page_id})
        else:
            logger.warning("⚠️ Aucun ID de page trouvé pour test_get_page")
        
        # Test avec une page inexistante
        await self.test_tool("doxyde:get_page", {"page_id": 99999}, False)
    
    async def test_get_page_by_path(self):
        """Test de l'outil get_page_by_path"""
        await self.test_tool("doxyde:get_page_by_path", {"path": "/"})
        await self.test_tool("doxyde:get_page_by_path", {"path": "/about"})
        await self.test_tool("doxyde:get_page_by_path", {"path": "/inexistant"}, False)
    
    async def test_search_pages(self):
        """Test de l'outil search_pages"""
        await self.test_tool("doxyde:search_pages", {"query": "Doxyde"})
        await self.test_tool("doxyde:search_pages", {"query": "About"})
        await self.test_tool("doxyde:search_pages", {"query": "inexistant"})
        await self.test_tool("doxyde:search_pages", {"query": ""}, False)
    
    # Tests de contenu
    async def test_get_published_content(self):
        """Test de l'outil get_published_content"""
        pages = await self.test_list_pages()
        page_ids = self.extract_page_ids(pages)
        
        if page_ids:
            page_id = page_ids[0]
            await self.test_tool("doxyde:get_published_content", {"page_id": page_id})
        else:
            logger.warning("⚠️ Aucun ID de page pour test_get_published_content")
        
        await self.test_tool("doxyde:get_published_content", {"page_id": 99999}, False)
    
    async def test_get_draft_content(self):
        """Test de l'outil get_draft_content"""
        pages = await self.test_list_pages()
        page_ids = self.extract_page_ids(pages)
        
        if page_ids:
            page_id = page_ids[0]
            await self.test_tool("doxyde:get_draft_content", {"page_id": page_id})
        else:
            logger.warning("⚠️ Aucun ID de page pour test_get_draft_content")
        
        await self.test_tool("doxyde:get_draft_content", {"page_id": 99999}, False)
    
    # Tests CRUD des pages
    async def test_create_page(self):
        """Test de l'outil create_page"""
        # Récupère l'ID de la page racine
        pages = await self.test_list_pages()
        page_ids = self.extract_page_ids(pages)
        
        if page_ids:
            parent_id = page_ids[0]
            
            # Création d'une page de test
            timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
            result = await self.test_tool("doxyde:create_page", {
                "parent_page_id": parent_id,
                "slug": f"test-page-{timestamp}",
                "title": f"Page de Test {timestamp}",
                "template": "default"
            })
            
            if result and isinstance(result, dict):
                # Essaie différentes structures possibles
                created_id = None
                if 'page' in result and 'id' in result['page']:
                    created_id = result['page']['id']
                elif 'id' in result:
                    created_id = result['id']
                
                if created_id:
                    self.test_data['created_pages'].append(created_id)
                    logger.info(f"✅ Page créée avec ID: {created_id}")
        else:
            logger.warning("⚠️ Aucun parent ID pour test_create_page")
        
        # Test avec paramètres invalides
        await self.test_tool("doxyde:create_page", {
            "parent_page_id": 99999,
            "slug": "invalid-parent",
            "title": "Test Invalid"
        }, False)
    
    async def test_update_page(self):
        """Test de l'outil update_page"""
        if self.test_data['created_pages']:
            page_id = self.test_data['created_pages'][0]
            await self.test_tool("doxyde:update_page", {
                "page_id": page_id,
                "title": "Titre Modifié",
                "template": "full_width"
            })
        
        await self.test_tool("doxyde:update_page", {
            "page_id": 99999,
            "title": "Inexistant"
        }, False)
    
    async def test_move_page(self):
        """Test de l'outil move_page"""
        if len(self.test_data['created_pages']) >= 1:
            page_id = self.test_data['created_pages'][0]
            pages = await self.test_list_pages()
            page_ids = self.extract_page_ids(pages)
            
            if page_ids:
                new_parent_id = page_ids[0]
                await self.test_tool("doxyde:move_page", {
                    "page_id": page_id,
                    "new_parent_id": new_parent_id,
                    "position": 0
                })
            else:
                logger.warning("⚠️ Aucun parent ID pour test_move_page")
        
        # Test avec IDs invalides
        await self.test_tool("doxyde:move_page", {
            "page_id": 99999,
            "new_parent_id": 1
        }, False)
    
    # Tests des composants
    async def test_create_component_markdown(self):
        """Test de l'outil create_component_markdown"""
        if self.test_data['created_pages']:
            page_id = self.test_data['created_pages'][0]
            result = await self.test_tool("doxyde:create_component_markdown", {
                "page_id": page_id,
                "text": "# Composant de Test\n\nCeci est un composant markdown de test.",
                "title": "Composant Test",
                "template": "default"
            })
            
            if result and isinstance(result, dict):
                # Essaie différentes structures possibles
                created_id = None
                if 'component' in result and 'id' in result['component']:
                    created_id = result['component']['id']
                elif 'id' in result:
                    created_id = result['id']
                
                if created_id:
                    self.test_data['created_components'].append(created_id)
                    logger.info(f"✅ Composant créé avec ID: {created_id}")
        else:
            logger.warning("⚠️ Aucune page créée pour test_create_component_markdown")
        
        await self.test_tool("doxyde:create_component_markdown", {
            "page_id": 99999,
            "text": "Test"
        }, False)
    
    async def test_list_components(self):
        """Test de l'outil list_components"""
        if self.test_data['created_pages']:
            page_id = self.test_data['created_pages'][0]
            await self.test_tool("doxyde:list_components", {"page_id": page_id})
        
        await self.test_tool("doxyde:list_components", {"page_id": 99999}, False)
    
    async def test_get_component(self):
        """Test de l'outil get_component"""
        if self.test_data['created_components']:
            component_id = self.test_data['created_components'][0]
            await self.test_tool("doxyde:get_component", {"component_id": component_id})
        
        await self.test_tool("doxyde:get_component", {"component_id": 99999}, False)
    
    async def test_update_component_markdown(self):
        """Test de l'outil update_component_markdown"""
        if self.test_data['created_components']:
            component_id = self.test_data['created_components'][0]
            await self.test_tool("doxyde:update_component_markdown", {
                "component_id": component_id,
                "text": "# Composant Modifié\n\nContenu mis à jour.",
                "title": "Titre Modifié"
            })
        
        await self.test_tool("doxyde:update_component_markdown", {
            "component_id": 99999,
            "text": "Test"
        }, False)
    
    # Tests de publication
    async def test_publish_draft(self):
        """Test de l'outil publish_draft"""
        if self.test_data['created_pages']:
            page_id = self.test_data['created_pages'][0]
            await self.test_tool("doxyde:publish_draft", {"page_id": page_id})
        
        await self.test_tool("doxyde:publish_draft", {"page_id": 99999}, False)
    
    async def test_discard_draft(self):
        """Test de l'outil discard_draft"""
        if self.test_data['created_pages']:
            page_id = self.test_data['created_pages'][0]
            await self.test_tool("doxyde:discard_draft", {"page_id": page_id})
        
        await self.test_tool("doxyde:discard_draft", {"page_id": 99999}, False)
    
    # Nettoyage
    async def cleanup_test_data(self):
        """Nettoie les données de test créées"""
        logger.info("🧹 Nettoyage des données de test")
        
        # Supprime les composants de test
        for component_id in self.test_data['created_components']:
            await self.test_tool("doxyde:delete_component", {"component_id": component_id})
        
        # Supprime les pages de test
        for page_id in self.test_data['created_pages']:
            await self.test_tool("doxyde:delete_page", {"page_id": page_id})
    
    def generate_report(self):
        """Génère un rapport de test"""
        total_tests = len(self.test_results)
        successful_tests = sum(1 for t in self.test_results if t['success'])
        failed_tests = total_tests - successful_tests
        
        avg_execution_time = sum(
            t['execution_time'] for t in self.test_results 
            if t['execution_time'] is not None
        ) / max(1, len([t for t in self.test_results if t['execution_time'] is not None]))
        
        logger.info("📊 RAPPORT DE TESTS DOXYDE")
        logger.info("=" * 50)
        logger.info(f"Tests exécutés: {total_tests}")
        logger.info(f"Succès: {successful_tests} ✅")
        logger.info(f"Échecs: {failed_tests} ❌")
        logger.info(f"Taux de réussite: {successful_tests/total_tests*100:.1f}%")
        logger.info(f"Temps d'exécution moyen: {avg_execution_time:.2f}s")
        
        # Tests qui ont échoué
        if failed_tests > 0:
            logger.info("\n❌ TESTS ÉCHOUÉS:")
            for test in self.test_results:
                if not test['success']:
                    logger.info(f"  - {test['tool']}: {test['error']}")
        
        # Tests les plus lents
        slow_tests = sorted(
            [t for t in self.test_results if t['execution_time'] is not None],
            key=lambda x: x['execution_time'],
            reverse=True
        )[:5]
        
        if slow_tests:
            logger.info("\n🐌 TESTS LES PLUS LENTS:")
            for test in slow_tests:
                logger.info(f"  - {test['tool']}: {test['execution_time']:.2f}s")
        
        return {
            'total': total_tests,
            'success': successful_tests,
            'failed': failed_tests,
            'success_rate': successful_tests/total_tests*100,
            'avg_time': avg_execution_time,
            'results': self.test_results
        }

import subprocess
import json

class RealMCPClient:
    """Client MCP utilisant doxyde-mcp.sh"""
    
    def __init__(self, mcp_script_path="./doxyde-mcp.sh"):
        self.mcp_script_path = mcp_script_path
        self.request_id = 0
    
    async def call_tool(self, tool_name, params):
        """Appelle un outil MCP via le script shell"""
        self.request_id += 1
        
        # Enlève le préfixe "doxyde:" si présent
        if tool_name.startswith("doxyde:"):
            tool_name = tool_name[7:]
        
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
                return response["error"]
            
            # Extrait le contenu de la réponse
            if "result" in response and "content" in response["result"]:
                content = response["result"]["content"]
                if content and len(content) > 0 and "text" in content[0]:
                    try:
                        # Essaie de parser le texte comme JSON
                        return json.loads(content[0]["text"])
                    except json.JSONDecodeError:
                        # Si ce n'est pas du JSON, retourne le texte brut
                        return content[0]["text"]
            
            return response
            
        except Exception as e:
            logger.error(f"Erreur lors de l'appel MCP: {e}")
            return {"error": str(e)}

# Exemple d'utilisation
async def main():
    """Fonction principale pour exécuter les tests"""
    client = RealMCPClient("/Users/jhiver/doxyde/doxyde-mcp.sh")
    test_suite = DoxydeTestSuite(client)
    
    await test_suite.run_all_tests()

if __name__ == "__main__":
    asyncio.run(main())
