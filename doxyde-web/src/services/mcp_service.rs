use anyhow::Result;
use doxyde_core::models::{Page, Component};
use doxyde_db::repositories::{ComponentRepository, PageRepository, PageVersionRepository};
use serde::{Deserialize, Serialize};
use serde_json::json;
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

#[derive(Debug, Serialize, Deserialize)]
pub struct DraftInfo {
    pub page_id: i64,
    pub version_id: i64,
    pub version_number: i32,
    pub created_by: Option<String>,
    pub is_published: bool,
    pub component_count: i32,
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

        Ok(self.components_to_info(components))
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

        Ok(Some(self.components_to_info(components)))
    }

    /// Search pages by title or content
    pub async fn search_pages(&self, query: &str) -> Result<Vec<PageInfo>> {
        let page_repo = PageRepository::new(self.pool.clone());
        let pages = page_repo.list_by_site_id(self.site_id).await?;

        let query_lower = query.to_lowercase();
        let mut results = Vec::new();
        let mut found_pages = std::collections::HashSet::new();

        for page in pages.iter() {
            // Skip if already found
            if found_pages.contains(&page.id.unwrap()) {
                continue;
            }

            // Search in title
            if page.title.to_lowercase().contains(&query_lower) {
                results.push(self.page_to_info(&pages, page).await?);
                found_pages.insert(page.id.unwrap());
                continue;
            }

            // Search in content
            if self
                .page_content_matches(page.id.unwrap(), &query_lower)
                .await?
            {
                results.push(self.page_to_info(&pages, page).await?);
                found_pages.insert(page.id.unwrap());
            }
        }

        Ok(results)
    }

    /// Create a new page
    pub async fn create_page(
        &self,
        parent_page_id: Option<i64>,
        slug: Option<String>,
        title: String,
        template: Option<String>,
    ) -> Result<PageInfo> {
        let page_repo = PageRepository::new(self.pool.clone());

        // If parent_page_id is provided, verify it belongs to this site
        if let Some(parent_id) = parent_page_id {
            let parent = page_repo
                .find_by_id(parent_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Parent page not found"))?;
            if parent.site_id != self.site_id {
                return Err(anyhow::anyhow!("Parent page does not belong to this site"));
            }
        }

        // Calculate position for new page
        let position = self
            .calculate_page_position(&page_repo, parent_page_id)
            .await?;

        // Create the page object
        let mut new_page = match (parent_page_id, slug) {
            (Some(parent_id), Some(slug)) => Page::new_with_parent(self.site_id, parent_id, slug, title),
            (Some(parent_id), None) => Page::new_with_parent_and_title(self.site_id, parent_id, title),
            (None, Some(slug)) => Page::new(self.site_id, slug, title),
            (None, None) => Page::new_with_title(self.site_id, title),
        };

        // Set template and position
        new_page.template = template.unwrap_or_else(|| "default".to_string());
        new_page.position = position;

        // Create the page with auto-generated unique slug if needed
        let page_id = page_repo.create_with_auto_slug(&mut new_page).await?;

        // Retrieve the created page
        let created_page = page_repo
            .find_by_id(page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to retrieve created page"))?;

        // Get all pages to build path
        let all_pages = page_repo.list_by_site_id(self.site_id).await?;

        // Return page info
        Ok(PageInfo {
            id: created_page.id.unwrap(),
            slug: created_page.slug.clone(),
            title: created_page.title.clone(),
            path: self.build_page_path(&all_pages, &created_page).await?,
            parent_id: created_page.parent_page_id,
            position: created_page.position,
            has_children: false, // New page has no children
            template: Some(created_page.template.clone()),
        })
    }

    /// Update an existing page
    pub async fn update_page(
        &self,
        page_id: i64,
        title: Option<String>,
        slug: Option<String>,
        template: Option<String>,
    ) -> Result<PageInfo> {
        let page_repo = PageRepository::new(self.pool.clone());

        // Get the existing page
        let mut page = page_repo
            .find_by_id(page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page not found"))?;

        // Verify page belongs to this site
        if page.site_id != self.site_id {
            return Err(anyhow::anyhow!("Page does not belong to this site"));
        }

        // Update fields if provided
        if let Some(new_title) = title {
            page.title = new_title;
        }

        if let Some(new_slug) = slug {
            page.slug = new_slug;
        }

        if let Some(new_template) = template {
            page.template = new_template;
        }

        // Update timestamp
        page.updated_at = chrono::Utc::now();

        // Validate the updated page
        page.is_valid().map_err(|e| anyhow::anyhow!(e))?;

        // Save the updates
        page_repo.update(&page).await?;

        // Get all pages to build info
        let all_pages = page_repo.list_by_site_id(self.site_id).await?;

        // Return updated page info
        self.page_to_info(&all_pages, &page).await
    }

    /// Delete a page (with safety checks)
    pub async fn delete_page(&self, page_id: i64) -> Result<()> {
        let page_repo = PageRepository::new(self.pool.clone());

        // Verify page exists and belongs to this site
        let _page = self.get_page(page_id).await?;

        // Use the repository's delete method which handles all safety checks
        page_repo.delete(page_id).await?;

        Ok(())
    }

    /// Move a page to a new parent or reorder within siblings
    pub async fn move_page(&self, page_id: i64, new_parent_id: i64, position: Option<i32>) -> Result<PageInfo> {
        let page_repo = PageRepository::new(self.pool.clone());

        // Verify both pages exist and belong to this site
        let _page = self.get_page(page_id).await?;
        let _new_parent = self.get_page(new_parent_id).await?;

        // Use the repository's move_page method which handles all safety checks
        page_repo.move_page(page_id, new_parent_id).await?;

        // If position is specified, update it
        if let Some(new_position) = position {
            // Update position within the new parent
            let mut updated_page = page_repo
                .find_by_id(page_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Page not found after move"))?;
            
            updated_page.position = new_position;
            page_repo.update(&updated_page).await?;
        }

        // Get all pages to build info
        let all_pages = page_repo.list_by_site_id(self.site_id).await?;
        let moved_page = page_repo
            .find_by_id(page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page not found after move"))?;

        // Return updated page info
        self.page_to_info(&all_pages, &moved_page).await
    }

    /// Create a markdown component on a page
    pub async fn create_component_markdown(
        &self,
        page_id: i64,
        content_text: String,
        title: Option<String>,
        template: Option<String>,
    ) -> Result<ComponentInfo> {
        // Verify page exists and belongs to this site
        let _page = self.get_page(page_id).await?;

        let component_repo = ComponentRepository::new(self.pool.clone());

        // Get or create draft version
        let draft = crate::draft::get_or_create_draft(&self.pool, page_id, None).await?;
        let draft_id = draft.id.ok_or_else(|| anyhow::anyhow!("Draft ID not found"))?;

        // Get current components to determine position
        let existing_components = component_repo.list_by_page_version(draft_id).await?;
        let next_position = existing_components.len() as i32;

        // Create the markdown component
        let content = json!({
            "text": content_text
        });

        let mut component = Component::new(
            draft_id,
            "markdown".to_string(),
            next_position,
            content,
        );
        
        component.title = title;
        component.template = template.unwrap_or_else(|| "default".to_string());

        // Validate the component
        component.is_valid().map_err(|e| anyhow::anyhow!(e))?;

        // Create the component
        let component_id = component_repo.create(&component).await?;

        // Return component info
        Ok(ComponentInfo {
            id: component_id,
            component_type: component.component_type,
            position: component.position,
            template: component.template,
            title: component.title,
            content: component.content,
            created_at: component.created_at.to_rfc3339(),
            updated_at: component.updated_at.to_rfc3339(),
        })
    }

    /// Update a markdown component
    pub async fn update_component_markdown(
        &self,
        component_id: i64,
        content_text: String,
        title: Option<String>,
        template: Option<String>,
    ) -> Result<ComponentInfo> {
        let component_repo = ComponentRepository::new(self.pool.clone());

        // Get the existing component
        let mut component = component_repo
            .find_by_id(component_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Component not found"))?;

        // Verify it's a markdown component
        if component.component_type != "markdown" {
            return Err(anyhow::anyhow!("Component is not a markdown component"));
        }

        // Verify the component belongs to a page in this site
        let version_repo = PageVersionRepository::new(self.pool.clone());
        let version = version_repo
            .find_by_id(component.page_version_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page version not found"))?;

        let _page = self.get_page(version.page_id).await?;

        // Update the component
        component.content = json!({
            "text": content_text
        });
        
        if let Some(new_title) = title {
            component.title = Some(new_title);
        }
        
        if let Some(new_template) = template {
            component.template = new_template;
        }

        component.updated_at = chrono::Utc::now();

        // Validate and update
        component.is_valid().map_err(|e| anyhow::anyhow!(e))?;
        component_repo.update(&component).await?;

        // Return updated component info
        Ok(ComponentInfo {
            id: component_id,
            component_type: component.component_type,
            position: component.position,
            template: component.template,
            title: component.title,
            content: component.content,
            created_at: component.created_at.to_rfc3339(),
            updated_at: component.updated_at.to_rfc3339(),
        })
    }

    /// Delete a component
    pub async fn delete_component(&self, component_id: i64) -> Result<()> {
        let component_repo = ComponentRepository::new(self.pool.clone());

        // Get the component to verify it exists and get its page version
        let component = component_repo
            .find_by_id(component_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Component not found"))?;

        // Verify the component belongs to a page in this site
        let version_repo = PageVersionRepository::new(self.pool.clone());
        let version = version_repo
            .find_by_id(component.page_version_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page version not found"))?;

        let _page = self.get_page(version.page_id).await?;

        // Delete the component
        component_repo.delete(component_id).await?;

        Ok(())
    }

    /// List all components for a page
    pub async fn list_components(&self, page_id: i64) -> Result<Vec<ComponentInfo>> {
        // Verify page exists and belongs to this site
        let _page = self.get_page(page_id).await?;

        let version_repo = PageVersionRepository::new(self.pool.clone());
        let component_repo = ComponentRepository::new(self.pool.clone());

        // Get the draft version if it exists, otherwise get published
        let version = if let Some(draft) = version_repo.get_draft(page_id).await? {
            draft
        } else if let Some(published) = version_repo.get_published(page_id).await? {
            published
        } else {
            return Ok(vec![]); // No versions exist
        };

        let version_id = version.id.ok_or_else(|| anyhow::anyhow!("Version ID not found"))?;

        // Get components
        let components = component_repo.list_by_page_version(version_id).await?;

        Ok(self.components_to_info(components))
    }

    /// Get a specific component
    pub async fn get_component(&self, component_id: i64) -> Result<ComponentInfo> {
        let component_repo = ComponentRepository::new(self.pool.clone());

        // Get the component
        let component = component_repo
            .find_by_id(component_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Component not found"))?;

        // Verify the component belongs to a page in this site
        let version_repo = PageVersionRepository::new(self.pool.clone());
        let version = version_repo
            .find_by_id(component.page_version_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page version not found"))?;

        let _page = self.get_page(version.page_id).await?;

        // Return component info
        Ok(ComponentInfo {
            id: component_id,
            component_type: component.component_type,
            position: component.position,
            template: component.template,
            title: component.title,
            content: component.content,
            created_at: component.created_at.to_rfc3339(),
            updated_at: component.updated_at.to_rfc3339(),
        })
    }

    /// Publish the draft version of a page
    pub async fn publish_draft(&self, page_id: i64) -> Result<DraftInfo> {
        // Verify page exists and belongs to this site
        let _page = self.get_page(page_id).await?;
        
        let version_repo = PageVersionRepository::new(self.pool.clone());
        
        // Get the draft
        let draft = version_repo
            .get_draft(page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("No draft version found for this page"))?;
        
        let draft_id = draft.id.ok_or_else(|| anyhow::anyhow!("Draft ID not found"))?;
        
        // Publish it
        crate::draft::publish_draft(&self.pool, page_id).await?;
        
        // Get component count for info
        let component_repo = ComponentRepository::new(self.pool.clone());
        let components = component_repo.list_by_page_version(draft_id).await?;
        
        Ok(DraftInfo {
            page_id,
            version_id: draft_id,
            version_number: draft.version_number,
            created_by: draft.created_by,
            is_published: true,
            component_count: components.len() as i32,
        })
    }
    
    /// Discard the draft version of a page
    pub async fn discard_draft(&self, page_id: i64) -> Result<()> {
        // Verify page exists and belongs to this site
        let _page = self.get_page(page_id).await?;
        
        let version_repo = PageVersionRepository::new(self.pool.clone());
        
        // Check if there's a draft
        let draft = version_repo.get_draft(page_id).await?;
        if draft.is_none() {
            return Err(anyhow::anyhow!("No draft version found for this page"));
        }
        
        // Delete it
        crate::draft::delete_draft_if_exists(&self.pool, page_id).await?;
        
        Ok(())
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

    async fn calculate_page_position(
        &self,
        page_repo: &PageRepository,
        parent_page_id: Option<i64>,
    ) -> Result<i32> {
        let pages = page_repo.list_by_site_id(self.site_id).await?;

        let position = if let Some(parent_id) = parent_page_id {
            // Get max position among siblings
            pages
                .iter()
                .filter(|p| p.parent_page_id == Some(parent_id))
                .map(|p| p.position)
                .max()
                .unwrap_or(0)
                + 1
        } else {
            // Get max position among root pages
            pages
                .iter()
                .filter(|p| p.parent_page_id.is_none())
                .map(|p| p.position)
                .max()
                .unwrap_or(0)
                + 1
        };

        Ok(position)
    }

    async fn page_to_info(&self, all_pages: &[Page], page: &Page) -> Result<PageInfo> {
        Ok(PageInfo {
            id: page.id.unwrap(),
            slug: page.slug.clone(),
            title: page.title.clone(),
            path: self.build_page_path(all_pages, page).await?,
            parent_id: page.parent_page_id,
            position: page.position,
            has_children: all_pages.iter().any(|p| p.parent_page_id == page.id),
            template: Some(page.template.clone()),
        })
    }

    async fn page_content_matches(&self, page_id: i64, query_lower: &str) -> Result<bool> {
        if let Ok(components) = self.get_published_content(page_id).await {
            for component in components {
                // Check in component title
                if let Some(title) = &component.title {
                    if title.to_lowercase().contains(query_lower) {
                        return Ok(true);
                    }
                }

                // Check in content JSON
                let content_str = component.content.to_string().to_lowercase();
                if content_str.contains(query_lower) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn components_to_info(
        &self,
        components: Vec<doxyde_core::models::Component>,
    ) -> Vec<ComponentInfo> {
        components
            .into_iter()
            .map(|component| ComponentInfo {
                id: component.id.unwrap(),
                component_type: component.component_type,
                position: component.position,
                template: component.template,
                title: component.title,
                content: component.content,
                created_at: component.created_at.to_rfc3339(),
                updated_at: component.updated_at.to_rfc3339(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{create_test_app_state, create_test_site};
    use serde_json::json;

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

    #[tokio::test]
    async fn test_create_page_as_child_of_root() -> Result<()> {
        let (service, _) = create_test_service().await?;

        // Get the root page
        let pages = service.list_pages().await?;
        let root_page_id = pages[0].page.id;

        // Create a page as child of root
        let page_info = service
            .create_page(
                Some(root_page_id),
                "about".to_string(),
                "About Us".to_string(),
                Some("default".to_string()),
            )
            .await?;

        assert_eq!(page_info.slug, "about");
        assert_eq!(page_info.title, "About Us");
        assert_eq!(page_info.path, "/about");
        assert_eq!(page_info.parent_id, Some(root_page_id));
        // Position depends on whether there are already child pages
        assert!(page_info.position >= 0);
        assert!(!page_info.has_children);
        assert_eq!(page_info.template, Some("default".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_create_page_with_parent() -> Result<()> {
        let (service, _site_id) = create_test_service().await?;

        // Get the root page to use as parent
        let pages = service.list_pages().await?;
        let root_page_id = pages[0].page.id;

        // Create a child page
        let page_info = service
            .create_page(
                Some(root_page_id),
                "team".to_string(),
                "Our Team".to_string(),
                Some("full_width".to_string()),
            )
            .await?;

        assert_eq!(page_info.slug, "team");
        assert_eq!(page_info.title, "Our Team");
        assert_eq!(page_info.path, "/team");
        assert_eq!(page_info.parent_id, Some(root_page_id));
        assert!(page_info.position >= 0); // Position should be valid
        assert!(!page_info.has_children);
        assert_eq!(page_info.template, Some("full_width".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_create_page_validation_errors() -> Result<()> {
        let (service, _) = create_test_service().await?;

        // Get the root page to use as parent
        let pages = service.list_pages().await?;
        let root_page_id = pages[0].page.id;

        // Test empty slug
        let result = service
            .create_page(
                Some(root_page_id),
                "".to_string(),
                "Title".to_string(),
                None,
            )
            .await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Slug cannot be empty"));

        // Test empty title
        let result = service
            .create_page(
                Some(root_page_id),
                "valid-slug".to_string(),
                "".to_string(),
                None,
            )
            .await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Title cannot be empty"));

        // Test invalid slug characters (space in slug)
        let result = service
            .create_page(
                Some(root_page_id),
                "invalid slug!".to_string(),
                "Title".to_string(),
                None,
            )
            .await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Slug cannot contain spaces"));

        Ok(())
    }

    #[tokio::test]
    async fn test_create_root_page_not_allowed() -> Result<()> {
        let (service, _) = create_test_service().await?;

        // Try to create a root page (without parent)
        let result = service
            .create_page(
                None,
                "new-root".to_string(),
                "New Root Page".to_string(),
                None,
            )
            .await;

        assert!(result.is_err());
        // The error will come from PageRepository

        Ok(())
    }

    #[tokio::test]
    async fn test_create_page_with_invalid_parent() -> Result<()> {
        let (service, _) = create_test_service().await?;

        // Try to create page with non-existent parent
        let result = service
            .create_page(
                Some(9999),
                "orphan".to_string(),
                "Orphan Page".to_string(),
                None,
            )
            .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Parent page not found"));

        Ok(())
    }

    #[tokio::test]
    async fn test_create_page_default_template() -> Result<()> {
        let (service, _) = create_test_service().await?;

        // Get the root page
        let pages = service.list_pages().await?;
        let root_page_id = pages[0].page.id;

        // Create page without specifying template
        let page_info = service
            .create_page(
                Some(root_page_id),
                "contact".to_string(),
                "Contact Us".to_string(),
                None,
            )
            .await?;

        // Should default to "default" template
        assert_eq!(page_info.template, Some("default".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_calculate_page_position() -> Result<()> {
        let (service, _) = create_test_service().await?;
        let page_repo = PageRepository::new(service.pool.clone());

        // Test position for root page (should be based on existing root pages)
        let position = service.calculate_page_position(&page_repo, None).await?;
        assert!(position >= 0);

        // Get root page for parent tests
        let pages = service.list_pages().await?;
        let root_page_id = pages[0].page.id;

        // Test position for child page
        let position = service
            .calculate_page_position(&page_repo, Some(root_page_id))
            .await?;
        assert!(position >= 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_page_to_info() -> Result<()> {
        let (service, site_id) = create_test_service().await?;
        let page_repo = PageRepository::new(service.pool.clone());
        let pages = page_repo.list_by_site_id(site_id).await?;

        assert!(!pages.is_empty());
        let page = &pages[0];

        let info = service.page_to_info(&pages, page).await?;

        assert_eq!(info.id, page.id.unwrap());
        assert_eq!(info.slug, page.slug);
        assert_eq!(info.title, page.title);
        assert_eq!(info.parent_id, page.parent_page_id);
        assert_eq!(info.position, page.position);
        assert_eq!(info.template, Some(page.template.clone()));
        assert_eq!(info.path, "/"); // Root page should have "/" path

        Ok(())
    }

    #[tokio::test]
    async fn test_page_content_matches() -> Result<()> {
        let (service, _) = create_test_service().await?;

        // Get a page
        let pages = service.list_pages().await?;
        let page_id = pages[0].page.id;

        // Test with non-matching query
        let matches = service
            .page_content_matches(page_id, "nonexistentcontent")
            .await?;
        assert!(!matches);

        // Note: We can't test positive matches without first adding content to the page
        // This would require creating components, which is beyond the scope of this unit test

        Ok(())
    }

    #[tokio::test]
    async fn test_components_to_info() -> Result<()> {
        use chrono::Utc;
        use doxyde_core::models::Component;

        let (service, _) = create_test_service().await?;

        let components = vec![
            Component {
                id: Some(1),
                page_version_id: 1,
                component_type: "text".to_string(),
                position: 0,
                template: "default".to_string(),
                title: Some("Test Title".to_string()),
                content: json!({"text": "Test content"}),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            Component {
                id: Some(2),
                page_version_id: 1,
                component_type: "image".to_string(),
                position: 1,
                template: "full_width".to_string(),
                title: None,
                content: json!({"url": "test.jpg"}),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        ];

        let infos = service.components_to_info(components.clone());

        assert_eq!(infos.len(), 2);

        assert_eq!(infos[0].id, 1);
        assert_eq!(infos[0].component_type, "text");
        assert_eq!(infos[0].position, 0);
        assert_eq!(infos[0].template, "default");
        assert_eq!(infos[0].title, Some("Test Title".to_string()));
        assert_eq!(infos[0].content, json!({"text": "Test content"}));

        assert_eq!(infos[1].id, 2);
        assert_eq!(infos[1].component_type, "image");
        assert_eq!(infos[1].position, 1);
        assert_eq!(infos[1].template, "full_width");
        assert_eq!(infos[1].title, None);
        assert_eq!(infos[1].content, json!({"url": "test.jpg"}));

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_page_success() -> Result<()> {
        let (service, _) = create_test_service().await?;

        // Get the root page
        let pages = service.list_pages().await?;
        let root_page_id = pages[0].page.id;

        // Create a child page to delete
        let page_info = service
            .create_page(
                Some(root_page_id),
                "delete-me".to_string(),
                "Delete Me".to_string(),
                None,
            )
            .await?;

        // Delete the page
        service.delete_page(page_info.id).await?;

        // Verify it's deleted - get_page should fail
        let result = service.get_page(page_info.id).await;
        assert!(result.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_page_wrong_site() -> Result<()> {
        let state = create_test_app_state().await?;
        let other_site = create_test_site(&state.db, "other.com", "Other Site").await?;
        let service = McpService::new(state.db.clone(), other_site.id.unwrap());

        // Create a page in a different site
        let main_site = create_test_site(&state.db, "main.com", "Main Site").await?;
        let main_service = McpService::new(state.db.clone(), main_site.id.unwrap());
        
        // Get root page of main site
        let pages = main_service.list_pages().await?;
        let root_page_id = pages[0].page.id;
        
        // Create a page in main site
        let page_info = main_service
            .create_page(
                Some(root_page_id),
                "test-page".to_string(),
                "Test Page".to_string(),
                None,
            )
            .await?;

        // Try to delete it from other site - should fail
        let result = service.delete_page(page_info.id).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("does not belong to this site"));

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_nonexistent_page() -> Result<()> {
        let (service, _) = create_test_service().await?;

        // Try to delete a non-existent page
        let result = service.delete_page(9999).await;
        assert!(result.is_err());

        Ok(())
    }

    #[tokio::test]
    async fn test_move_page_success() -> Result<()> {
        let (service, _) = create_test_service().await?;

        // Get the root page
        let pages = service.list_pages().await?;
        let root_page_id = pages[0].page.id;

        // Create two pages under root
        let page1_info = service
            .create_page(
                Some(root_page_id),
                "page1".to_string(),
                "Page 1".to_string(),
                None,
            )
            .await?;

        let page2_info = service
            .create_page(
                Some(root_page_id),
                "page2".to_string(),
                "Page 2".to_string(),
                None,
            )
            .await?;

        // Move page1 under page2
        let moved_page = service.move_page(page1_info.id, page2_info.id, None).await?;

        assert_eq!(moved_page.id, page1_info.id);
        assert_eq!(moved_page.parent_id, Some(page2_info.id));
        assert_eq!(moved_page.path, "/page2/page1");

        Ok(())
    }

    #[tokio::test]
    async fn test_move_page_with_position() -> Result<()> {
        let (service, _) = create_test_service().await?;

        // Get the root page
        let pages = service.list_pages().await?;
        let root_page_id = pages[0].page.id;

        // Create parent page
        let parent_info = service
            .create_page(
                Some(root_page_id),
                "parent".to_string(),
                "Parent".to_string(),
                None,
            )
            .await?;

        // Create three child pages
        let _child1_info = service
            .create_page(
                Some(parent_info.id),
                "child1".to_string(),
                "Child 1".to_string(),
                None,
            )
            .await?;

        let _child2_info = service
            .create_page(
                Some(parent_info.id),
                "child2".to_string(),
                "Child 2".to_string(),
                None,
            )
            .await?;

        let _child3_info = service
            .create_page(
                Some(parent_info.id),
                "child3".to_string(),
                "Child 3".to_string(),
                None,
            )
            .await?;

        // Create another page under root
        let other_page_info = service
            .create_page(
                Some(root_page_id),
                "other".to_string(),
                "Other".to_string(),
                None,
            )
            .await?;

        // Move other page to parent with position 1 (between child1 and child2)
        let moved_page = service
            .move_page(other_page_info.id, parent_info.id, Some(1))
            .await?;

        assert_eq!(moved_page.id, other_page_info.id);
        assert_eq!(moved_page.parent_id, Some(parent_info.id));
        assert_eq!(moved_page.position, 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_move_page_wrong_site() -> Result<()> {
        let state = create_test_app_state().await?;
        let site1 = create_test_site(&state.db, "site1.com", "Site 1").await?;
        let site2 = create_test_site(&state.db, "site2.com", "Site 2").await?;
        
        let service1 = McpService::new(state.db.clone(), site1.id.unwrap());
        let service2 = McpService::new(state.db.clone(), site2.id.unwrap());

        // Get root pages
        let pages1 = service1.list_pages().await?;
        let root1_id = pages1[0].page.id;

        let pages2 = service2.list_pages().await?;
        let root2_id = pages2[0].page.id;

        // Create page in site1
        let page_info = service1
            .create_page(
                Some(root1_id),
                "test-page".to_string(),
                "Test Page".to_string(),
                None,
            )
            .await?;

        // Try to move page from site1 to site2 - should fail
        let result = service1.move_page(page_info.id, root2_id, None).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("does not belong to this site"));

        Ok(())
    }
}
