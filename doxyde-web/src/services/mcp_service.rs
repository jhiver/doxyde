use anyhow::Result;
use doxyde_core::models::Page;
use doxyde_db::repositories::{ComponentRepository, PageRepository, PageVersionRepository};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct McpService {
    pool: SqlitePool,
    site_id: i64,
}

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

impl McpService {
    pub fn new(pool: SqlitePool, site_id: i64) -> Self {
        Self { pool, site_id }
    }

    /// List all pages in the site with hierarchy
    pub async fn list_pages(&self) -> Result<Vec<PageHierarchy>> {
        let page_repo = PageRepository::new(self.pool.clone());
        let pages = page_repo.list_by_site_id(self.site_id).await?;

        // Build hierarchy
        let mut hierarchy = Vec::new();
        let mut page_map = std::collections::HashMap::new();

        // First pass: create PageInfo for all pages
        for page in &pages {
            let info = PageInfo {
                id: page.id.unwrap(),
                slug: page.slug.clone(),
                title: page.title.clone(),
                path: self.build_page_path(&pages, page).await?,
                parent_id: page.parent_page_id,
                position: page.position,
                has_children: pages.iter().any(|p| p.parent_page_id == page.id),
                template: Some(page.template.clone()),
            };
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

    /// Get full page details by ID
    pub async fn get_page(&self, page_id: i64) -> Result<Page> {
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

    /// Find page by URL path
    pub async fn get_page_by_path(&self, path: &str) -> Result<Page> {
        let page_repo = PageRepository::new(self.pool.clone());
        let pages = page_repo.list_by_site_id(self.site_id).await?;

        // Split path into segments
        let segments: Vec<&str> = path
            .trim_start_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        if segments.is_empty() {
            // Root page - just return any page with no parent
            return pages
                .iter()
                .find(|p| p.parent_page_id.is_none())
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("No root page found"));
        }

        // Find root page first
        let root_page = pages
            .iter()
            .find(|p| p.parent_page_id.is_none())
            .ok_or_else(|| anyhow::anyhow!("No root page found"))?;

        // Navigate through hierarchy starting from root
        let mut current_parent = root_page.id;
        let mut current_page = None;

        for segment in segments {
            current_page = pages
                .iter()
                .find(|p| p.slug == segment && p.parent_page_id == current_parent);

            match current_page {
                Some(page) => current_parent = page.id,
                None => return Err(anyhow::anyhow!("Page not found at path: {}", path)),
            }
        }

        current_page
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Page not found"))
    }

    /// Get published content of a page
    pub async fn get_published_content(&self, page_id: i64) -> Result<Vec<ComponentInfo>> {
        // Verify page belongs to this site
        let _ = self.get_page(page_id).await?;

        let version_repo = PageVersionRepository::new(self.pool.clone());
        let component_repo = ComponentRepository::new(self.pool.clone());

        // Get published version
        let version = version_repo
            .get_published(page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("No published version found"))?;

        // Get components
        let components = component_repo
            .list_by_page_version(version.id.unwrap())
            .await?;

        let mut result = Vec::new();
        for component in components {
            let info = ComponentInfo {
                id: component.id.unwrap(),
                component_type: component.component_type.clone(),
                position: component.position,
                template: component.template.clone(),
                title: component.title.clone(),
                content: component.content.clone(),
                created_at: component.created_at.to_rfc3339(),
                updated_at: component.updated_at.to_rfc3339(),
            };
            result.push(info);
        }

        Ok(result)
    }

    /// Get draft content of a page (if exists)
    pub async fn get_draft_content(&self, page_id: i64) -> Result<Option<Vec<ComponentInfo>>> {
        // Verify page belongs to this site
        let _ = self.get_page(page_id).await?;

        let version_repo = PageVersionRepository::new(self.pool.clone());
        let component_repo = ComponentRepository::new(self.pool.clone());

        // Get draft version
        let version = match version_repo.get_draft(page_id).await? {
            Some(v) => v,
            None => return Ok(None),
        };

        // Get components
        let components = component_repo
            .list_by_page_version(version.id.unwrap())
            .await?;

        let mut result = Vec::new();
        for component in components {
            let info = ComponentInfo {
                id: component.id.unwrap(),
                component_type: component.component_type.clone(),
                position: component.position,
                template: component.template.clone(),
                title: component.title.clone(),
                content: component.content.clone(),
                created_at: component.created_at.to_rfc3339(),
                updated_at: component.updated_at.to_rfc3339(),
            };
            result.push(info);
        }

        Ok(Some(result))
    }

    /// Search pages by title or content
    pub async fn search_pages(&self, query: &str) -> Result<Vec<PageInfo>> {
        let page_repo = PageRepository::new(self.pool.clone());
        let pages = page_repo.list_by_site_id(self.site_id).await?;

        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        for page in pages.iter() {
            // Search in title
            if page.title.to_lowercase().contains(&query_lower) {
                results.push(PageInfo {
                    id: page.id.unwrap(),
                    slug: page.slug.clone(),
                    title: page.title.clone(),
                    path: self.build_page_path(&pages, page).await?,
                    parent_id: page.parent_page_id,
                    position: page.position,
                    has_children: pages.iter().any(|p| p.parent_page_id == page.id),
                    template: Some(page.template.clone()),
                });
                continue;
            }

            // Search in content
            if let Ok(components) = self.get_published_content(page.id.unwrap()).await {
                for component in components {
                    // Check in title
                    if let Some(title) = &component.title {
                        if title.to_lowercase().contains(&query_lower) {
                            results.push(PageInfo {
                                id: page.id.unwrap(),
                                slug: page.slug.clone(),
                                title: page.title.clone(),
                                path: self.build_page_path(&pages, page).await?,
                                parent_id: page.parent_page_id,
                                position: page.position,
                                has_children: pages.iter().any(|p| p.parent_page_id == page.id),
                                template: Some(page.template.clone()),
                            });
                            break;
                        }
                    }

                    // Check in content JSON
                    let content_str = component.content.to_string().to_lowercase();
                    if content_str.contains(&query_lower) {
                        results.push(PageInfo {
                            id: page.id.unwrap(),
                            slug: page.slug.clone(),
                            title: page.title.clone(),
                            path: self.build_page_path(&pages, page).await?,
                            parent_id: page.parent_page_id,
                            position: page.position,
                            has_children: pages.iter().any(|p| p.parent_page_id == page.id),
                            template: Some(page.template.clone()),
                        });
                        break;
                    }
                }
            }
        }

        Ok(results)
    }

    // Helper methods

    async fn build_page_path(&self, all_pages: &[Page], page: &Page) -> Result<String> {
        // Special case for root page
        if page.parent_page_id.is_none() {
            return Ok("/".to_string());
        }

        let mut path_segments = vec![page.slug.clone()];
        let mut current_parent = page.parent_page_id;

        while let Some(parent_id) = current_parent {
            if let Some(parent) = all_pages.iter().find(|p| p.id == Some(parent_id)) {
                // Don't include root page slug in path
                if parent.parent_page_id.is_some() {
                    path_segments.insert(0, parent.slug.clone());
                }
                current_parent = parent.parent_page_id;
            } else {
                break;
            }
        }

        Ok(format!("/{}", path_segments.join("/")))
    }

    fn build_hierarchy_node(
        &self,
        page_map: &std::collections::HashMap<i64, (PageInfo, Vec<i64>)>,
        id: i64,
    ) -> Option<PageHierarchy> {
        if let Some((info, child_ids)) = page_map.get(&id) {
            let mut children = Vec::new();
            for child_id in child_ids {
                if let Some(child_node) = self.build_hierarchy_node(page_map, *child_id) {
                    children.push(child_node);
                }
            }

            // Sort children by position
            children.sort_by(|a, b| a.page.position.cmp(&b.page.position));

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
    use crate::test_helpers::{create_test_app_state, create_test_site};

    async fn create_test_service() -> Result<(McpService, i64)> {
        let state = create_test_app_state().await?;
        let site = create_test_site(&state.db, "test.com", "Test Site").await?;
        let service = McpService::new(state.db, site.id.unwrap());

        Ok((service, site.id.unwrap()))
    }

    #[tokio::test]
    async fn test_list_pages_with_root() -> Result<()> {
        let (service, _) = create_test_service().await?;
        let pages = service.list_pages().await?;
        // Site creation creates a root page
        assert_eq!(pages.len(), 1);
        assert_eq!(pages[0].page.slug, "home");
        assert!(pages[0].children.is_empty());
        Ok(())
    }

    #[tokio::test]
    async fn test_get_page_not_found() -> Result<()> {
        let (service, _) = create_test_service().await?;
        let result = service.get_page(999).await;
        assert!(result.is_err());
        Ok(())
    }
}
