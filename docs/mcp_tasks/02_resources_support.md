# Task 02: Resources Support Implementation

## Overview
Implement resources/list and resources/read functionality. Resources allow MCP clients to browse and read page content.

## Implementation Steps

### 1. Add Resources Trait Implementation

The rmcp library requires implementing the `ResourcesProvider` trait. Add this to `doxyde-shared/src/mcp/service.rs`:

```rust
use rmcp::handler::server::resources::{Resource, ResourcesProvider, ReadResourceResult};
use async_trait::async_trait;

#[async_trait]
impl ResourcesProvider for DoxydeRmcpService {
    async fn list_resources(&self) -> Result<Vec<Resource>> {
        use doxyde_db::repositories::PageRepository;
        use std::collections::VecDeque;

        // Get pages in breadth-first order with 100 page limit
        let page_repo = PageRepository::new(self.pool.clone());
        let all_pages = page_repo.list_by_site_id(self.site_id).await?;

        // Create a map of page_id to children
        let mut children_map: std::collections::HashMap<Option<i64>, Vec<&doxyde_core::models::Page>> =
            std::collections::HashMap::new();

        for page in &all_pages {
            children_map
                .entry(page.parent_page_id)
                .or_insert_with(Vec::new)
                .push(page);
        }

        // Sort children by position
        for children in children_map.values_mut() {
            children.sort_by_key(|p| p.position);
        }

        // Breadth-first traversal
        let mut resources = Vec::new();
        let mut queue = VecDeque::new();

        // Start with root pages (parent_page_id = None)
        if let Some(roots) = children_map.get(&None) {
            for root in roots {
                queue.push_back(root);
            }
        }

        // Process queue in breadth-first order (limit to 100)
        while let Some(page) = queue.pop_front() {
            if resources.len() >= 100 {
                break;
            }

            let page_type = if page.parent_page_id.is_none() {
                "Homepage"
            } else {
                "Page"
            };

            let description = format!(
                "{} • Template: {} • Path: {}",
                page_type,
                page.template.as_deref().unwrap_or("default"),
                self.build_page_path(&all_pages, page).await?
            );

            resources.push(Resource {
                uri: format!("page://{}", page.id.unwrap()),
                name: page.title.clone(),
                description: Some(description),
                mime_type: Some("text/html".to_string()),
            });

            // Add children to queue
            if let Some(children) = children_map.get(&page.id) {
                for child in children {
                    queue.push_back(child);
                }
            }
        }

        Ok(resources)
    }

    async fn read_resource(&self, uri: &str) -> Result<ReadResourceResult> {
        use doxyde_db::repositories::{ComponentRepository, PageRepository, PageVersionRepository};

        // Parse page ID from URI (format: "page://123")
        let page_id = uri
            .strip_prefix("page://")
            .and_then(|id_str| id_str.parse::<i64>().ok())
            .ok_or_else(|| anyhow::anyhow!("Invalid URI format. Expected: page://[id]"))?;

        // Get the page
        let page_repo = PageRepository::new(self.pool.clone());
        let page = page_repo
            .find_by_id(page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page not found: {}", page_id))?;

        // Verify page belongs to this site
        if page.site_id != self.site_id {
            return Err(anyhow::anyhow!("Page does not belong to this site"));
        }

        // Get the published version's components
        let version_repo = PageVersionRepository::new(self.pool.clone());
        let version = match version_repo.get_published(page_id).await? {
            Some(v) => v,
            None => {
                // No published version, return empty content
                return Ok(ReadResourceResult {
                    contents: vec![rmcp::model::ResourceContent {
                        uri: uri.to_string(),
                        mime_type: Some("text/html".to_string()),
                        text: Some(format!(
                            "<h1>{}</h1>\n<p>This page has no published content yet.</p>",
                            page.title
                        )),
                        blob: None,
                    }],
                });
            }
        };

        // Get components for the published version
        let component_repo = ComponentRepository::new(self.pool.clone());
        let components = component_repo
            .list_by_page_version(version.id.unwrap())
            .await?;

        // Build HTML content
        let mut html = format!("<h1>{}</h1>\n", page.title);

        // Add metadata if present
        if page.description.is_some() || page.keywords.is_some() {
            html.push_str("<div class=\"page-metadata\">\n");
            if let Some(desc) = &page.description {
                html.push_str(&format!("  <p class=\"description\">{}</p>\n", desc));
            }
            if let Some(keywords) = &page.keywords {
                html.push_str(&format!("  <p class=\"keywords\">Keywords: {}</p>\n", keywords));
            }
            html.push_str("</div>\n\n");
        }

        // Add components
        for component in components {
            match component.component_type.as_str() {
                "text" | "markdown" => {
                    if let Some(content_str) = component.content.as_str() {
                        html.push_str(&format!(
                            "<div class=\"component component-{}\">\n",
                            component.component_type
                        ));
                        if let Some(title) = component.title {
                            html.push_str(&format!("  <h2>{}</h2>\n", title));
                        }
                        html.push_str(&format!("  {}\n", content_str));
                        html.push_str("</div>\n\n");
                    }
                }
                _ => {
                    // Handle other component types as needed
                    html.push_str(&format!(
                        "<div class=\"component component-{}\">\n",
                        component.component_type
                    ));
                    if let Some(title) = component.title {
                        html.push_str(&format!("  <h2>{}</h2>\n", title));
                    }
                    html.push_str(&format!("  <p>[{} component]</p>\n", component.component_type));
                    html.push_str("</div>\n\n");
                }
            }
        }

        Ok(ReadResourceResult {
            contents: vec![rmcp::model::ResourceContent {
                uri: uri.to_string(),
                mime_type: Some("text/html".to_string()),
                text: Some(html),
                blob: None,
            }],
        })
    }
}
```

### 2. Add Helper Method for Building Page Paths

Add this helper method to the impl block:

```rust
impl DoxydeRmcpService {
    // ... existing new() method ...

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
}
```

### 3. Update Dependencies

Add to `doxyde-shared/Cargo.toml`:

```toml
async-trait = "0.1"
```

## Notes

- Resources are limited to 100 pages in breadth-first order (root pages first, then their children)
- The URI format is `page://[id]` for consistency with the original implementation
- HTML content is generated from published versions only (drafts are not exposed)
- The resources feature is automatically enabled by rmcp when ResourcesProvider is implemented

## Testing

```rust
#[cfg(test)]
mod resources_tests {
    use super::*;
    use sqlx::SqlitePool;

    #[sqlx::test]
    async fn test_list_resources(pool: SqlitePool) -> Result<()> {
        // Create test site and pages
        let site_id = 1; // ... create test site
        let service = DoxydeRmcpService::new(pool, site_id);

        let resources = service.list_resources().await?;
        assert!(!resources.is_empty());

        // Verify first resource is root page
        assert!(resources[0].description.as_ref().unwrap().contains("Homepage"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_read_resource(pool: SqlitePool) -> Result<()> {
        // Create test page with content
        let page_id = 1; // ... create test page
        let service = DoxydeRmcpService::new(pool, 1);

        let result = service.read_resource(&format!("page://{}", page_id)).await?;
        assert_eq!(result.contents.len(), 1);
        assert_eq!(result.contents[0].mime_type.as_ref().unwrap(), "text/html");
        Ok(())
    }
}
```

## Next Steps

WRITE UNIT TESTS - MAKE SURE EVERYTHING IS STILL PASSING.

With resources support implemented, MCP clients can now browse and read page content. Next, we'll add prompts support.