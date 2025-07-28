// Doxyde - A modern, AI-native CMS built with Rust
// Copyright (C) 2025 Doxyde Project Contributors
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use rmcp::{
    handler::server::{router::tool::ToolRouter, tool::Parameters, ServerHandler},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_router, tool_handler,
    schemars::{self, JsonSchema},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::SqlitePool;
use tracing::info;
use anyhow::{Context, Result};
use std::future::Future;

#[derive(Debug, Clone)]
pub struct DoxydeRmcpService {
    pool: SqlitePool,
    site_id: i64,
    tool_router: ToolRouter<Self>,
}

impl DoxydeRmcpService {
    pub fn new(pool: SqlitePool, site_id: i64) -> Self {
        let router = Self::tool_router();
        info!("Created DoxydeRmcpService with tool_router for site_id={}", site_id);
        Self {
            pool,
            site_id,
            tool_router: router,
        }
    }
}

// Request structures
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetPageRequest {
    #[schemars(description = "The page ID")]
    pub page_id: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetPageByPathRequest {
    #[schemars(description = "The URL path to search for")]
    pub path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchPagesRequest {
    #[schemars(description = "Search query")]
    pub query: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetPublishedContentRequest {
    #[schemars(description = "The page ID")]
    pub page_id: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetDraftContentRequest {
    #[schemars(description = "The page ID")]
    pub page_id: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetOrCreateDraftRequest {
    #[schemars(description = "The page ID to get or create draft for")]
    pub page_id: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateComponentMarkdownRequest {
    #[schemars(description = "ID of the page to add the component to")]
    pub page_id: i64,
    
    #[schemars(description = "Position in the component list (0-based). If not provided, component is added at the end.")]
    pub position: Option<i32>,
    
    #[schemars(description = "Component template (default, card, highlight, quote)")]
    pub template: Option<String>,
    
    #[schemars(description = "Optional component title")]
    pub title: Option<String>,
    
    #[schemars(description = "Markdown content of the component")]
    pub content: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateComponentMarkdownRequest {
    #[schemars(description = "ID of the component to update")]
    pub component_id: i64,
    
    #[schemars(description = "New markdown content (optional)")]
    pub content: Option<String>,
    
    #[schemars(description = "New component title (optional)")]
    pub title: Option<String>,
    
    #[schemars(description = "New template (optional)")]
    pub template: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PublishDraftRequest {
    #[schemars(description = "ID of the page whose draft to publish")]
    pub page_id: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreatePageRequest {
    #[schemars(description = "ID of the parent page (required - root pages cannot be created)")]
    pub parent_page_id: Option<i64>,
    
    #[schemars(description = "Optional URL-friendly page identifier. If not provided, will be auto-generated from title")]
    pub slug: Option<String>,
    
    #[schemars(description = "Page title")]
    pub title: String,
    
    #[schemars(description = "Page description/summary for SEO (recommended 150-160 characters). This appears in search results.")]
    pub description: Option<String>,
    
    #[schemars(description = "Comma-separated keywords for SEO (e.g., 'cms, content management, rust')")]
    pub keywords: Option<String>,
    
    #[schemars(description = "Page template (default, full_width, landing, blog)")]
    pub template: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdatePageRequest {
    #[schemars(description = "ID of the page to update")]
    pub page_id: i64,
    
    #[schemars(description = "New URL-friendly page identifier (optional). Will update the page path.")]
    pub slug: Option<String>,
    
    #[schemars(description = "New page title (optional)")]
    pub title: Option<String>,
    
    #[schemars(description = "New page description for SEO (optional)")]
    pub description: Option<String>,
    
    #[schemars(description = "New comma-separated keywords for SEO (optional)")]
    pub keywords: Option<String>,
    
    #[schemars(description = "New page template (optional)")]
    pub template: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteComponentRequest {
    #[schemars(description = "ID of the component to delete")]
    pub component_id: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DiscardDraftRequest {
    #[schemars(description = "ID of the page whose draft to discard")]
    pub page_id: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListComponentsRequest {
    #[schemars(description = "ID of the page to list components for")]
    pub page_id: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetComponentRequest {
    #[schemars(description = "ID of the component to retrieve")]
    pub component_id: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeletePageRequest {
    #[schemars(description = "ID of the page to delete")]
    pub page_id: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MovePageRequest {
    #[schemars(description = "ID of the page to move")]
    pub page_id: i64,
    
    #[schemars(description = "ID of the new parent page (null for root level)")]
    pub new_parent_id: Option<i64>,
    
    #[schemars(description = "Position within the new parent (0-based). If not provided, page is added at the end.")]
    pub position: Option<i32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MoveComponentBeforeRequest {
    #[schemars(description = "ID of the component to move")]
    pub component_id: i64,
    
    #[schemars(description = "ID of the component to move before")]
    pub before_component_id: i64,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MoveComponentAfterRequest {
    #[schemars(description = "ID of the component to move")]
    pub component_id: i64,
    
    #[schemars(description = "ID of the component to move after")]
    pub after_component_id: i64,
}

#[tool_router]
impl DoxydeRmcpService {
    #[tool(description = "Get all pages in the site with hierarchy")]
    pub async fn list_pages(&self) -> String {
        match self.internal_list_pages().await {
            Ok(pages) => serde_json::to_string_pretty(&pages).unwrap_or_else(|e| {
                format!("{{\"error\": \"Failed to serialize pages: {}\"}}", e)
            }),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }

    #[tool(description = "Get full page details by ID")]
    pub async fn get_page(&self, Parameters(req): Parameters<GetPageRequest>) -> String {
        match self.internal_get_page(req.page_id).await {
            Ok(page) => serde_json::to_string_pretty(&page).unwrap_or_else(|e| {
                format!("{{\"error\": \"Failed to serialize page: {}\"}}", e)
            }),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }

    #[tool(description = "Find page by URL path (e.g., '/about/team')")]
    pub async fn get_page_by_path(&self, Parameters(req): Parameters<GetPageByPathRequest>) -> String {
        match self.internal_get_page_by_path(&req.path).await {
            Ok(page) => serde_json::to_string_pretty(&page).unwrap_or_else(|e| {
                format!("{{\"error\": \"Failed to serialize page: {}\"}}", e)
            }),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }

    #[tool(description = "Search pages by title or content")]
    pub async fn search_pages(&self, Parameters(req): Parameters<SearchPagesRequest>) -> String {
        match self.internal_search_pages(&req.query).await {
            Ok(pages) => serde_json::to_string_pretty(&pages).unwrap_or_else(|e| {
                format!("{{\"error\": \"Failed to serialize search results: {}\"}}", e)
            }),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }

    #[tool(description = "Get published content of a page")]
    pub async fn get_published_content(&self, Parameters(req): Parameters<GetPublishedContentRequest>) -> String {
        match self.internal_get_published_content(req.page_id).await {
            Ok(components) => serde_json::to_string_pretty(&components).unwrap_or_else(|e| {
                format!("{{\"error\": \"Failed to serialize components: {}\"}}", e)
            }),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }

    #[tool(description = "Get draft content of a page (if exists)")]
    pub async fn get_draft_content(&self, Parameters(req): Parameters<GetDraftContentRequest>) -> String {
        match self.internal_get_draft_content(req.page_id).await {
            Ok(Some(components)) => serde_json::to_string_pretty(&components).unwrap_or_else(|e| {
                format!("{{\"error\": \"Failed to serialize components: {}\"}}", e)
            }),
            Ok(None) => "null".to_string(), // No draft exists
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }

    #[tool(description = "Get existing draft or create a new one for a page. This is the starting point for editing page content. Returns draft version info and all components in the draft.")]
    pub async fn get_or_create_draft(&self, Parameters(req): Parameters<GetOrCreateDraftRequest>) -> String {
        match self.internal_get_or_create_draft(req.page_id).await {
            Ok(result) => serde_json::to_string_pretty(&result).unwrap_or_else(|e| {
                format!("{{\"error\": \"Failed to serialize result: {}\"}}", e)
            }),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }

    #[tool(description = "Create a new markdown component in a page's draft version. Automatically creates a draft if none exists.")]
    pub async fn create_component_markdown(&self, Parameters(req): Parameters<CreateComponentMarkdownRequest>) -> String {
        match self.internal_create_component_markdown(req).await {
            Ok(component_info) => serde_json::to_string_pretty(&component_info).unwrap_or_else(|e| {
                format!("{{\"error\": \"Failed to serialize component: {}\"}}", e)
            }),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }

    #[tool(description = "Update the content, title, or template of a markdown component. Component must be in a draft version.")]
    pub async fn update_component_markdown(&self, Parameters(req): Parameters<UpdateComponentMarkdownRequest>) -> String {
        match self.internal_update_component_markdown(req).await {
            Ok(component_info) => serde_json::to_string_pretty(&component_info).unwrap_or_else(|e| {
                format!("{{\"error\": \"Failed to serialize component: {}\"}}", e)
            }),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }

    #[tool(description = "Publish the draft version of a page, making it the live version")]
    pub async fn publish_draft(&self, Parameters(req): Parameters<PublishDraftRequest>) -> String {
        match self.internal_publish_draft(req.page_id).await {
            Ok(draft_info) => {
                format!(
                    "Successfully published draft for page {}. Version {} is now live.",
                    draft_info.page_id,
                    draft_info.version_number
                )
            },
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }

    #[tool(description = "Create a new page with metadata for SEO. Always provide meaningful description and relevant keywords for better search engine visibility.")]
    pub async fn create_page(&self, Parameters(req): Parameters<CreatePageRequest>) -> String {
        match self.internal_create_page(req).await {
            Ok(page_info) => serde_json::to_string_pretty(&page_info).unwrap_or_else(|e| {
                format!("{{\"error\": \"Failed to serialize page info: {}\"}}", e)
            }),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }

    #[tool(description = "Update page metadata including slug, title, SEO fields, and template. Only provided fields will be updated.")]
    pub async fn update_page(&self, Parameters(req): Parameters<UpdatePageRequest>) -> String {
        match self.internal_update_page(req).await {
            Ok(page_info) => serde_json::to_string_pretty(&page_info).unwrap_or_else(|e| {
                format!("{{\"error\": \"Failed to serialize page info: {}\"}}", e)
            }),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }

    #[tool(description = "Delete a component from a draft version. This operation cannot be undone.")]
    pub async fn delete_component(&self, Parameters(req): Parameters<DeleteComponentRequest>) -> String {
        match self.internal_delete_component(req.component_id).await {
            Ok(_) => format!("Successfully deleted component {}", req.component_id),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }

    #[tool(description = "Discard the draft version of a page, reverting to the published version")]
    pub async fn discard_draft(&self, Parameters(req): Parameters<DiscardDraftRequest>) -> String {
        match self.internal_discard_draft(req.page_id).await {
            Ok(_) => format!("Successfully discarded draft for page {}", req.page_id),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }

    #[tool(description = "List all components for a page")]
    pub async fn list_components(&self, Parameters(req): Parameters<ListComponentsRequest>) -> String {
        match self.internal_list_components(req.page_id).await {
            Ok(components) => serde_json::to_string_pretty(&components).unwrap_or_else(|e| {
                format!("{{\"error\": \"Failed to serialize components: {}\"}}", e)
            }),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }

    #[tool(description = "Get details of a specific component")]
    pub async fn get_component(&self, Parameters(req): Parameters<GetComponentRequest>) -> String {
        match self.internal_get_component(req.component_id).await {
            Ok(component) => serde_json::to_string_pretty(&component).unwrap_or_else(|e| {
                format!("{{\"error\": \"Failed to serialize component: {}\"}}", e)
            }),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }

    #[tool(description = "Delete a page. The page must not have any children. To delete a page with children, first delete or move all child pages.")]
    pub async fn delete_page(&self, Parameters(req): Parameters<DeletePageRequest>) -> String {
        match self.internal_delete_page(req.page_id).await {
            Ok(_) => format!("Successfully deleted page {}", req.page_id),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }

    #[tool(description = "Move a page to a different parent or reorder within the same parent. Cannot create circular references.")]
    pub async fn move_page(&self, Parameters(req): Parameters<MovePageRequest>) -> String {
        match self.internal_move_page(req).await {
            Ok(page_info) => serde_json::to_string_pretty(&page_info).unwrap_or_else(|e| {
                format!("{{\"error\": \"Failed to serialize page info: {}\"}}", e)
            }),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }

    #[tool(description = "Move a component before another component in the same draft version")]
    pub async fn move_component_before(&self, Parameters(req): Parameters<MoveComponentBeforeRequest>) -> String {
        match self.internal_move_component_before(req).await {
            Ok(component_info) => serde_json::to_string_pretty(&component_info).unwrap_or_else(|e| {
                format!("{{\"error\": \"Failed to serialize component: {}\"}}", e)
            }),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }

    #[tool(description = "Move a component after another component in the same draft version")]
    pub async fn move_component_after(&self, Parameters(req): Parameters<MoveComponentAfterRequest>) -> String {
        match self.internal_move_component_after(req).await {
            Ok(component_info) => serde_json::to_string_pretty(&component_info).unwrap_or_else(|e| {
                format!("{{\"error\": \"Failed to serialize component: {}\"}}", e)
            }),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }
}

#[tool_handler]
impl ServerHandler for DoxydeRmcpService {
    fn get_info(&self) -> ServerInfo {
        info!("Getting server info");
        ServerInfo {
            protocol_version: rmcp::model::ProtocolVersion::default(),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
            server_info: rmcp::model::Implementation {
                name: "doxyde-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some("Doxyde CMS MCP integration for AI-native content management".to_string()),
        }
    }
}

// Helper data structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageInfo {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub path: String,
    pub parent_id: Option<i64>,
    pub position: i32,
    pub has_children: bool,
    pub template: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PageHierarchy {
    pub page: PageInfo,
    pub children: Vec<PageHierarchy>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComponentInfo {
    pub id: i64,
    pub component_type: String,
    pub position: i32,
    pub template: String,
    pub title: Option<String>,
    pub content: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DraftInfo {
    pub page_id: i64,
    pub version_id: i64,
    pub version_number: i32,
    pub created_by: Option<String>,
    pub is_published: bool,
    pub component_count: i32,
}

// Internal implementation methods
impl DoxydeRmcpService {
    async fn internal_list_pages(&self) -> Result<Vec<PageHierarchy>> {
        use doxyde_db::repositories::PageRepository;

        let page_repo = PageRepository::new(self.pool.clone());
        let pages = page_repo.list_by_site_id(self.site_id).await?;

        // Build hierarchy
        let mut hierarchy = Vec::new();
        let mut page_map = std::collections::HashMap::new();

        // First pass: create PageInfo for all pages
        for page in &pages {
            let info = self.page_to_info(&pages, page).await?;
            page_map.insert(page.id.unwrap(), (info, Vec::new()));
        }

        // Second pass: build hierarchy
        let mut root_pages = Vec::new();
        let ids: Vec<i64> = page_map.keys().copied().collect();

        for id in ids {
            if let Some((info, _)) = page_map.get(&id) {
                if let Some(parent_id) = info.parent_id {
                    if let Some((_, children)) = page_map.get_mut(&parent_id) {
                        children.push(id);
                    }
                } else {
                    root_pages.push(id);
                }
            }
        }

        // Build final hierarchy
        for root_id in root_pages {
            if let Some(node) = self.build_hierarchy_node(&page_map, root_id) {
                hierarchy.push(node);
            }
        }

        Ok(hierarchy)
    }

    async fn internal_get_page(&self, page_id: i64) -> Result<doxyde_core::models::Page> {
        use doxyde_db::repositories::PageRepository;

        let page_repo = PageRepository::new(self.pool.clone());
        let page = page_repo
            .find_by_id(page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page not found"))?;

        // Verify page belongs to this site
        if page.site_id != self.site_id {
            return Err(anyhow::anyhow!("Page does not belong to this site"));
        }

        Ok(page)
    }

    async fn internal_get_page_by_path(&self, path: &str) -> Result<doxyde_core::models::Page> {
        use doxyde_db::repositories::PageRepository;

        let page_repo = PageRepository::new(self.pool.clone());
        let all_pages = page_repo.list_by_site_id(self.site_id).await?;

        // Handle root path
        if path == "/" || path.is_empty() {
            return all_pages
                .into_iter()
                .find(|p| p.parent_page_id.is_none())
                .ok_or_else(|| anyhow::anyhow!("Root page not found"));
        }

        // Normalize path (remove leading/trailing slashes)
        let normalized_path = path.trim_matches('/');
        let _path_parts: Vec<&str> = normalized_path.split('/').collect();

        // Find page by matching the constructed path
        for page in &all_pages {
            let page_path = self.build_page_path(&all_pages, page).await?;
            if page_path.trim_matches('/') == normalized_path {
                return Ok(page.clone());
            }
        }

        Err(anyhow::anyhow!("Page not found at path: {}", path))
    }

    async fn internal_search_pages(&self, query: &str) -> Result<Vec<PageInfo>> {
        use doxyde_db::repositories::{ComponentRepository, PageRepository, PageVersionRepository};

        let page_repo = PageRepository::new(self.pool.clone());
        let version_repo = PageVersionRepository::new(self.pool.clone());
        let component_repo = ComponentRepository::new(self.pool.clone());

        let all_pages = page_repo.list_by_site_id(self.site_id).await?;
        let query_lower = query.to_lowercase();

        let mut results = Vec::new();

        for page in &all_pages {
            let mut matches = false;

            // Search in page title
            if page.title.to_lowercase().contains(&query_lower) {
                matches = true;
            }

            // Search in page slug
            if !matches && page.slug.to_lowercase().contains(&query_lower) {
                matches = true;
            }

            // Search in page description
            if !matches {
                if let Some(desc) = &page.description {
                    if desc.to_lowercase().contains(&query_lower) {
                        matches = true;
                    }
                }
            }

            // Search in page keywords
            if !matches {
                if let Some(keywords) = &page.keywords {
                    if keywords.to_lowercase().contains(&query_lower) {
                        matches = true;
                    }
                }
            }

            // Search in page content (published version)
            if !matches {
                if let Some(page_id) = page.id {
                    if let Ok(Some(version)) = version_repo.get_published(page_id).await {
                        if let Some(version_id) = version.id {
                            if let Ok(components) = component_repo.list_by_page_version(version_id).await {
                                for component in components {
                                    // Search in component title
                                    if let Some(title) = &component.title {
                                        if title.to_lowercase().contains(&query_lower) {
                                            matches = true;
                                            break;
                                        }
                                    }

                                    // Search in component content
                                    if let Some(content_str) = component.content.as_str() {
                                        if content_str.to_lowercase().contains(&query_lower) {
                                            matches = true;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if matches {
                let info = self.page_to_info(&all_pages, page).await?;
                results.push(info);
            }
        }

        // Sort results by title
        results.sort_by(|a, b| a.title.cmp(&b.title));

        Ok(results)
    }

    async fn internal_get_published_content(&self, page_id: i64) -> Result<Vec<ComponentInfo>> {
        use doxyde_db::repositories::{ComponentRepository, PageRepository, PageVersionRepository};

        // Verify page exists and belongs to this site
        let page_repo = PageRepository::new(self.pool.clone());
        let page = page_repo
            .find_by_id(page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page not found"))?;

        if page.site_id != self.site_id {
            return Err(anyhow::anyhow!("Page does not belong to this site"));
        }

        // Get published version
        let version_repo = PageVersionRepository::new(self.pool.clone());
        let version = version_repo
            .get_published(page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("No published version exists for this page"))?;

        // Get components
        let component_repo = ComponentRepository::new(self.pool.clone());
        let components = component_repo
            .list_by_page_version(version.id.unwrap())
            .await?;

        // Convert to ComponentInfo
        let component_infos = components
            .into_iter()
            .map(|c| self.component_to_info(c))
            .collect();

        Ok(component_infos)
    }

    async fn internal_get_draft_content(&self, page_id: i64) -> Result<Option<Vec<ComponentInfo>>> {
        use doxyde_db::repositories::{ComponentRepository, PageRepository, PageVersionRepository};

        // Verify page exists and belongs to this site
        let page_repo = PageRepository::new(self.pool.clone());
        let page = page_repo
            .find_by_id(page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page not found"))?;

        if page.site_id != self.site_id {
            return Err(anyhow::anyhow!("Page does not belong to this site"));
        }

        // Get draft version
        let version_repo = PageVersionRepository::new(self.pool.clone());
        let version = match version_repo.get_draft(page_id).await? {
            Some(v) => v,
            None => return Ok(None), // No draft exists
        };

        // Get components
        let component_repo = ComponentRepository::new(self.pool.clone());
        let components = component_repo
            .list_by_page_version(version.id.unwrap())
            .await?;

        // Convert to ComponentInfo
        let component_infos = components
            .into_iter()
            .map(|c| self.component_to_info(c))
            .collect();

        Ok(Some(component_infos))
    }

    async fn internal_get_or_create_draft(&self, page_id: i64) -> Result<serde_json::Value> {
        use doxyde_db::repositories::{ComponentRepository, PageRepository, PageVersionRepository};

        // Verify page exists and belongs to this site
        let page_repo = PageRepository::new(self.pool.clone());
        let page = page_repo
            .find_by_id(page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page not found"))?;

        if page.site_id != self.site_id {
            return Err(anyhow::anyhow!("Page does not belong to this site"));
        }

        let version_repo = PageVersionRepository::new(self.pool.clone());
        let component_repo = ComponentRepository::new(self.pool.clone());

        // Check if draft already exists
        let (draft, is_new) = if let Some(existing_draft) = version_repo.get_draft(page_id).await? {
            (existing_draft, false)
        } else {
            // Create new draft
            // First, check if there's a published version to copy from
            if let Some(published) = version_repo.get_published(page_id).await? {
                // Copy components from published version
                let published_components = component_repo
                    .list_by_page_version(published.id.unwrap())
                    .await?;

                // Create new draft version
                let new_version = doxyde_core::models::PageVersion::new(
                    page_id, 
                    published.version_number + 1, 
                    None
                );
                let new_draft_id = version_repo.create(&new_version).await?;

                // Copy components to new draft
                for component in published_components {
                    let mut new_component = doxyde_core::models::Component::new(
                        new_draft_id,
                        component.component_type.clone(),
                        component.position,
                        component.content.clone(),
                    );
                    new_component.template = component.template.clone();
                    new_component.title = component.title.clone();
                    component_repo.create(&new_component).await?;
                }

                // Get the created draft
                let draft = version_repo.find_by_id(new_draft_id).await?
                    .ok_or_else(|| anyhow::anyhow!("Failed to retrieve created draft"))?;
                (draft, true)
            } else {
                // No published version, create version 1
                let new_version = doxyde_core::models::PageVersion::new(page_id, 1, None);
                let version_id = version_repo.create(&new_version).await?;
                let draft = version_repo.find_by_id(version_id).await?
                    .ok_or_else(|| anyhow::anyhow!("Failed to retrieve created draft"))?;
                (draft, true)
            }
        };

        // Get all components in the draft
        let components = component_repo
            .list_by_page_version(draft.id.unwrap())
            .await?;

        let component_infos: Vec<ComponentInfo> = components
            .into_iter()
            .map(|c| self.component_to_info(c))
            .collect();

        // Build response

        Ok(json!({
            "draft": {
                "version_id": draft.id.unwrap(),
                "version_number": draft.version_number,
                "is_published": draft.is_published,
                "is_new": is_new,
                "created_by": draft.created_by,
                "created_at": draft.created_at.to_rfc3339(),
            },
            "page": {
                "id": page.id.unwrap(),
                "title": page.title,
                "slug": page.slug,
                "template": page.template,
            },
            "components": component_infos,
            "component_count": component_infos.len(),
        }))
    }

    async fn internal_create_component_markdown(&self, req: CreateComponentMarkdownRequest) -> Result<ComponentInfo> {
        use doxyde_db::repositories::{ComponentRepository, PageRepository};

        // Verify page exists and belongs to this site
        let page_repo = PageRepository::new(self.pool.clone());
        let page = page_repo
            .find_by_id(req.page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page not found"))?;

        if page.site_id != self.site_id {
            return Err(anyhow::anyhow!("Page does not belong to this site"));
        }

        // Get or create draft version
        let draft_result = self.internal_get_or_create_draft(req.page_id).await?;
        let draft_info = draft_result.as_object()
            .ok_or_else(|| anyhow::anyhow!("Invalid draft response format"))?;
        let draft = draft_info.get("draft")
            .ok_or_else(|| anyhow::anyhow!("Missing draft info"))?;
        let version_id = draft.get("version_id")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| anyhow::anyhow!("Missing or invalid version_id"))?;

        // Determine position
        let component_repo = ComponentRepository::new(self.pool.clone());
        let existing_components = component_repo
            .list_by_page_version(version_id)
            .await?;

        let target_position = req.position
            .unwrap_or(existing_components.len() as i32)
            .clamp(0, existing_components.len() as i32);

        // Shift existing components if needed
        if target_position < existing_components.len() as i32 {
            for mut comp in existing_components {
                if comp.position >= target_position {
                    comp.position += 1;
                    component_repo.update(&comp).await?;
                }
            }
        }

        // Create component data
        let component_data = serde_json::json!({
            "text": req.content
        });

        // Create the component
        let mut new_component = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            target_position,
            component_data,
        );
        new_component.template = req.template.unwrap_or_else(|| "default".to_string());
        new_component.title = req.title;

        let component_id = component_repo.create(&new_component).await?;

        // Get the created component
        let created_component = component_repo
            .find_by_id(component_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve created component"))?;

        Ok(self.component_to_info(created_component))
    }

    async fn internal_update_component_markdown(&self, req: UpdateComponentMarkdownRequest) -> Result<ComponentInfo> {
        use doxyde_db::repositories::{ComponentRepository, PageRepository, PageVersionRepository};
        
        let component_repo = ComponentRepository::new(self.pool.clone());
        
        // Get the component
        let mut component = component_repo
            .find_by_id(req.component_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Component not found"))?;
        
        // Verify it's a markdown component
        if component.component_type != "markdown" {
            return Err(anyhow::anyhow!(
                "Component is not a markdown component (type: {})",
                component.component_type
            ));
        }
        
        // Get the page version to verify it's a draft
        let version_repo = PageVersionRepository::new(self.pool.clone());
        let version = version_repo
            .find_by_id(component.page_version_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page version not found"))?;
        
        if version.is_published {
            return Err(anyhow::anyhow!(
                "Cannot update component in published version. Create or edit a draft first."
            ));
        }
        
        // Verify the page belongs to this site
        let page_repo = PageRepository::new(self.pool.clone());
        let page = page_repo
            .find_by_id(version.page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page not found"))?;
        
        if page.site_id != self.site_id {
            return Err(anyhow::anyhow!("Component belongs to a page in a different site"));
        }
        
        // Track if anything changed
        let mut changed = false;
        
        // Update content if provided
        if let Some(new_content) = req.content {
            if let Some(text) = component.content.get("text").and_then(|v| v.as_str()) {
                if text != new_content {
                    component.content = serde_json::json!({
                        "text": new_content
                    });
                    changed = true;
                }
            }
        }
        
        // Update title if provided
        if let Some(new_title) = req.title {
            if component.title.as_ref() != Some(&new_title) {
                component.title = Some(new_title);
                changed = true;
            }
        }
        
        // Update template if provided
        if let Some(new_template) = req.template {
            if component.template != new_template {
                component.template = new_template;
                changed = true;
            }
        }
        
        // Only update if something changed
        if changed {
            component.updated_at = chrono::Utc::now();
            component_repo.update(&component).await?;
        }
        
        // Get updated component
        let updated_component = component_repo
            .find_by_id(req.component_id)
            .await?
            .unwrap();
        
        Ok(self.component_to_info(updated_component))
    }

    async fn internal_publish_draft(&self, page_id: i64) -> Result<DraftInfo> {
        use doxyde_db::repositories::{PageRepository, PageVersionRepository};

        // Verify page exists and belongs to this site
        let page_repo = PageRepository::new(self.pool.clone());
        let page = page_repo
            .find_by_id(page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page not found"))?;

        if page.site_id != self.site_id {
            return Err(anyhow::anyhow!("Page does not belong to this site"));
        }

        let version_repo = PageVersionRepository::new(self.pool.clone());

        // Get the draft version
        let draft = version_repo
            .get_draft(page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!(
                "No draft version exists for this page. Use get_or_create_draft first."
            ))?;

        // Unpublish current published version if exists
        if let Some(current_published) = version_repo.get_published(page_id).await? {
            // Manually unpublish by updating is_published field
            sqlx::query(
                r#"
                UPDATE page_versions
                SET is_published = 0
                WHERE id = ?
                "#,
            )
            .bind(current_published.id.unwrap())
            .execute(&self.pool)
            .await
            .context("Failed to unpublish current version")?;
        }

        // Publish the draft
        version_repo.publish(draft.id.unwrap()).await?;

        // Get component count for the draft
        use doxyde_db::repositories::ComponentRepository;
        let component_repo = ComponentRepository::new(self.pool.clone());
        let components = component_repo
            .list_by_page_version(draft.id.unwrap())
            .await?;

        Ok(DraftInfo {
            page_id,
            version_id: draft.id.unwrap(),
            version_number: draft.version_number,
            created_by: draft.created_by,
            is_published: true,
            component_count: components.len() as i32,
        })
    }

    async fn internal_create_page(&self, req: CreatePageRequest) -> Result<PageInfo> {
        use doxyde_db::repositories::{PageRepository, PageVersionRepository};
        
        let page_repo = PageRepository::new(self.pool.clone());
        
        // Verify parent page exists and belongs to this site (if provided)
        if let Some(parent_id) = req.parent_page_id {
            let parent = page_repo
                .find_by_id(parent_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Parent page not found"))?;
            
            if parent.site_id != self.site_id {
                return Err(anyhow::anyhow!("Parent page does not belong to this site"));
            }
        } else {
            // Check if root page already exists
            let existing_pages = page_repo.list_by_site_id(self.site_id).await?;
            if existing_pages.iter().any(|p| p.parent_page_id.is_none()) {
                return Err(anyhow::anyhow!("Root page already exists. New pages must have a parent."));
            }
        }
        
        // Generate slug if not provided
        let slug = req.slug.unwrap_or_else(|| {
            req.title
                .to_lowercase()
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '-' })
                .collect::<String>()
                .split('-')
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("-")
        });
        
        // Validate slug uniqueness within parent
        let siblings = if let Some(parent_id) = req.parent_page_id {
            page_repo.list_children(parent_id).await?
        } else {
            page_repo.list_by_site_id(self.site_id).await?
                .into_iter()
                .filter(|p| p.parent_page_id.is_none())
                .collect()
        };
        
        if siblings.iter().any(|p| p.slug == slug) {
            return Err(anyhow::anyhow!("A page with slug '{}' already exists at this level", slug));
        }
        
        // Determine position (at the end)
        let position = siblings.len() as i32;
        
        // Create the page
        let template = req.template.unwrap_or_else(|| "default".to_string());
        
        let new_page = doxyde_core::models::Page {
            id: None,
            site_id: self.site_id,
            parent_page_id: req.parent_page_id,
            slug: slug.clone(),
            title: req.title.clone(),
            template: template.clone(),
            position,
            description: req.description,
            keywords: req.keywords,
            meta_robots: "index, follow".to_string(),
            canonical_url: None,
            og_image_url: None,
            structured_data_type: "Article".to_string(),
            sort_mode: "position".to_string(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        
        let page_id = page_repo.create(&new_page).await?;
        
        // Create initial empty version
        let version_repo = PageVersionRepository::new(self.pool.clone());
        let version = doxyde_core::models::PageVersion::new(page_id, 1, None);
        version_repo.create(&version).await?;
        
        // Get all pages to build path
        let all_pages = page_repo.list_by_site_id(self.site_id).await?;
        let created_page = page_repo.find_by_id(page_id).await?.unwrap();
        
        Ok(self.page_to_info(&all_pages, &created_page).await?)
    }

    async fn internal_update_page(&self, req: UpdatePageRequest) -> Result<PageInfo> {
        use doxyde_db::repositories::PageRepository;
        
        let page_repo = PageRepository::new(self.pool.clone());
        
        // Get the page
        let mut page = page_repo
            .find_by_id(req.page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page not found"))?;
        
        if page.site_id != self.site_id {
            return Err(anyhow::anyhow!("Page does not belong to this site"));
        }
        
        // Track if anything changed
        let mut changed = false;
        
        // Update slug if provided
        if let Some(new_slug) = req.slug {
            if new_slug != page.slug {
                // Validate slug uniqueness within same parent
                let siblings = if let Some(parent_id) = page.parent_page_id {
                    page_repo.list_children(parent_id).await?
                } else {
                    page_repo.list_by_site_id(self.site_id).await?
                        .into_iter()
                        .filter(|p| p.parent_page_id.is_none())
                        .collect()
                };
                
                if siblings.iter().any(|p| p.slug == new_slug && p.id != page.id) {
                    return Err(anyhow::anyhow!("A page with slug '{}' already exists at this level", new_slug));
                }
                
                page.slug = new_slug;
                changed = true;
            }
        }
        
        // Update title if provided
        if let Some(new_title) = req.title {
            if new_title != page.title {
                page.title = new_title;
                changed = true;
            }
        }
        
        // Update description if provided
        if let Some(new_description) = req.description {
            if page.description.as_ref() != Some(&new_description) {
                page.description = Some(new_description);
                changed = true;
            }
        }
        
        // Update keywords if provided
        if let Some(new_keywords) = req.keywords {
            if page.keywords.as_ref() != Some(&new_keywords) {
                page.keywords = Some(new_keywords);
                changed = true;
            }
        }
        
        // Update template if provided
        if let Some(new_template) = req.template {
            if new_template != page.template {
                page.template = new_template;
                changed = true;
            }
        }
        
        // Only update if something changed
        if changed {
            page.updated_at = chrono::Utc::now();
            page_repo.update(&page).await?;
        }
        
        // Get all pages to build path
        let all_pages = page_repo.list_by_site_id(self.site_id).await?;
        let updated_page = page_repo.find_by_id(req.page_id).await?.unwrap();
        
        Ok(self.page_to_info(&all_pages, &updated_page).await?)
    }

    async fn internal_delete_component(&self, component_id: i64) -> Result<()> {
        use doxyde_db::repositories::{ComponentRepository, PageRepository, PageVersionRepository};
        
        let component_repo = ComponentRepository::new(self.pool.clone());
        
        // Get the component
        let component = component_repo
            .find_by_id(component_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Component not found"))?;
        
        // Get the page version to verify it's a draft
        let version_repo = PageVersionRepository::new(self.pool.clone());
        let version = version_repo
            .find_by_id(component.page_version_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page version not found"))?;
        
        if version.is_published {
            return Err(anyhow::anyhow!(
                "Cannot delete component from published version. Create or edit a draft first."
            ));
        }
        
        // Verify the page belongs to this site
        let page_repo = PageRepository::new(self.pool.clone());
        let page = page_repo
            .find_by_id(version.page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page not found"))?;
        
        if page.site_id != self.site_id {
            return Err(anyhow::anyhow!("Component belongs to a page in a different site"));
        }
        
        let deleted_position = component.position;
        
        // Delete the component
        component_repo.delete(component_id).await?;
        
        // Update positions of remaining components
        let mut remaining_components = component_repo
            .list_by_page_version(component.page_version_id)
            .await?;
        
        remaining_components.sort_by_key(|c| c.position);
        
        // Shift positions down for components after the deleted one
        for mut comp in remaining_components {
            if comp.position > deleted_position {
                comp.position -= 1;
                component_repo.update(&comp).await?;
            }
        }
        
        Ok(())
    }

    async fn internal_discard_draft(&self, page_id: i64) -> Result<()> {
        use doxyde_db::repositories::{PageRepository, PageVersionRepository};

        // Verify page exists and belongs to this site
        let page_repo = PageRepository::new(self.pool.clone());
        let page = page_repo
            .find_by_id(page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page not found"))?;

        if page.site_id != self.site_id {
            return Err(anyhow::anyhow!("Page does not belong to this site"));
        }

        let version_repo = PageVersionRepository::new(self.pool.clone());

        // Get the draft version
        let draft = version_repo
            .get_draft(page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!(
                "No draft version exists for this page. Drafts are created automatically when you start editing."
            ))?;

        // Delete the draft version
        version_repo.delete_draft(draft.id.unwrap()).await?;

        Ok(())
    }

    async fn internal_list_components(&self, page_id: i64) -> Result<Vec<ComponentInfo>> {
        use doxyde_db::repositories::{ComponentRepository, PageRepository, PageVersionRepository};

        // Verify page exists and belongs to this site
        let page_repo = PageRepository::new(self.pool.clone());
        let page = page_repo
            .find_by_id(page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page not found"))?;

        if page.site_id != self.site_id {
            return Err(anyhow::anyhow!("Page does not belong to this site"));
        }

        let version_repo = PageVersionRepository::new(self.pool.clone());
        let component_repo = ComponentRepository::new(self.pool.clone());

        // Try to get draft version first
        let version = if let Some(draft) = version_repo.get_draft(page_id).await? {
            draft
        } else if let Some(published) = version_repo.get_published(page_id).await? {
            published
        } else {
            // No versions exist
            return Ok(Vec::new());
        };

        // Get components
        let components = component_repo
            .list_by_page_version(version.id.unwrap())
            .await?;

        // Convert to ComponentInfo
        let component_infos = components
            .into_iter()
            .map(|c| self.component_to_info(c))
            .collect();

        Ok(component_infos)
    }

    async fn internal_get_component(&self, component_id: i64) -> Result<ComponentInfo> {
        use doxyde_db::repositories::{ComponentRepository, PageRepository, PageVersionRepository};

        let component_repo = ComponentRepository::new(self.pool.clone());

        // Get the component
        let component = component_repo
            .find_by_id(component_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Component not found"))?;

        // Get the page version to find the page
        let version_repo = PageVersionRepository::new(self.pool.clone());
        let version = version_repo
            .find_by_id(component.page_version_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page version not found"))?;

        // Verify the page belongs to this site
        let page_repo = PageRepository::new(self.pool.clone());
        let page = page_repo
            .find_by_id(version.page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page not found"))?;

        if page.site_id != self.site_id {
            return Err(anyhow::anyhow!("Component belongs to a page in a different site"));
        }

        Ok(self.component_to_info(component))
    }

    async fn internal_delete_page(&self, page_id: i64) -> Result<()> {
        use doxyde_db::repositories::PageRepository;
        
        let page_repo = PageRepository::new(self.pool.clone());
        
        // Verify page exists and belongs to this site
        let page = page_repo
            .find_by_id(page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page not found"))?;
        
        if page.site_id != self.site_id {
            return Err(anyhow::anyhow!("Page does not belong to this site"));
        }
        
        // Store parent_id before deletion
        let parent_id = page.parent_page_id;
        
        // The repository's delete method will handle:
        // - Checking if it's a root page
        // - Checking if it has children
        // - Deleting the page and its versions/components
        page_repo.delete(page_id).await?;
        
        // Update positions of remaining siblings
        if let Some(parent_id) = parent_id {
            let mut siblings = page_repo.list_children(parent_id).await?;
            siblings.sort_by_key(|p| p.position);
            
            for (idx, mut sibling) in siblings.into_iter().enumerate() {
                if sibling.position != idx as i32 {
                    sibling.position = idx as i32;
                    page_repo.update(&sibling).await?;
                }
            }
        }
        
        Ok(())
    }

    async fn internal_move_page(&self, req: MovePageRequest) -> Result<PageInfo> {
        use doxyde_db::repositories::PageRepository;
        
        let page_repo = PageRepository::new(self.pool.clone());
        
        // Get the page to move
        let mut page = page_repo
            .find_by_id(req.page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page not found"))?;
        
        if page.site_id != self.site_id {
            return Err(anyhow::anyhow!("Page does not belong to this site"));
        }
        
        // Cannot move the root page
        if page.parent_page_id.is_none() && req.new_parent_id.is_none() {
            return Err(anyhow::anyhow!("Root page is already at the root level"));
        }
        
        if page.parent_page_id.is_none() && req.new_parent_id.is_some() {
            return Err(anyhow::anyhow!("Cannot move the root page under another page"));
        }
        
        // Verify new parent exists and belongs to same site
        if let Some(new_parent_id) = req.new_parent_id {
            let new_parent = page_repo
                .find_by_id(new_parent_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("New parent page not found"))?;
            
            if new_parent.site_id != self.site_id {
                return Err(anyhow::anyhow!("New parent page does not belong to this site"));
            }
            
            // Check for circular reference
            if self.would_create_circular_reference(req.page_id, new_parent_id).await? {
                return Err(anyhow::anyhow!("Cannot move page under its own descendant"));
            }
        }
        
        let old_parent_id = page.parent_page_id;
        let _old_position = page.position;
        
        // Get siblings at destination
        let new_siblings = if let Some(parent_id) = req.new_parent_id {
            page_repo.list_children(parent_id).await?
        } else {
            page_repo.list_by_site_id(self.site_id).await?
                .into_iter()
                .filter(|p| p.parent_page_id.is_none())
                .collect()
        };
        
        // Filter out the page being moved if it's already in this parent
        let mut new_siblings: Vec<_> = new_siblings
            .into_iter()
            .filter(|p| p.id != Some(req.page_id))
            .collect();
        new_siblings.sort_by_key(|p| p.position);
        
        // Determine target position
        let target_position = req.position.unwrap_or(new_siblings.len() as i32);
        let target_position = target_position.clamp(0, new_siblings.len() as i32);
        
        // Update the page
        page.parent_page_id = req.new_parent_id;
        page.position = target_position;
        page.updated_at = chrono::Utc::now();
        page_repo.update(&page).await?;
        
        // Update positions at old location (if changed parent)
        if old_parent_id != req.new_parent_id {
            let mut old_siblings = if let Some(parent_id) = old_parent_id {
                page_repo.list_children(parent_id).await?
            } else {
                page_repo.list_by_site_id(self.site_id).await?
                    .into_iter()
                    .filter(|p| p.parent_page_id.is_none())
                    .collect()
            };
            old_siblings.sort_by_key(|p| p.position);
            
            for (idx, mut sibling) in old_siblings.into_iter().enumerate() {
                if sibling.position != idx as i32 {
                    sibling.position = idx as i32;
                    page_repo.update(&sibling).await?;
                }
            }
        }
        
        // Update positions at new location
        let mut all_siblings = if let Some(parent_id) = req.new_parent_id {
            page_repo.list_children(parent_id).await?
        } else {
            page_repo.list_by_site_id(self.site_id).await?
                .into_iter()
                .filter(|p| p.parent_page_id.is_none())
                .collect()
        };
        all_siblings.sort_by_key(|p| {
            if p.id == Some(req.page_id) {
                target_position
            } else if p.position >= target_position {
                p.position + 1
            } else {
                p.position
            }
        });
        
        for (idx, mut sibling) in all_siblings.into_iter().enumerate() {
            if sibling.position != idx as i32 {
                sibling.position = idx as i32;
                page_repo.update(&sibling).await?;
            }
        }
        
        // Get updated page info
        let all_pages = page_repo.list_by_site_id(self.site_id).await?;
        let moved_page = page_repo.find_by_id(req.page_id).await?.unwrap();
        
        Ok(self.page_to_info(&all_pages, &moved_page).await?)
    }
    
    async fn would_create_circular_reference(&self, page_id: i64, new_parent_id: i64) -> Result<bool> {
        use doxyde_db::repositories::PageRepository;
        
        if page_id == new_parent_id {
            return Ok(true);
        }
        
        let page_repo = PageRepository::new(self.pool.clone());
        let all_pages = page_repo.list_by_site_id(self.site_id).await?;
        
        // Check if new_parent_id is a descendant of page_id
        let mut current_id = Some(new_parent_id);
        
        while let Some(id) = current_id {
            if id == page_id {
                return Ok(true);
            }
            
            current_id = all_pages
                .iter()
                .find(|p| p.id == Some(id))
                .and_then(|p| p.parent_page_id);
        }
        
        Ok(false)
    }

    async fn internal_move_component_before(&self, req: MoveComponentBeforeRequest) -> Result<ComponentInfo> {
        use doxyde_db::repositories::{ComponentRepository, PageRepository, PageVersionRepository};
        
        if req.component_id == req.before_component_id {
            return Err(anyhow::anyhow!("Cannot move component before itself"));
        }
        
        let component_repo = ComponentRepository::new(self.pool.clone());
        
        // Get both components
        let component = component_repo
            .find_by_id(req.component_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Component to move not found"))?;
        
        let before_component = component_repo
            .find_by_id(req.before_component_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Target component not found"))?;
        
        // Verify they're in the same page version
        if component.page_version_id != before_component.page_version_id {
            return Err(anyhow::anyhow!("Components must be in the same page version"));
        }
        
        // Verify it's a draft version
        let version_repo = PageVersionRepository::new(self.pool.clone());
        let version = version_repo
            .find_by_id(component.page_version_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page version not found"))?;
        
        if version.is_published {
            return Err(anyhow::anyhow!(
                "Cannot move components in published version. Create or edit a draft first."
            ));
        }
        
        // Verify the page belongs to this site
        let page_repo = PageRepository::new(self.pool.clone());
        let page = page_repo
            .find_by_id(version.page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page not found"))?;
        
        if page.site_id != self.site_id {
            return Err(anyhow::anyhow!("Components belong to a page in a different site"));
        }
        
        // Get all components in the version
        let mut all_components = component_repo
            .list_by_page_version(component.page_version_id)
            .await?;
        
        all_components.sort_by_key(|c| c.position);
        
        let old_position = component.position;
        let new_position = before_component.position;
        
        // If already in correct position, no-op
        if old_position + 1 == new_position {
            return Ok(self.component_to_info(component));
        }
        
        // Reorder components
        if old_position < new_position {
            // Moving down: shift components between old and new position up
            for mut comp in all_components {
                if comp.position > old_position && comp.position < new_position {
                    comp.position -= 1;
                    component_repo.update(&comp).await?;
                } else if comp.id == Some(req.component_id) {
                    comp.position = new_position - 1;
                    component_repo.update(&comp).await?;
                }
            }
        } else {
            // Moving up: shift components between new and old position down
            for mut comp in all_components {
                if comp.position >= new_position && comp.position < old_position {
                    comp.position += 1;
                    component_repo.update(&comp).await?;
                } else if comp.id == Some(req.component_id) {
                    comp.position = new_position;
                    component_repo.update(&comp).await?;
                }
            }
        }
        
        // Get updated component
        let updated_component = component_repo
            .find_by_id(req.component_id)
            .await?
            .unwrap();
        
        Ok(self.component_to_info(updated_component))
    }

    async fn internal_move_component_after(&self, req: MoveComponentAfterRequest) -> Result<ComponentInfo> {
        use doxyde_db::repositories::{ComponentRepository, PageRepository, PageVersionRepository};
        
        if req.component_id == req.after_component_id {
            return Err(anyhow::anyhow!("Cannot move component after itself"));
        }
        
        let component_repo = ComponentRepository::new(self.pool.clone());
        
        // Get both components
        let component = component_repo
            .find_by_id(req.component_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Component to move not found"))?;
        
        let after_component = component_repo
            .find_by_id(req.after_component_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Target component not found"))?;
        
        // Verify they're in the same page version
        if component.page_version_id != after_component.page_version_id {
            return Err(anyhow::anyhow!("Components must be in the same page version"));
        }
        
        // Verify it's a draft version
        let version_repo = PageVersionRepository::new(self.pool.clone());
        let version = version_repo
            .find_by_id(component.page_version_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page version not found"))?;
        
        if version.is_published {
            return Err(anyhow::anyhow!(
                "Cannot move components in published version. Create or edit a draft first."
            ));
        }
        
        // Verify the page belongs to this site
        let page_repo = PageRepository::new(self.pool.clone());
        let page = page_repo
            .find_by_id(version.page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page not found"))?;
        
        if page.site_id != self.site_id {
            return Err(anyhow::anyhow!("Components belong to a page in a different site"));
        }
        
        // Get all components in the version
        let mut all_components = component_repo
            .list_by_page_version(component.page_version_id)
            .await?;
        
        all_components.sort_by_key(|c| c.position);
        
        let old_position = component.position;
        let new_position = after_component.position;
        
        // If already in correct position, no-op
        if old_position == new_position + 1 {
            return Ok(self.component_to_info(component));
        }
        
        // Reorder components
        if old_position < new_position {
            // Moving down: shift components between old and new position up
            for mut comp in all_components {
                if comp.position > old_position && comp.position <= new_position {
                    comp.position -= 1;
                    component_repo.update(&comp).await?;
                } else if comp.id == Some(req.component_id) {
                    comp.position = new_position;
                    component_repo.update(&comp).await?;
                }
            }
        } else {
            // Moving up: shift components between new and old position down
            for mut comp in all_components {
                if comp.position > new_position && comp.position < old_position {
                    comp.position += 1;
                    component_repo.update(&comp).await?;
                } else if comp.id == Some(req.component_id) {
                    comp.position = new_position + 1;
                    component_repo.update(&comp).await?;
                }
            }
        }
        
        // Get updated component
        let updated_component = component_repo
            .find_by_id(req.component_id)
            .await?
            .unwrap();
        
        Ok(self.component_to_info(updated_component))
    }

    fn component_to_info(&self, component: doxyde_core::models::Component) -> ComponentInfo {
        ComponentInfo {
            id: component.id.unwrap(),
            component_type: component.component_type,
            position: component.position,
            template: component.template,
            title: component.title,
            content: component.content,
            created_at: component.created_at.to_rfc3339(),
            updated_at: component.updated_at.to_rfc3339(),
        }
    }

    async fn page_to_info(&self, all_pages: &[doxyde_core::models::Page], page: &doxyde_core::models::Page) -> Result<PageInfo> {
        let has_children = all_pages.iter().any(|p| p.parent_page_id == page.id);

        Ok(PageInfo {
            id: page.id.unwrap(),
            slug: page.slug.clone(),
            title: page.title.clone(),
            path: self.build_page_path(all_pages, page).await?,
            parent_id: page.parent_page_id,
            position: page.position,
            has_children,
            template: Some(page.template.clone()),
        })
    }

    async fn build_page_path(&self, all_pages: &[doxyde_core::models::Page], page: &doxyde_core::models::Page) -> Result<String> {
        // Special case for root page
        if page.parent_page_id.is_none() {
            return Ok("/".to_string());
        }

        let mut path_parts = vec![page.slug.clone()];
        let mut current_parent = page.parent_page_id;

        while let Some(parent_id) = current_parent {
            if let Some(parent) = all_pages.iter().find(|p| p.id == Some(parent_id)) {
                // Don't include root page slug in path
                if parent.parent_page_id.is_some() {
                    path_parts.push(parent.slug.clone());
                }
                current_parent = parent.parent_page_id;
            } else {
                break;
            }
        }

        path_parts.reverse();
        Ok(format!("/{}", path_parts.join("/")))
    }

    fn build_hierarchy_node(
        &self,
        page_map: &std::collections::HashMap<i64, (PageInfo, Vec<i64>)>,
        page_id: i64,
    ) -> Option<PageHierarchy> {
        if let Some((info, child_ids)) = page_map.get(&page_id) {
            let mut children = Vec::new();

            // Sort children by position
            let mut sorted_child_ids = child_ids.clone();
            sorted_child_ids.sort_by_key(|&id| {
                page_map.get(&id).map(|(info, _)| info.position).unwrap_or(0)
            });

            for child_id in sorted_child_ids {
                if let Some(child_node) = self.build_hierarchy_node(page_map, child_id) {
                    children.push(child_node);
                }
            }

            Some(PageHierarchy {
                page: info.clone(),
                children,
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;
    use anyhow::Context;

    async fn setup_test_db(pool: &SqlitePool) -> Result<()> {
        // Run migrations
        sqlx::migrate!("../migrations")
            .run(pool)
            .await
            .context("Failed to run migrations")?;
        Ok(())
    }

    #[sqlx::test]
    async fn test_service_creation(pool: SqlitePool) -> Result<()> {
        let service = DoxydeRmcpService::new(pool, 1);
        let info = service.get_info();
        assert_eq!(info.server_info.name, "doxyde-mcp");
        Ok(())
    }

    #[sqlx::test]
    async fn test_server_info_protocol_compliance(pool: SqlitePool) -> Result<()> {
        let service = DoxydeRmcpService::new(pool, 1);
        let info = service.get_info();

        // Verify protocol version is set
        assert!(!info.protocol_version.to_string().is_empty());

        // Verify capabilities
        assert!(info.capabilities.tools.is_some());

        // Verify server info
        assert_eq!(info.server_info.name, "doxyde-mcp");
        assert!(!info.server_info.version.is_empty());

        // Verify instructions are set
        assert!(info.instructions.is_some());
        assert!(info.instructions.unwrap().contains("Doxyde CMS"));

        Ok(())
    }

    #[sqlx::test]
    async fn test_get_page_tool(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository};
        
        // Create test site
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        // Get the root page (created automatically)
        let page_repo = PageRepository::new(pool.clone());
        let root_page = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create test page as child of root
        let mut page = doxyde_core::models::Page::new(
            site_id,
            "test-page".to_string(),
            "Test Page".to_string(),
        );
        page.parent_page_id = root_page.id;
        page.template = "default".to_string();
        let page_id = page_repo.create(&page).await?;
        
        let service = DoxydeRmcpService::new(pool, site_id);
        let req = GetPageRequest { page_id };
        let result = service.get_page(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        let returned_page: doxyde_core::models::Page = serde_json::from_str(&result)?;
        assert_eq!(returned_page.id.unwrap(), page_id);
        assert_eq!(returned_page.title, "Test Page");
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_page_not_found(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        let service = DoxydeRmcpService::new(pool, 1);
        
        let req = GetPageRequest { page_id: 99999 };
        let result = service.get_page(Parameters(req)).await;
        
        assert!(result.contains("error"));
        // The error could be either from SQLx or from our code
        assert!(result.contains("Page not found") || result.contains("Failed to find page"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_page_wrong_site(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository};
        
        // Create two sites
        let site_repo = SiteRepository::new(pool.clone());
        let site1 = doxyde_core::models::Site::new("site1.com".to_string(), "Site 1".to_string());
        let site1_id = site_repo.create(&site1).await?;
        let site2 = doxyde_core::models::Site::new("site2.com".to_string(), "Site 2".to_string());
        let site2_id = site_repo.create(&site2).await?;
        
        // Get root page of site2
        let page_repo = PageRepository::new(pool.clone());
        let root2 = page_repo.get_root_page(site2_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create page in site2
        let mut page = doxyde_core::models::Page::new(
            site2_id,
            "test-page".to_string(),
            "Test Page".to_string(),
        );
        page.parent_page_id = root2.id;
        page.template = "default".to_string();
        let page_id = page_repo.create(&page).await?;
        
        // Try to access from site1 service
        let service = DoxydeRmcpService::new(pool, site1_id);
        let req = GetPageRequest { page_id };
        let result = service.get_page(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("Page does not belong to this site"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_page_by_path_root(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::SiteRepository;
        
        // Create test site
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        // The root page should be created automatically
        let service = DoxydeRmcpService::new(pool, site_id);
        
        // Test with "/" path
        let req = GetPageByPathRequest { path: "/".to_string() };
        let result = service.get_page_by_path(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        let page: doxyde_core::models::Page = serde_json::from_str(&result)?;
        assert!(page.parent_page_id.is_none());
        assert_eq!(page.slug, "");
        
        // Test with empty path
        let req = GetPageByPathRequest { path: "".to_string() };
        let result = service.get_page_by_path(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_page_by_path_nested(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository};
        
        // Create test site
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        // Get root page
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create about page
        let mut about = doxyde_core::models::Page::new(
            site_id,
            "about".to_string(),
            "About".to_string(),
        );
        about.parent_page_id = root.id;
        let about_id = page_repo.create(&about).await?;
        
        // Create team page under about
        let mut team = doxyde_core::models::Page::new(
            site_id,
            "team".to_string(),
            "Our Team".to_string(),
        );
        team.parent_page_id = Some(about_id);
        let team_id = page_repo.create(&team).await?;
        
        let service = DoxydeRmcpService::new(pool, site_id);
        
        // Test finding /about
        let req = GetPageByPathRequest { path: "/about".to_string() };
        let result = service.get_page_by_path(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        let page: doxyde_core::models::Page = serde_json::from_str(&result)?;
        assert_eq!(page.slug, "about");
        assert_eq!(page.id.unwrap(), about_id);
        
        // Test finding /about/team
        let req = GetPageByPathRequest { path: "/about/team".to_string() };
        let result = service.get_page_by_path(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        let page: doxyde_core::models::Page = serde_json::from_str(&result)?;
        assert_eq!(page.slug, "team");
        assert_eq!(page.id.unwrap(), team_id);
        
        // Test with different path formats
        let req = GetPageByPathRequest { path: "about/team/".to_string() };
        let result = service.get_page_by_path(Parameters(req)).await;
        assert!(!result.contains("error"));
        
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_page_by_path_not_found(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::SiteRepository;
        
        // Create test site
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let service = DoxydeRmcpService::new(pool, site_id);
        
        let req = GetPageByPathRequest { path: "/does/not/exist".to_string() };
        let result = service.get_page_by_path(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("Page not found at path"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_page_by_path_wrong_site(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository};
        
        // Create two sites
        let site_repo = SiteRepository::new(pool.clone());
        let site1 = doxyde_core::models::Site::new("site1.com".to_string(), "Site 1".to_string());
        let site1_id = site_repo.create(&site1).await?;
        let site2 = doxyde_core::models::Site::new("site2.com".to_string(), "Site 2".to_string());
        let site2_id = site_repo.create(&site2).await?;
        
        // Create page in site2
        let page_repo = PageRepository::new(pool.clone());
        let root2 = page_repo.get_root_page(site2_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        let mut page = doxyde_core::models::Page::new(
            site2_id,
            "exclusive".to_string(),
            "Site 2 Exclusive".to_string(),
        );
        page.parent_page_id = root2.id;
        page_repo.create(&page).await?;
        
        // Try to access from site1 service
        let service = DoxydeRmcpService::new(pool.clone(), site1_id);
        let req = GetPageByPathRequest { path: "/exclusive".to_string() };
        let result = service.get_page_by_path(Parameters(req)).await;
        
        // Should not find the page since it belongs to site2
        assert!(result.contains("error"));
        assert!(result.contains("Page not found at path"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_search_pages_by_title(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository};
        
        // Create test site
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        // Create pages with "team" in title
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        let mut team_page = doxyde_core::models::Page::new(
            site_id,
            "team".to_string(),
            "Our Team".to_string(),
        );
        team_page.parent_page_id = root.id;
        page_repo.create(&team_page).await?;
        
        let mut values_page = doxyde_core::models::Page::new(
            site_id,
            "team-values".to_string(),
            "Team Values".to_string(),
        );
        values_page.parent_page_id = root.id;
        page_repo.create(&values_page).await?;
        
        // Create page without "team"
        let mut about_page = doxyde_core::models::Page::new(
            site_id,
            "about".to_string(),
            "About Us".to_string(),
        );
        about_page.parent_page_id = root.id;
        page_repo.create(&about_page).await?;
        
        let service = DoxydeRmcpService::new(pool, site_id);
        
        let req = SearchPagesRequest { query: "team".to_string() };
        let result = service.search_pages(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        let pages: Vec<PageInfo> = serde_json::from_str(&result)?;
        assert_eq!(pages.len(), 2);
        assert!(pages.iter().all(|p| p.title.to_lowercase().contains("team")));
        Ok(())
    }

    #[sqlx::test]
    async fn test_search_pages_in_content(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository, PageVersionRepository, ComponentRepository};
        
        // Create test site
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        // Create page
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        let mut page = doxyde_core::models::Page::new(
            site_id,
            "collaboration".to_string(),
            "Working Together".to_string(),
        );
        page.parent_page_id = root.id;
        let page_id = page_repo.create(&page).await?;
        
        // Create published version with content containing "collaboration"
        let version_repo = PageVersionRepository::new(pool.clone());
        let version = doxyde_core::models::PageVersion::new(page_id, 1, None);
        let version_id = version_repo.create(&version).await?;
        version_repo.publish(version_id).await?;
        
        // Add component with "collaboration" in content
        let component_repo = ComponentRepository::new(pool.clone());
        let content = serde_json::json!({
            "text": "We believe in collaboration and teamwork to achieve great results."
        });
        
        let mut component = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            0,
            content,
        );
        component.title = Some("Team Philosophy".to_string());
        component_repo.create(&component).await?;
        
        let service = DoxydeRmcpService::new(pool, site_id);
        
        let req = SearchPagesRequest { query: "collaboration".to_string() };
        let result = service.search_pages(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        let pages: Vec<PageInfo> = serde_json::from_str(&result)?;
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].slug, "collaboration");
        Ok(())
    }

    #[sqlx::test]
    async fn test_search_pages_no_results(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::SiteRepository;
        
        // Create test site
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let service = DoxydeRmcpService::new(pool, site_id);
        
        let req = SearchPagesRequest { query: "xyznonexistent".to_string() };
        let result = service.search_pages(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        let pages: Vec<PageInfo> = serde_json::from_str(&result)?;
        assert!(pages.is_empty());
        Ok(())
    }

    #[sqlx::test]
    async fn test_search_pages_case_insensitive(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository};
        
        // Create test site
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        // Create page with mixed case
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        let mut page = doxyde_core::models::Page::new(
            site_id,
            "contact".to_string(),
            "Contact Us".to_string(),
        );
        page.parent_page_id = root.id;
        page.description = Some("Get in touch with our SUPPORT team".to_string());
        page_repo.create(&page).await?;
        
        let service = DoxydeRmcpService::new(pool, site_id);
        
        // Search with different cases
        let req1 = SearchPagesRequest { query: "CONTACT".to_string() };
        let result1 = service.search_pages(Parameters(req1)).await;
        
        let req2 = SearchPagesRequest { query: "support".to_string() };
        let result2 = service.search_pages(Parameters(req2)).await;
        
        assert!(!result1.contains("error"));
        assert!(!result2.contains("error"));
        
        let pages1: Vec<PageInfo> = serde_json::from_str(&result1)?;
        let pages2: Vec<PageInfo> = serde_json::from_str(&result2)?;
        
        assert_eq!(pages1.len(), 1);
        assert_eq!(pages2.len(), 1);
        assert_eq!(pages1[0].slug, "contact");
        assert_eq!(pages2[0].slug, "contact");
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_published_content(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository, PageVersionRepository, ComponentRepository};
        
        // Create test site and page
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create published version for root page
        let version_repo = PageVersionRepository::new(pool.clone());
        let version = doxyde_core::models::PageVersion::new(root.id.unwrap(), 1, None);
        let version_id = version_repo.create(&version).await?;
        version_repo.publish(version_id).await?;
        
        // Add components
        let component_repo = ComponentRepository::new(pool.clone());
        
        let content1 = serde_json::json!({"text": "Welcome to our site"});
        let mut comp1 = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            0,
            content1,
        );
        comp1.title = Some("Welcome".to_string());
        component_repo.create(&comp1).await?;
        
        let content2 = serde_json::json!({"text": "## Features\n\n- Feature 1\n- Feature 2"});
        let mut comp2 = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            1,
            content2,
        );
        comp2.template = "card".to_string();
        comp2.title = Some("Features".to_string());
        component_repo.create(&comp2).await?;
        
        let service = DoxydeRmcpService::new(pool, site_id);
        
        let req = GetPublishedContentRequest { page_id: root.id.unwrap() };
        let result = service.get_published_content(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        let components: Vec<ComponentInfo> = serde_json::from_str(&result)?;
        assert_eq!(components.len(), 2);
        assert_eq!(components[0].position, 0);
        assert_eq!(components[0].title, Some("Welcome".to_string()));
        assert_eq!(components[1].position, 1);
        assert_eq!(components[1].template, "card");
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_published_content_no_version(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository};
        
        // Create test site and page without published version
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let page_repo = PageRepository::new(pool.clone());
        let mut page = doxyde_core::models::Page::new(
            site_id,
            "unpublished".to_string(),
            "Unpublished Page".to_string(),
        );
        page.parent_page_id = page_repo.get_root_page(site_id).await?.unwrap().id;
        let page_id = page_repo.create(&page).await?;
        
        let service = DoxydeRmcpService::new(pool, site_id);
        
        let req = GetPublishedContentRequest { page_id };
        let result = service.get_published_content(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("No published version exists"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_published_content_wrong_site(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository};
        
        // Create two sites
        let site_repo = SiteRepository::new(pool.clone());
        let site1 = doxyde_core::models::Site::new("site1.com".to_string(), "Site 1".to_string());
        let site1_id = site_repo.create(&site1).await?;
        let site2 = doxyde_core::models::Site::new("site2.com".to_string(), "Site 2".to_string());
        let site2_id = site_repo.create(&site2).await?;
        
        // Create page in site2
        let page_repo = PageRepository::new(pool.clone());
        let page2_root = page_repo.get_root_page(site2_id).await?.unwrap();
        
        // Try to access from site1 service
        let service = DoxydeRmcpService::new(pool, site1_id);
        
        let req = GetPublishedContentRequest { page_id: page2_root.id.unwrap() };
        let result = service.get_published_content(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("Page does not belong to this site"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_draft_content_exists(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository, PageVersionRepository, ComponentRepository};
        
        // Create test site and page
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create draft version (not published)
        let version_repo = PageVersionRepository::new(pool.clone());
        let version = doxyde_core::models::PageVersion::new(root.id.unwrap(), 1, None);
        let version_id = version_repo.create(&version).await?;
        // Note: NOT calling publish() - this keeps it as a draft
        
        // Add component to draft
        let component_repo = ComponentRepository::new(pool.clone());
        let content = serde_json::json!({"text": "Draft content here"});
        let mut comp = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            0,
            content,
        );
        comp.title = Some("Draft Title".to_string());
        component_repo.create(&comp).await?;
        
        let service = DoxydeRmcpService::new(pool, site_id);
        
        let req = GetDraftContentRequest { page_id: root.id.unwrap() };
        let result = service.get_draft_content(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        assert!(result != "null");
        let components: Vec<ComponentInfo> = serde_json::from_str(&result)?;
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].title, Some("Draft Title".to_string()));
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_draft_content_no_draft(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository, PageVersionRepository};
        
        // Create test site and page
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create and publish a version (no draft)
        let version_repo = PageVersionRepository::new(pool.clone());
        let version = doxyde_core::models::PageVersion::new(root.id.unwrap(), 1, None);
        let version_id = version_repo.create(&version).await?;
        version_repo.publish(version_id).await?;
        
        let service = DoxydeRmcpService::new(pool, site_id);
        
        let req = GetDraftContentRequest { page_id: root.id.unwrap() };
        let result = service.get_draft_content(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        assert_eq!(result, "null");
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_draft_content_page_not_found(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::SiteRepository;
        
        // Create test site
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let service = DoxydeRmcpService::new(pool, site_id);
        
        let req = GetDraftContentRequest { page_id: 99999 };
        let result = service.get_draft_content(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("Page not found"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_draft_content_wrong_site(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository};
        
        // Create two sites
        let site_repo = SiteRepository::new(pool.clone());
        let site1 = doxyde_core::models::Site::new("site1.com".to_string(), "Site 1".to_string());
        let site1_id = site_repo.create(&site1).await?;
        let site2 = doxyde_core::models::Site::new("site2.com".to_string(), "Site 2".to_string());
        let site2_id = site_repo.create(&site2).await?;
        
        // Get page from site2
        let page_repo = PageRepository::new(pool.clone());
        let page2_root = page_repo.get_root_page(site2_id).await?.unwrap();
        
        // Try to access from site1 service
        let service = DoxydeRmcpService::new(pool, site1_id);
        
        let req = GetDraftContentRequest { page_id: page2_root.id.unwrap() };
        let result = service.get_draft_content(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("Page does not belong to this site"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_or_create_draft_from_published(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository, PageVersionRepository, ComponentRepository};
        
        // Create test site and page
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create and publish a version with components
        let version_repo = PageVersionRepository::new(pool.clone());
        let version = doxyde_core::models::PageVersion::new(root.id.unwrap(), 1, Some("author@example.com".to_string()));
        let version_id = version_repo.create(&version).await?;
        version_repo.publish(version_id).await?;
        
        // Add components to published version
        let component_repo = ComponentRepository::new(pool.clone());
        let content1 = serde_json::json!({"text": "Published content"});
        let mut comp1 = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            0,
            content1,
        );
        comp1.title = Some("Published Title".to_string());
        component_repo.create(&comp1).await?;
        
        let service = DoxydeRmcpService::new(pool, site_id);
        
        let req = GetOrCreateDraftRequest { page_id: root.id.unwrap() };
        let result = service.get_or_create_draft(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        let data: serde_json::Value = serde_json::from_str(&result)?;
        assert_eq!(data["draft"]["version_number"], 2);
        assert_eq!(data["draft"]["is_new"], true);
        assert_eq!(data["draft"]["is_published"], false);
        assert_eq!(data["component_count"], 1);
        assert_eq!(data["components"][0]["title"], "Published Title");
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_or_create_draft_existing(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository, PageVersionRepository};
        
        // Create test site and page
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create a draft version
        let version_repo = PageVersionRepository::new(pool.clone());
        let version = doxyde_core::models::PageVersion::new(root.id.unwrap(), 1, Some("editor@example.com".to_string()));
        let _version_id = version_repo.create(&version).await?;
        
        let service = DoxydeRmcpService::new(pool, site_id);
        
        // First call should return the existing draft
        let req = GetOrCreateDraftRequest { page_id: root.id.unwrap() };
        let result = service.get_or_create_draft(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        let data: serde_json::Value = serde_json::from_str(&result)?;
        assert_eq!(data["draft"]["version_number"], 1);
        assert_eq!(data["draft"]["is_new"], false);  // Existing draft, not newly created
        assert_eq!(data["draft"]["created_by"], "editor@example.com");
        
        // Second call should return the same draft
        let req2 = GetOrCreateDraftRequest { page_id: root.id.unwrap() };
        let result2 = service.get_or_create_draft(Parameters(req2)).await;
        
        assert!(!result2.contains("error"));
        let data2: serde_json::Value = serde_json::from_str(&result2)?;
        assert_eq!(data2["draft"]["version_id"], data["draft"]["version_id"]);
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_or_create_draft_no_versions(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository};
        
        // Create test site and page
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        let service = DoxydeRmcpService::new(pool, site_id);
        
        let req = GetOrCreateDraftRequest { page_id: root.id.unwrap() };
        let result = service.get_or_create_draft(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        let data: serde_json::Value = serde_json::from_str(&result)?;
        assert_eq!(data["draft"]["version_number"], 1);
        assert_eq!(data["draft"]["is_new"], true);
        assert_eq!(data["component_count"], 0);
        Ok(())
    }

    #[sqlx::test]
    async fn test_create_component_markdown(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository};
        
        // Create test site and page
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        let service = DoxydeRmcpService::new(pool, site_id);
        
        let req = CreateComponentMarkdownRequest {
            page_id: root.id.unwrap(),
            position: Some(0),
            template: Some("card".to_string()),
            title: Some("Test Component".to_string()),
            content: "# Test Content\n\nThis is a test.".to_string(),
        };
        
        let result = service.create_component_markdown(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        let component: ComponentInfo = serde_json::from_str(&result)?;
        assert_eq!(component.component_type, "markdown");
        assert_eq!(component.position, 0);
        assert_eq!(component.template, "card");
        assert_eq!(component.title, Some("Test Component".to_string()));
        
        let content = component.content.get("text").unwrap().as_str().unwrap();
        assert!(content.contains("# Test Content"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_create_component_auto_draft(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository, PageVersionRepository};
        
        // Create test site and page without draft
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        let req = CreateComponentMarkdownRequest {
            page_id: root.id.unwrap(),
            position: None,
            template: None,
            title: None,
            content: "Simple content".to_string(),
        };
        
        let result = service.create_component_markdown(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        
        // Verify draft was created
        let version_repo = PageVersionRepository::new(pool);
        let draft = version_repo.get_draft(root.id.unwrap()).await?;
        assert!(draft.is_some());
        Ok(())
    }

    #[sqlx::test]
    async fn test_create_component_position_shift(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository, PageVersionRepository, ComponentRepository};
        
        // Create test site and page with existing components
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create draft with components
        let version_repo = PageVersionRepository::new(pool.clone());
        let version = doxyde_core::models::PageVersion::new(root.id.unwrap(), 1, None);
        let version_id = version_repo.create(&version).await?;
        
        let component_repo = ComponentRepository::new(pool.clone());
        
        // Add 3 components at positions 0, 1, 2
        for i in 0..3 {
            let content = serde_json::json!({"text": format!("Component {}", i)});
            let comp = doxyde_core::models::Component::new(
                version_id,
                "markdown".to_string(),
                i,
                content,
            );
            component_repo.create(&comp).await?;
        }
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        // Insert at position 1
        let req = CreateComponentMarkdownRequest {
            page_id: root.id.unwrap(),
            position: Some(1),
            template: None,
            title: None,
            content: "Inserted content".to_string(),
        };
        
        service.create_component_markdown(Parameters(req)).await;
        
        // Verify positions
        let components = component_repo.list_by_page_version(version_id).await?;
        
        assert_eq!(components.len(), 4);
        assert_eq!(components[0].position, 0); // Original first
        assert_eq!(components[1].position, 1); // New component
        assert_eq!(components[2].position, 2); // Original second (shifted)
        assert_eq!(components[3].position, 3); // Original third (shifted)
        
        // Verify content
        let comp1_text = components[1].content.get("text").unwrap().as_str().unwrap();
        assert_eq!(comp1_text, "Inserted content");
        Ok(())
    }

    #[sqlx::test]
    async fn test_update_component_markdown_all_fields(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository, PageVersionRepository, ComponentRepository};
        
        // Create test site and page
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create draft with a markdown component
        let version_repo = PageVersionRepository::new(pool.clone());
        let version = doxyde_core::models::PageVersion::new(root.id.unwrap(), 1, None);
        let version_id = version_repo.create(&version).await?;
        
        let component_repo = ComponentRepository::new(pool.clone());
        let mut component = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            0,
            serde_json::json!({"text": "Original content"}),
        );
        component.title = Some("Original title".to_string());
        component.template = "default".to_string();
        let component_id = component_repo.create(&component).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        // Update all fields
        let req = UpdateComponentMarkdownRequest {
            component_id,
            content: Some("Updated content".to_string()),
            title: Some("Updated title".to_string()),
            template: Some("highlight".to_string()),
        };
        
        let result = service.update_component_markdown(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        let updated: ComponentInfo = serde_json::from_str(&result)?;
        assert_eq!(updated.template, "highlight");
        assert_eq!(updated.title, Some("Updated title".to_string()));
        
        let content = updated.content.get("text").unwrap().as_str().unwrap();
        assert_eq!(content, "Updated content");
        Ok(())
    }

    #[sqlx::test]
    async fn test_update_component_markdown_partial(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository, PageVersionRepository, ComponentRepository};
        
        // Create test site and page
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create draft with component
        let version_repo = PageVersionRepository::new(pool.clone());
        let version = doxyde_core::models::PageVersion::new(root.id.unwrap(), 1, None);
        let version_id = version_repo.create(&version).await?;
        
        let component_repo = ComponentRepository::new(pool.clone());
        let mut component = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            0,
            serde_json::json!({"text": "Original content"}),
        );
        component.title = Some("Original title".to_string());
        component.template = "default".to_string();
        let component_id = component_repo.create(&component).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        // Only update content
        let req = UpdateComponentMarkdownRequest {
            component_id,
            content: Some("New content only".to_string()),
            title: None,
            template: None,
        };
        
        let result = service.update_component_markdown(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        let updated: ComponentInfo = serde_json::from_str(&result)?;
        
        // Title and template should remain unchanged
        assert_eq!(updated.title, Some("Original title".to_string()));
        assert_eq!(updated.template, "default");
        
        // Content should be updated
        let content = updated.content.get("text").unwrap().as_str().unwrap();
        assert_eq!(content, "New content only");
        Ok(())
    }

    #[sqlx::test]
    async fn test_update_component_published_fails(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository, PageVersionRepository, ComponentRepository};
        
        // Create test site and page
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create published version with component
        let version_repo = PageVersionRepository::new(pool.clone());
        let mut version = doxyde_core::models::PageVersion::new(root.id.unwrap(), 1, None);
        version.is_published = true;
        let version_id = version_repo.create(&version).await?;
        
        let component_repo = ComponentRepository::new(pool.clone());
        let component = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            0,
            serde_json::json!({"text": "Published content"}),
        );
        let component_id = component_repo.create(&component).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        let req = UpdateComponentMarkdownRequest {
            component_id,
            content: Some("Should fail".to_string()),
            title: None,
            template: None,
        };
        
        let result = service.update_component_markdown(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("Cannot update component in published version"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_update_non_markdown_component_fails(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository, PageVersionRepository, ComponentRepository};
        
        // Create test site and page
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create draft with image component
        let version_repo = PageVersionRepository::new(pool.clone());
        let version = doxyde_core::models::PageVersion::new(root.id.unwrap(), 1, None);
        let version_id = version_repo.create(&version).await?;
        
        let component_repo = ComponentRepository::new(pool.clone());
        let component = doxyde_core::models::Component::new(
            version_id,
            "image".to_string(),
            0,
            serde_json::json!({
                "slug": "test-image",
                "format": "jpg",
                "file_path": "/images/test.jpg"
            }),
        );
        let component_id = component_repo.create(&component).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        let req = UpdateComponentMarkdownRequest {
            component_id,
            content: Some("Should fail".to_string()),
            title: None,
            template: None,
        };
        
        let result = service.update_component_markdown(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("not a markdown component"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_publish_draft(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository, PageVersionRepository};
        
        // Create test site and page
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create draft version
        let version_repo = PageVersionRepository::new(pool.clone());
        let draft = doxyde_core::models::PageVersion::new(root.id.unwrap(), 1, None);
        version_repo.create(&draft).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        let req = PublishDraftRequest { page_id: root.id.unwrap() };
        let result = service.publish_draft(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        assert!(result.contains("Successfully published"));
        assert!(result.contains("is now live"));
        
        // Verify draft is now published
        let published = version_repo.get_published(root.id.unwrap()).await?;
        assert!(published.is_some());
        assert_eq!(published.unwrap().version_number, 1);
        Ok(())
    }

    #[sqlx::test]
    async fn test_publish_draft_no_draft(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository};
        
        // Create test site and page without draft
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        let service = DoxydeRmcpService::new(pool, site_id);
        
        let req = PublishDraftRequest { page_id: root.id.unwrap() };
        let result = service.publish_draft(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("No draft version exists"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_publish_draft_replaces_published(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository, PageVersionRepository};
        
        // Create test site and page
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        let version_repo = PageVersionRepository::new(pool.clone());
        
        // Create and publish first version
        let mut v1 = doxyde_core::models::PageVersion::new(root.id.unwrap(), 1, None);
        v1.is_published = true;
        version_repo.create(&v1).await?;
        
        // Create draft version 2
        let v2 = doxyde_core::models::PageVersion::new(root.id.unwrap(), 2, None);
        version_repo.create(&v2).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        let req = PublishDraftRequest { page_id: root.id.unwrap() };
        let result = service.publish_draft(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        assert!(result.contains("Version 2 is now live"));
        
        // Verify only one published version
        let versions = version_repo.list_by_page(root.id.unwrap()).await?;
        let published_count = versions.iter().filter(|v| v.is_published).count();
        assert_eq!(published_count, 1);
        
        // Verify it's version 2
        let published = version_repo.get_published(root.id.unwrap()).await?;
        assert!(published.is_some());
        assert_eq!(published.unwrap().version_number, 2);
        Ok(())
    }

    #[sqlx::test]
    async fn test_create_page(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository};
        
        // Create test site
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        // Get root page ID
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        let req = CreatePageRequest {
            parent_page_id: Some(root.id.unwrap()),
            slug: Some("test-page".to_string()),
            title: "Test Page".to_string(),
            description: Some("Test description for SEO".to_string()),
            keywords: Some("test, page, seo".to_string()),
            template: Some("default".to_string()),
        };
        
        let result = service.create_page(Parameters(req)).await;
        
        assert!(!result.contains("error"), "Result: {}", result);
        let page_info: PageInfo = serde_json::from_str(&result)?;
        assert_eq!(page_info.slug, "test-page");
        assert_eq!(page_info.title, "Test Page");
        assert_eq!(page_info.path, "/test-page");
        assert_eq!(page_info.parent_id, root.id);
        Ok(())
    }

    #[sqlx::test]
    async fn test_create_page_auto_slug(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository};
        
        // Create test site
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        // Get root page ID
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        let req = CreatePageRequest {
            parent_page_id: Some(root.id.unwrap()),
            slug: None,
            title: "Test Page With Spaces & Special!".to_string(),
            description: None,
            keywords: None,
            template: None,
        };
        
        let result = service.create_page(Parameters(req)).await;
        
        assert!(!result.contains("error"), "Result: {}", result);
        let page_info: PageInfo = serde_json::from_str(&result)?;
        assert_eq!(page_info.slug, "test-page-with-spaces-special");
        assert_eq!(page_info.template, Some("default".to_string()));
        Ok(())
    }

    #[sqlx::test]
    async fn test_create_page_duplicate_slug(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository};
        
        // Create test site
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        // Get root page
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        // Create first page
        let req1 = CreatePageRequest {
            parent_page_id: Some(root.id.unwrap()),
            slug: Some("duplicate".to_string()),
            title: "First Page".to_string(),
            description: None,
            keywords: None,
            template: None,
        };
        service.create_page(Parameters(req1)).await;
        
        // Try to create second with same slug
        let req2 = CreatePageRequest {
            parent_page_id: Some(root.id.unwrap()),
            slug: Some("duplicate".to_string()),
            title: "Second Page".to_string(),
            description: None,
            keywords: None,
            template: None,
        };
        let result = service.create_page(Parameters(req2)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("already exists"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_create_page_wrong_site_parent(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository};
        
        // Create two sites
        let site_repo = SiteRepository::new(pool.clone());
        let site1 = doxyde_core::models::Site::new("site1.com".to_string(), "Site 1".to_string());
        let site1_id = site_repo.create(&site1).await?;
        let site2 = doxyde_core::models::Site::new("site2.com".to_string(), "Site 2".to_string());
        let site2_id = site_repo.create(&site2).await?;
        
        // Get root from site2
        let page_repo = PageRepository::new(pool.clone());
        let root2 = page_repo.get_root_page(site2_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Try to create page under site2's root from site1's service
        let service = DoxydeRmcpService::new(pool.clone(), site1_id);
        
        let req = CreatePageRequest {
            parent_page_id: Some(root2.id.unwrap()),
            slug: Some("should-fail".to_string()),
            title: "Should Fail".to_string(),
            description: None,
            keywords: None,
            template: None,
        };
        let result = service.create_page(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("does not belong to this site"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_create_root_page_fails(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::SiteRepository;
        
        // Create test site (root page already exists)
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        // Try to create another root page
        let req = CreatePageRequest {
            parent_page_id: None,
            slug: Some("another-root".to_string()),
            title: "Another Root".to_string(),
            description: None,
            keywords: None,
            template: None,
        };
        let result = service.create_page(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("Root page already exists"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_update_page_all_fields(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository};
        
        // Create test site
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        // Get root page
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create a page to update
        let mut page = doxyde_core::models::Page::new(site_id, "original".to_string(), "Original Title".to_string());
        page.parent_page_id = root.id;
        page.description = Some("Original description".to_string());
        page.keywords = Some("original, keywords".to_string());
        page.template = "default".to_string();
        let page_id = page_repo.create(&page).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        let req = UpdatePageRequest {
            page_id,
            slug: Some("updated-slug".to_string()),
            title: Some("Updated Title".to_string()),
            description: Some("Updated description".to_string()),
            keywords: Some("updated, keywords".to_string()),
            template: Some("landing".to_string()),
        };
        
        let result = service.update_page(Parameters(req)).await;
        
        assert!(!result.contains("error"), "Result: {}", result);
        let page_info: PageInfo = serde_json::from_str(&result)?;
        assert_eq!(page_info.slug, "updated-slug");
        assert_eq!(page_info.title, "Updated Title");
        assert_eq!(page_info.template, Some("landing".to_string()));
        assert_eq!(page_info.path, "/updated-slug");
        Ok(())
    }

    #[sqlx::test]
    async fn test_update_page_partial(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository};
        
        // Create test site
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        // Get root page
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create a page to update
        let mut page = doxyde_core::models::Page::new(site_id, "original".to_string(), "Original Title".to_string());
        page.parent_page_id = root.id;
        page.template = "default".to_string();
        let page_id = page_repo.create(&page).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        // Only update title
        let req = UpdatePageRequest {
            page_id,
            slug: None,
            title: Some("New Title Only".to_string()),
            description: None,
            keywords: None,
            template: None,
        };
        
        let result = service.update_page(Parameters(req)).await;
        
        assert!(!result.contains("error"), "Result: {}", result);
        let page_info: PageInfo = serde_json::from_str(&result)?;
        assert_eq!(page_info.title, "New Title Only");
        assert_eq!(page_info.slug, "original"); // Unchanged
        assert_eq!(page_info.template, Some("default".to_string())); // Unchanged
        Ok(())
    }

    #[sqlx::test]
    async fn test_update_page_duplicate_slug(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository};
        
        // Create test site
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        // Get root page
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create two pages
        let mut page1 = doxyde_core::models::Page::new(site_id, "page1".to_string(), "Page 1".to_string());
        page1.parent_page_id = root.id;
        let page_id1 = page_repo.create(&page1).await?;
        
        let mut page2 = doxyde_core::models::Page::new(site_id, "existing".to_string(), "Page 2".to_string());
        page2.parent_page_id = root.id;
        let _page_id2 = page_repo.create(&page2).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        // Try to update page1 with page2's slug
        let req = UpdatePageRequest {
            page_id: page_id1,
            slug: Some("existing".to_string()),
            title: None,
            description: None,
            keywords: None,
            template: None,
        };
        
        let result = service.update_page(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("already exists"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_update_page_wrong_site(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository};
        
        // Create two sites
        let site_repo = SiteRepository::new(pool.clone());
        let site1 = doxyde_core::models::Site::new("site1.com".to_string(), "Site 1".to_string());
        let site1_id = site_repo.create(&site1).await?;
        let site2 = doxyde_core::models::Site::new("site2.com".to_string(), "Site 2".to_string());
        let site2_id = site_repo.create(&site2).await?;
        
        // Create page in site2
        let page_repo = PageRepository::new(pool.clone());
        let root2 = page_repo.get_root_page(site2_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        let mut page = doxyde_core::models::Page::new(site2_id, "page".to_string(), "Page".to_string());
        page.parent_page_id = root2.id;
        let page_id = page_repo.create(&page).await?;
        
        // Try to update from site1's service
        let service = DoxydeRmcpService::new(pool.clone(), site1_id);
        
        let req = UpdatePageRequest {
            page_id,
            slug: Some("should-fail".to_string()),
            title: None,
            description: None,
            keywords: None,
            template: None,
        };
        
        let result = service.update_page(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("does not belong to this site"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_update_page_not_found(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::SiteRepository;
        
        // Create test site
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        let req = UpdatePageRequest {
            page_id: 999,
            slug: Some("should-fail".to_string()),
            title: None,
            description: None,
            keywords: None,
            template: None,
        };
        
        let result = service.update_page(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("Page not found"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_component(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository, PageVersionRepository, ComponentRepository};
        
        // Create test site and page
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create draft version
        let version_repo = PageVersionRepository::new(pool.clone());
        let version = doxyde_core::models::PageVersion::new(root.id.unwrap(), 1, None);
        let version_id = version_repo.create(&version).await?;
        
        // Create component
        let component_repo = ComponentRepository::new(pool.clone());
        let component = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            0,
            serde_json::json!({"text": "Test content"}),
        );
        let component_id = component_repo.create(&component).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        let req = DeleteComponentRequest { component_id };
        let result = service.delete_component(Parameters(req)).await;
        
        assert!(!result.contains("error"), "Result: {}", result);
        assert!(result.contains("Successfully deleted"));
        
        // Verify component is gone
        let component = component_repo.find_by_id(component_id).await?;
        assert!(component.is_none());
        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_component_position_update(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository, PageVersionRepository, ComponentRepository};
        
        // Create test site and page
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create draft version
        let version_repo = PageVersionRepository::new(pool.clone());
        let version = doxyde_core::models::PageVersion::new(root.id.unwrap(), 1, None);
        let version_id = version_repo.create(&version).await?;
        
        // Create 3 components at positions 0, 1, 2
        let component_repo = ComponentRepository::new(pool.clone());
        let comp1 = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            0,
            serde_json::json!({"text": "Component 1"}),
        );
        let comp1_id = component_repo.create(&comp1).await?;
        
        let comp2 = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            1,
            serde_json::json!({"text": "Component 2"}),
        );
        let comp2_id = component_repo.create(&comp2).await?;
        
        let comp3 = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            2,
            serde_json::json!({"text": "Component 3"}),
        );
        let comp3_id = component_repo.create(&comp3).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        // Delete middle component
        let req = DeleteComponentRequest { component_id: comp2_id };
        service.delete_component(Parameters(req)).await;
        
        // Verify positions updated
        let comp1 = component_repo.find_by_id(comp1_id).await?.unwrap();
        let comp3 = component_repo.find_by_id(comp3_id).await?.unwrap();
        
        assert_eq!(comp1.position, 0); // Should remain at 0
        assert_eq!(comp3.position, 1); // Should move from 2 to 1
        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_component_published_fails(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository, PageVersionRepository, ComponentRepository};
        
        // Create test site and page
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create published version
        let version_repo = PageVersionRepository::new(pool.clone());
        let version = doxyde_core::models::PageVersion::new(root.id.unwrap(), 1, None);
        let version_id = version_repo.create(&version).await?;
        version_repo.publish(version_id).await?;
        
        // Create component in published version
        let component_repo = ComponentRepository::new(pool.clone());
        let component = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            0,
            serde_json::json!({"text": "Published content"}),
        );
        let component_id = component_repo.create(&component).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        let req = DeleteComponentRequest { component_id };
        let result = service.delete_component(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("Cannot delete component from published version"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_component_wrong_site(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository, PageVersionRepository, ComponentRepository};
        
        // Create two sites
        let site_repo = SiteRepository::new(pool.clone());
        let site1 = doxyde_core::models::Site::new("site1.com".to_string(), "Site 1".to_string());
        let site1_id = site_repo.create(&site1).await?;
        let site2 = doxyde_core::models::Site::new("site2.com".to_string(), "Site 2".to_string());
        let site2_id = site_repo.create(&site2).await?;
        
        // Create page in site2
        let page_repo = PageRepository::new(pool.clone());
        let root2 = page_repo.get_root_page(site2_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create draft version in site2
        let version_repo = PageVersionRepository::new(pool.clone());
        let version = doxyde_core::models::PageVersion::new(root2.id.unwrap(), 1, None);
        let version_id = version_repo.create(&version).await?;
        
        // Create component in site2
        let component_repo = ComponentRepository::new(pool.clone());
        let component = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            0,
            serde_json::json!({"text": "Site 2 content"}),
        );
        let component_id = component_repo.create(&component).await?;
        
        // Try to delete from site1's service
        let service = DoxydeRmcpService::new(pool.clone(), site1_id);
        
        let req = DeleteComponentRequest { component_id };
        let result = service.delete_component(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("different site"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_component_not_found(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::SiteRepository;
        
        // Create test site
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        let req = DeleteComponentRequest { component_id: 999 };
        let result = service.delete_component(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("Component not found"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_discard_draft(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository, PageVersionRepository};
        
        // Create test site and page
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create draft version
        let version_repo = PageVersionRepository::new(pool.clone());
        let version = doxyde_core::models::PageVersion::new(root.id.unwrap(), 1, None);
        version_repo.create(&version).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        let req = DiscardDraftRequest { page_id: root.id.unwrap() };
        let result = service.discard_draft(Parameters(req)).await;
        
        assert!(!result.contains("error"), "Result: {}", result);
        assert!(result.contains("Successfully discarded"));
        
        // Verify draft is gone
        let draft = version_repo.get_draft(root.id.unwrap()).await?;
        assert!(draft.is_none());
        Ok(())
    }

    #[sqlx::test]
    async fn test_discard_draft_no_draft(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository, PageVersionRepository};
        
        // Create test site and page (root page has no versions initially)
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create a published version (not a draft)
        let version_repo = PageVersionRepository::new(pool.clone());
        let version = doxyde_core::models::PageVersion::new(root.id.unwrap(), 1, None);
        let version_id = version_repo.create(&version).await?;
        version_repo.publish(version_id).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        let req = DiscardDraftRequest { page_id: root.id.unwrap() };
        let result = service.discard_draft(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("No draft version exists"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_discard_draft_cascade_delete(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository, PageVersionRepository, ComponentRepository};
        
        // Create test site and page
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create draft version
        let version_repo = PageVersionRepository::new(pool.clone());
        let version = doxyde_core::models::PageVersion::new(root.id.unwrap(), 1, None);
        let version_id = version_repo.create(&version).await?;
        
        // Create components in the draft
        let component_repo = ComponentRepository::new(pool.clone());
        let comp1 = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            0,
            serde_json::json!({"text": "Component 1"}),
        );
        component_repo.create(&comp1).await?;
        
        let comp2 = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            1,
            serde_json::json!({"text": "Component 2"}),
        );
        component_repo.create(&comp2).await?;
        
        // Verify components exist
        let components_before = component_repo.list_by_page_version(version_id).await?;
        assert_eq!(components_before.len(), 2);
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        // Discard draft
        let req = DiscardDraftRequest { page_id: root.id.unwrap() };
        service.discard_draft(Parameters(req)).await;
        
        // Verify components are also deleted
        let components_after = component_repo.list_by_page_version(version_id).await?;
        assert!(components_after.is_empty());
        Ok(())
    }

    #[sqlx::test]
    async fn test_discard_draft_wrong_site(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository, PageVersionRepository};
        
        // Create two sites
        let site_repo = SiteRepository::new(pool.clone());
        let site1 = doxyde_core::models::Site::new("site1.com".to_string(), "Site 1".to_string());
        let site1_id = site_repo.create(&site1).await?;
        let site2 = doxyde_core::models::Site::new("site2.com".to_string(), "Site 2".to_string());
        let site2_id = site_repo.create(&site2).await?;
        
        // Create page in site2
        let page_repo = PageRepository::new(pool.clone());
        let root2 = page_repo.get_root_page(site2_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create draft in site2
        let version_repo = PageVersionRepository::new(pool.clone());
        let version = doxyde_core::models::PageVersion::new(root2.id.unwrap(), 1, None);
        version_repo.create(&version).await?;
        
        // Try to discard from site1's service
        let service = DoxydeRmcpService::new(pool.clone(), site1_id);
        
        let req = DiscardDraftRequest { page_id: root2.id.unwrap() };
        let result = service.discard_draft(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("does not belong to this site"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_discard_draft_page_not_found(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::SiteRepository;
        
        // Create test site
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        let req = DiscardDraftRequest { page_id: 999 };
        let result = service.discard_draft(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("Page not found"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_list_components_draft(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository, PageVersionRepository, ComponentRepository};
        
        // Create test site and page
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create published version with components
        let version_repo = PageVersionRepository::new(pool.clone());
        let published = doxyde_core::models::PageVersion::new(root.id.unwrap(), 1, None);
        let published_id = version_repo.create(&published).await?;
        version_repo.publish(published_id).await?;
        
        let component_repo = ComponentRepository::new(pool.clone());
        let pub_comp = doxyde_core::models::Component::new(
            published_id,
            "markdown".to_string(),
            0,
            serde_json::json!({"text": "Published content"}),
        );
        component_repo.create(&pub_comp).await?;
        
        // Create draft version with different components
        let draft = doxyde_core::models::PageVersion::new(root.id.unwrap(), 2, None);
        let draft_id = version_repo.create(&draft).await?;
        
        let draft_comp1 = doxyde_core::models::Component::new(
            draft_id,
            "markdown".to_string(),
            0,
            serde_json::json!({"text": "Draft content 1"}),
        );
        component_repo.create(&draft_comp1).await?;
        
        let mut draft_comp2 = doxyde_core::models::Component::new(
            draft_id,
            "image".to_string(),
            1,
            serde_json::json!({
                "slug": "test-image",
                "format": "jpg",
                "file_path": "/images/test.jpg"
            }),
        );
        draft_comp2.title = Some("Test Image".to_string());
        component_repo.create(&draft_comp2).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        let req = ListComponentsRequest { page_id: root.id.unwrap() };
        let result = service.list_components(Parameters(req)).await;
        
        assert!(!result.contains("error"), "Result: {}", result);
        let components: Vec<ComponentInfo> = serde_json::from_str(&result)?;
        
        // Should return draft components (not published)
        assert_eq!(components.len(), 2);
        assert_eq!(components[0].component_type, "markdown");
        assert_eq!(components[0].position, 0);
        assert_eq!(components[1].component_type, "image");
        assert_eq!(components[1].position, 1);
        assert_eq!(components[1].title, Some("Test Image".to_string()));
        Ok(())
    }

    #[sqlx::test]
    async fn test_list_components_published_only(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository, PageVersionRepository, ComponentRepository};
        
        // Create test site and page
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create only published version with components
        let version_repo = PageVersionRepository::new(pool.clone());
        let published = doxyde_core::models::PageVersion::new(root.id.unwrap(), 1, None);
        let published_id = version_repo.create(&published).await?;
        version_repo.publish(published_id).await?;
        
        let component_repo = ComponentRepository::new(pool.clone());
        let comp1 = doxyde_core::models::Component::new(
            published_id,
            "markdown".to_string(),
            0,
            serde_json::json!({"text": "Published content"}),
        );
        component_repo.create(&comp1).await?;
        
        let comp2 = doxyde_core::models::Component::new(
            published_id,
            "code".to_string(),
            1,
            serde_json::json!({"code": "console.log('hello');", "language": "javascript"}),
        );
        component_repo.create(&comp2).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        let req = ListComponentsRequest { page_id: root.id.unwrap() };
        let result = service.list_components(Parameters(req)).await;
        
        assert!(!result.contains("error"), "Result: {}", result);
        let components: Vec<ComponentInfo> = serde_json::from_str(&result)?;
        
        assert_eq!(components.len(), 2);
        assert_eq!(components[0].component_type, "markdown");
        assert_eq!(components[1].component_type, "code");
        Ok(())
    }

    #[sqlx::test]
    async fn test_list_components_no_versions(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository};
        
        // Create test site and page
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create additional page without any versions
        let mut page = doxyde_core::models::Page::new(site_id, "empty".to_string(), "Empty Page".to_string());
        page.parent_page_id = root.id;
        let page_id = page_repo.create(&page).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        let req = ListComponentsRequest { page_id };
        let result = service.list_components(Parameters(req)).await;
        
        assert!(!result.contains("error"), "Result: {}", result);
        let components: Vec<ComponentInfo> = serde_json::from_str(&result)?;
        assert!(components.is_empty());
        Ok(())
    }

    #[sqlx::test]
    async fn test_list_components_wrong_site(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository};
        
        // Create two sites
        let site_repo = SiteRepository::new(pool.clone());
        let site1 = doxyde_core::models::Site::new("site1.com".to_string(), "Site 1".to_string());
        let site1_id = site_repo.create(&site1).await?;
        let site2 = doxyde_core::models::Site::new("site2.com".to_string(), "Site 2".to_string());
        let site2_id = site_repo.create(&site2).await?;
        
        // Create page in site2
        let page_repo = PageRepository::new(pool.clone());
        let root2 = page_repo.get_root_page(site2_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Try to list components from site1's service
        let service = DoxydeRmcpService::new(pool.clone(), site1_id);
        
        let req = ListComponentsRequest { page_id: root2.id.unwrap() };
        let result = service.list_components(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("does not belong to this site"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_list_components_page_not_found(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::SiteRepository;
        
        // Create test site
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        let req = ListComponentsRequest { page_id: 999 };
        let result = service.list_components(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("Page not found"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_component(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository, PageVersionRepository, ComponentRepository};
        
        // Create test site and page
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create version
        let version_repo = PageVersionRepository::new(pool.clone());
        let version = doxyde_core::models::PageVersion::new(root.id.unwrap(), 1, None);
        let version_id = version_repo.create(&version).await?;
        
        // Create component
        let component_repo = ComponentRepository::new(pool.clone());
        let mut component = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            2,
            serde_json::json!({"text": "Test content"}),
        );
        component.title = Some("Test Component".to_string());
        component.template = "card".to_string();
        let component_id = component_repo.create(&component).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        let req = GetComponentRequest { component_id };
        let result = service.get_component(Parameters(req)).await;
        
        assert!(!result.contains("error"), "Result: {}", result);
        let comp_info: ComponentInfo = serde_json::from_str(&result)?;
        assert_eq!(comp_info.id, component_id);
        assert_eq!(comp_info.component_type, "markdown");
        assert_eq!(comp_info.position, 2);
        assert_eq!(comp_info.template, "card");
        assert_eq!(comp_info.title, Some("Test Component".to_string()));
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_component_not_found(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::SiteRepository;
        
        // Create test site
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        let req = GetComponentRequest { component_id: 99999 };
        let result = service.get_component(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("Component not found"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_component_wrong_site(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository, PageVersionRepository, ComponentRepository};
        
        // Create two sites
        let site_repo = SiteRepository::new(pool.clone());
        let site1 = doxyde_core::models::Site::new("site1.com".to_string(), "Site 1".to_string());
        let site1_id = site_repo.create(&site1).await?;
        let site2 = doxyde_core::models::Site::new("site2.com".to_string(), "Site 2".to_string());
        let site2_id = site_repo.create(&site2).await?;
        
        // Create page in site2
        let page_repo = PageRepository::new(pool.clone());
        let root2 = page_repo.get_root_page(site2_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create version in site2
        let version_repo = PageVersionRepository::new(pool.clone());
        let version = doxyde_core::models::PageVersion::new(root2.id.unwrap(), 1, None);
        let version_id = version_repo.create(&version).await?;
        
        // Create component in site2
        let component_repo = ComponentRepository::new(pool.clone());
        let component = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            0,
            serde_json::json!({"text": "Site 2 content"}),
        );
        let component_id = component_repo.create(&component).await?;
        
        // Try to get from site1's service
        let service = DoxydeRmcpService::new(pool.clone(), site1_id);
        
        let req = GetComponentRequest { component_id };
        let result = service.get_component(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("different site"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_page_simple(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository};
        
        // Create site
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        // Get root page
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create a child page
        let mut child = doxyde_core::models::Page::new(site_id, "child".to_string(), "Child".to_string());
        child.parent_page_id = Some(root.id.unwrap());
        child.canonical_url = Some("/child".to_string());
        let child_id = page_repo.create(&child).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        let req = DeletePageRequest { page_id: child_id };
        let result = service.delete_page(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        assert!(result.contains("Successfully deleted"));
        
        // Verify page is gone
        let page = page_repo.find_by_id(child_id).await?;
        assert!(page.is_none());
        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_page_with_children_fails(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository};
        
        // Create site
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        // Create page hierarchy: root -> parent -> child1, child2
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        let mut parent = doxyde_core::models::Page::new(site_id, "parent".to_string(), "Parent".to_string());
        parent.parent_page_id = Some(root.id.unwrap());
        parent.canonical_url = Some("/parent".to_string());
        let parent_id = page_repo.create(&parent).await?;
        
        let mut child1 = doxyde_core::models::Page::new(site_id, "child1".to_string(), "Child 1".to_string());
        child1.parent_page_id = Some(parent_id);
        child1.canonical_url = Some("/parent/child1".to_string());
        let child1_id = page_repo.create(&child1).await?;
        
        let mut child2 = doxyde_core::models::Page::new(site_id, "child2".to_string(), "Child 2".to_string());
        child2.parent_page_id = Some(parent_id);
        child2.canonical_url = Some("/parent/child2".to_string());
        let child2_id = page_repo.create(&child2).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        // Try to delete parent page that has children
        let req = DeletePageRequest { page_id: parent_id };
        let result = service.delete_page(Parameters(req)).await;
        
        // Should fail
        assert!(result.contains("error"));
        assert!(result.contains("has 2 child page(s)"));
        
        // Verify all pages still exist
        assert!(page_repo.find_by_id(parent_id).await?.is_some());
        assert!(page_repo.find_by_id(child1_id).await?.is_some());
        assert!(page_repo.find_by_id(child2_id).await?.is_some());
        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_root_page_fails(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository};
        
        // Create site
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        // Get root page
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        let root_id = root.id.unwrap();
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        let req = DeletePageRequest { page_id: root_id };
        let result = service.delete_page(Parameters(req)).await;
        
        assert!(result.contains("error"), "Expected error, got: {}", result);
        assert!(result.contains("Cannot delete root page"), "Error: {}", result);
        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_page_updates_positions(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository};
        
        // Create site
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        // Create pages: root -> page1(pos=0), page2(pos=1), page3(pos=2)
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        let root_id = root.id.unwrap();
        
        let mut page1 = doxyde_core::models::Page::new(site_id, "page1".to_string(), "Page 1".to_string());
        page1.parent_page_id = Some(root_id);
        page1.canonical_url = Some("/page1".to_string());
        page1.position = 0;
        let page1_id = page_repo.create(&page1).await?;
        
        let mut page2 = doxyde_core::models::Page::new(site_id, "page2".to_string(), "Page 2".to_string());
        page2.parent_page_id = Some(root_id);
        page2.canonical_url = Some("/page2".to_string());
        page2.position = 1;
        let page2_id = page_repo.create(&page2).await?;
        
        let mut page3 = doxyde_core::models::Page::new(site_id, "page3".to_string(), "Page 3".to_string());
        page3.parent_page_id = Some(root_id);
        page3.canonical_url = Some("/page3".to_string());
        page3.position = 2;
        let page3_id = page_repo.create(&page3).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        // Delete middle page
        let req = DeletePageRequest { page_id: page2_id };
        let result = service.delete_page(Parameters(req)).await;
        assert!(!result.contains("error"));
        
        // Check positions are updated
        let page1 = page_repo.find_by_id(page1_id).await?.unwrap();
        let page3 = page_repo.find_by_id(page3_id).await?.unwrap();
        
        assert_eq!(page1.position, 0);
        assert_eq!(page3.position, 1); // Should move from 2 to 1
        Ok(())
    }

    #[sqlx::test]
    async fn test_move_page_different_parent(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository};
        
        // Create site
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        // Create pages: root -> parent1 -> child, parent2
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        let root_id = root.id.unwrap();
        
        let mut parent1 = doxyde_core::models::Page::new(site_id, "parent1".to_string(), "Parent 1".to_string());
        parent1.parent_page_id = Some(root_id);
        parent1.canonical_url = Some("/parent1".to_string());
        let parent1_id = page_repo.create(&parent1).await?;
        
        let mut parent2 = doxyde_core::models::Page::new(site_id, "parent2".to_string(), "Parent 2".to_string());
        parent2.parent_page_id = Some(root_id);
        parent2.canonical_url = Some("/parent2".to_string());
        let parent2_id = page_repo.create(&parent2).await?;
        
        let mut child = doxyde_core::models::Page::new(site_id, "child".to_string(), "Child".to_string());
        child.parent_page_id = Some(parent1_id);
        child.canonical_url = Some("/parent1/child".to_string());
        let child_id = page_repo.create(&child).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        let req = MovePageRequest {
            page_id: child_id,
            new_parent_id: Some(parent2_id),
            position: Some(0),
        };
        
        let result = service.move_page(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        let page_info: PageInfo = serde_json::from_str(&result)?;
        assert_eq!(page_info.parent_id, Some(parent2_id));
        assert_eq!(page_info.position, 0);
        assert_eq!(page_info.path, "/parent2/child");
        Ok(())
    }

    #[sqlx::test]
    async fn test_move_page_reorder_same_parent(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository};
        
        // Create site
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        // Create pages: root -> page1(0), page2(1), page3(2)
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        let root_id = root.id.unwrap();
        
        let mut page1 = doxyde_core::models::Page::new(site_id, "page1".to_string(), "Page 1".to_string());
        page1.parent_page_id = Some(root_id);
        page1.canonical_url = Some("/page1".to_string());
        page1.position = 0;
        let page1_id = page_repo.create(&page1).await?;
        
        let mut page2 = doxyde_core::models::Page::new(site_id, "page2".to_string(), "Page 2".to_string());
        page2.parent_page_id = Some(root_id);
        page2.canonical_url = Some("/page2".to_string());
        page2.position = 1;
        let page2_id = page_repo.create(&page2).await?;
        
        let mut page3 = doxyde_core::models::Page::new(site_id, "page3".to_string(), "Page 3".to_string());
        page3.parent_page_id = Some(root_id);
        page3.canonical_url = Some("/page3".to_string());
        page3.position = 2;
        let page3_id = page_repo.create(&page3).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        // Move page3 to position 0
        let req = MovePageRequest {
            page_id: page3_id,
            new_parent_id: Some(root_id),
            position: Some(0),
        };
        
        let result = service.move_page(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        let page_info: PageInfo = serde_json::from_str(&result)?;
        assert_eq!(page_info.position, 0);
        
        // Verify other positions updated
        let page1 = page_repo.find_by_id(page1_id).await?.unwrap();
        let page2 = page_repo.find_by_id(page2_id).await?.unwrap();
        assert_eq!(page1.position, 1);
        assert_eq!(page2.position, 2);
        Ok(())
    }

    #[sqlx::test]
    async fn test_move_page_circular_reference(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository};
        
        // Create site
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        // Create pages: root -> parent -> child
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        let root_id = root.id.unwrap();
        
        let mut parent = doxyde_core::models::Page::new(site_id, "parent".to_string(), "Parent".to_string());
        parent.parent_page_id = Some(root_id);
        parent.canonical_url = Some("/parent".to_string());
        let parent_id = page_repo.create(&parent).await?;
        
        let mut child = doxyde_core::models::Page::new(site_id, "child".to_string(), "Child".to_string());
        child.parent_page_id = Some(parent_id);
        child.canonical_url = Some("/parent/child".to_string());
        let child_id = page_repo.create(&child).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        // Try to move parent under child
        let req = MovePageRequest {
            page_id: parent_id,
            new_parent_id: Some(child_id),
            position: None,
        };
        
        let result = service.move_page(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("Cannot move page under its own descendant"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_move_root_page_fails(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository};
        
        // Create site
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        // Get root page
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        let root_id = root.id.unwrap();
        
        let mut other = doxyde_core::models::Page::new(site_id, "other".to_string(), "Other".to_string());
        other.parent_page_id = Some(root_id);
        other.canonical_url = Some("/other".to_string());
        let other_id = page_repo.create(&other).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        let req = MovePageRequest {
            page_id: root_id,
            new_parent_id: Some(other_id),
            position: None,
        };
        
        let result = service.move_page(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("Cannot move the root page"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_move_component_before_down(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository, PageVersionRepository, ComponentRepository};
        
        // Create site
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        // Create page
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create draft version
        let version_repo = PageVersionRepository::new(pool.clone());
        let version = doxyde_core::models::PageVersion::new(root.id.unwrap(), 1, None);
        let version_id = version_repo.create(&version).await?;
        
        // Create components: 0, 1, 2, 3
        let component_repo = ComponentRepository::new(pool.clone());
        let comp0 = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            0,
            serde_json::json!({"text": "Component 0"}),
        );
        let comp0_id = component_repo.create(&comp0).await?;
        
        let comp1 = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            1,
            serde_json::json!({"text": "Component 1"}),
        );
        let comp1_id = component_repo.create(&comp1).await?;
        
        let comp2 = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            2,
            serde_json::json!({"text": "Component 2"}),
        );
        let comp2_id = component_repo.create(&comp2).await?;
        
        let comp3 = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            3,
            serde_json::json!({"text": "Component 3"}),
        );
        let comp3_id = component_repo.create(&comp3).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        // Move component 0 before component 3 (moving down)
        let req = MoveComponentBeforeRequest {
            component_id: comp0_id,
            before_component_id: comp3_id,
        };
        
        let result = service.move_component_before(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        let moved: ComponentInfo = serde_json::from_str(&result)?;
        assert_eq!(moved.position, 2); // Should be at position 2 (before 3)
        
        // Verify new order: 1, 2, 0, 3
        assert_eq!(component_repo.find_by_id(comp1_id).await?.unwrap().position, 0);
        assert_eq!(component_repo.find_by_id(comp2_id).await?.unwrap().position, 1);
        assert_eq!(component_repo.find_by_id(comp0_id).await?.unwrap().position, 2);
        assert_eq!(component_repo.find_by_id(comp3_id).await?.unwrap().position, 3);
        Ok(())
    }

    #[sqlx::test]
    async fn test_move_component_before_up(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository, PageVersionRepository, ComponentRepository};
        
        // Create site
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        // Create page
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create draft version
        let version_repo = PageVersionRepository::new(pool.clone());
        let version = doxyde_core::models::PageVersion::new(root.id.unwrap(), 1, None);
        let version_id = version_repo.create(&version).await?;
        
        // Create components: 0, 1, 2, 3
        let component_repo = ComponentRepository::new(pool.clone());
        let comp0 = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            0,
            serde_json::json!({"text": "Component 0"}),
        );
        let comp0_id = component_repo.create(&comp0).await?;
        
        let comp1 = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            1,
            serde_json::json!({"text": "Component 1"}),
        );
        let comp1_id = component_repo.create(&comp1).await?;
        
        let comp2 = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            2,
            serde_json::json!({"text": "Component 2"}),
        );
        let comp2_id = component_repo.create(&comp2).await?;
        
        let comp3 = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            3,
            serde_json::json!({"text": "Component 3"}),
        );
        let comp3_id = component_repo.create(&comp3).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        // Move component 3 before component 1 (moving up)
        let req = MoveComponentBeforeRequest {
            component_id: comp3_id,
            before_component_id: comp1_id,
        };
        
        let result = service.move_component_before(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        let moved: ComponentInfo = serde_json::from_str(&result)?;
        assert_eq!(moved.position, 1); // Should be at position 1
        
        // Verify new order: 0, 3, 1, 2
        assert_eq!(component_repo.find_by_id(comp0_id).await?.unwrap().position, 0);
        assert_eq!(component_repo.find_by_id(comp3_id).await?.unwrap().position, 1);
        assert_eq!(component_repo.find_by_id(comp1_id).await?.unwrap().position, 2);
        assert_eq!(component_repo.find_by_id(comp2_id).await?.unwrap().position, 3);
        Ok(())
    }

    #[sqlx::test]
    async fn test_move_component_before_self_fails(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        let service = DoxydeRmcpService::new(pool, 1);
        
        let req = MoveComponentBeforeRequest {
            component_id: 100,
            before_component_id: 100,
        };
        
        let result = service.move_component_before(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("Cannot move component before itself"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_move_component_different_versions_fails(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository, PageVersionRepository, ComponentRepository};
        
        // Create site
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        // Create page
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create two different versions
        let version_repo = PageVersionRepository::new(pool.clone());
        let version1 = doxyde_core::models::PageVersion::new(root.id.unwrap(), 1, None);
        let version1_id = version_repo.create(&version1).await?;
        
        let version2 = doxyde_core::models::PageVersion::new(root.id.unwrap(), 2, None);
        let version2_id = version_repo.create(&version2).await?;
        
        // Create components in different versions
        let component_repo = ComponentRepository::new(pool.clone());
        let comp1 = doxyde_core::models::Component::new(
            version1_id,
            "markdown".to_string(),
            0,
            serde_json::json!({"text": "Component 1"}),
        );
        let comp1_id = component_repo.create(&comp1).await?;
        
        let comp2 = doxyde_core::models::Component::new(
            version2_id,
            "markdown".to_string(),
            0,
            serde_json::json!({"text": "Component 2"}),
        );
        let comp2_id = component_repo.create(&comp2).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        let req = MoveComponentBeforeRequest {
            component_id: comp1_id,
            before_component_id: comp2_id,
        };
        
        let result = service.move_component_before(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("must be in the same page version"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_move_component_after_down(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository, PageVersionRepository, ComponentRepository};
        
        // Create site
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        // Create page
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create draft version
        let version_repo = PageVersionRepository::new(pool.clone());
        let version = doxyde_core::models::PageVersion::new(root.id.unwrap(), 1, None);
        let version_id = version_repo.create(&version).await?;
        
        // Create components: 0, 1, 2, 3
        let component_repo = ComponentRepository::new(pool.clone());
        let comp0 = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            0,
            serde_json::json!({"text": "Component 0"}),
        );
        let comp0_id = component_repo.create(&comp0).await?;
        
        let comp1 = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            1,
            serde_json::json!({"text": "Component 1"}),
        );
        let comp1_id = component_repo.create(&comp1).await?;
        
        let comp2 = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            2,
            serde_json::json!({"text": "Component 2"}),
        );
        let comp2_id = component_repo.create(&comp2).await?;
        
        let comp3 = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            3,
            serde_json::json!({"text": "Component 3"}),
        );
        let comp3_id = component_repo.create(&comp3).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        // Move component 0 after component 2 (moving down)
        let req = MoveComponentAfterRequest {
            component_id: comp0_id,
            after_component_id: comp2_id,
        };
        
        let result = service.move_component_after(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        let moved: ComponentInfo = serde_json::from_str(&result)?;
        assert_eq!(moved.position, 2); // Should be at position 2 (after old 2)
        
        // Verify new order: 1, 2, 0, 3
        assert_eq!(component_repo.find_by_id(comp1_id).await?.unwrap().position, 0);
        assert_eq!(component_repo.find_by_id(comp2_id).await?.unwrap().position, 1);
        assert_eq!(component_repo.find_by_id(comp0_id).await?.unwrap().position, 2);
        assert_eq!(component_repo.find_by_id(comp3_id).await?.unwrap().position, 3);
        Ok(())
    }

    #[sqlx::test]
    async fn test_move_component_after_up(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository, PageVersionRepository, ComponentRepository};
        
        // Create site
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        // Create page
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create draft version
        let version_repo = PageVersionRepository::new(pool.clone());
        let version = doxyde_core::models::PageVersion::new(root.id.unwrap(), 1, None);
        let version_id = version_repo.create(&version).await?;
        
        // Create components: 0, 1, 2, 3
        let component_repo = ComponentRepository::new(pool.clone());
        let comp0 = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            0,
            serde_json::json!({"text": "Component 0"}),
        );
        let comp0_id = component_repo.create(&comp0).await?;
        
        let comp1 = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            1,
            serde_json::json!({"text": "Component 1"}),
        );
        let comp1_id = component_repo.create(&comp1).await?;
        
        let comp2 = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            2,
            serde_json::json!({"text": "Component 2"}),
        );
        let comp2_id = component_repo.create(&comp2).await?;
        
        let comp3 = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            3,
            serde_json::json!({"text": "Component 3"}),
        );
        let comp3_id = component_repo.create(&comp3).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        // Move component 3 after component 0 (moving up)
        let req = MoveComponentAfterRequest {
            component_id: comp3_id,
            after_component_id: comp0_id,
        };
        
        let result = service.move_component_after(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        let moved: ComponentInfo = serde_json::from_str(&result)?;
        assert_eq!(moved.position, 1); // Should be at position 1 (after 0)
        
        // Verify new order: 0, 3, 1, 2
        assert_eq!(component_repo.find_by_id(comp0_id).await?.unwrap().position, 0);
        assert_eq!(component_repo.find_by_id(comp3_id).await?.unwrap().position, 1);
        assert_eq!(component_repo.find_by_id(comp1_id).await?.unwrap().position, 2);
        assert_eq!(component_repo.find_by_id(comp2_id).await?.unwrap().position, 3);
        Ok(())
    }

    #[sqlx::test]
    async fn test_move_component_after_self_fails(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        let service = DoxydeRmcpService::new(pool, 1);
        
        let req = MoveComponentAfterRequest {
            component_id: 100,
            after_component_id: 100,
        };
        
        let result = service.move_component_after(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("Cannot move component after itself"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_move_component_after_last(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        use doxyde_db::repositories::{SiteRepository, PageRepository, PageVersionRepository, ComponentRepository};
        
        // Create site
        let site_repo = SiteRepository::new(pool.clone());
        let site = doxyde_core::models::Site::new("test.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;
        
        // Create page
        let page_repo = PageRepository::new(pool.clone());
        let root = page_repo.get_root_page(site_id).await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;
        
        // Create draft version
        let version_repo = PageVersionRepository::new(pool.clone());
        let version = doxyde_core::models::PageVersion::new(root.id.unwrap(), 1, None);
        let version_id = version_repo.create(&version).await?;
        
        // Create components: 0, 1, 2
        let component_repo = ComponentRepository::new(pool.clone());
        let comp0 = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            0,
            serde_json::json!({"text": "Component 0"}),
        );
        let comp0_id = component_repo.create(&comp0).await?;
        
        let comp1 = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            1,
            serde_json::json!({"text": "Component 1"}),
        );
        let comp1_id = component_repo.create(&comp1).await?;
        
        let comp2 = doxyde_core::models::Component::new(
            version_id,
            "markdown".to_string(),
            2,
            serde_json::json!({"text": "Component 2"}),
        );
        let comp2_id = component_repo.create(&comp2).await?;
        
        let service = DoxydeRmcpService::new(pool.clone(), site_id);
        
        // Move component 0 after component 2 (to end)
        let req = MoveComponentAfterRequest {
            component_id: comp0_id,
            after_component_id: comp2_id,
        };
        
        let result = service.move_component_after(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        let moved: ComponentInfo = serde_json::from_str(&result)?;
        assert_eq!(moved.position, 2); // Should be at last position
        
        // Verify new order: 1, 2, 0
        assert_eq!(component_repo.find_by_id(comp1_id).await?.unwrap().position, 0);
        assert_eq!(component_repo.find_by_id(comp2_id).await?.unwrap().position, 1);
        assert_eq!(component_repo.find_by_id(comp0_id).await?.unwrap().position, 2);
        Ok(())
    }
}