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
    handler::server::{router::tool::ToolRouter, ServerHandler},
    model::{ServerCapabilities, ServerInfo},
    tool, tool_router, tool_handler,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tracing::info;
use anyhow::Result;
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
}

#[tool_handler]
impl ServerHandler for DoxydeRmcpService {
    fn get_info(&self) -> ServerInfo {
        info!("Getting server info");
        ServerInfo {
            protocol_version: rmcp::model::ProtocolVersion::default(),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
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
}