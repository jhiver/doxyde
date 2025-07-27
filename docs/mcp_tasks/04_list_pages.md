# Task 04: List Pages Tool

## Overview
Implement the `list_pages` tool to get all pages in the site with hierarchy.

## Implementation

Add this tool to the `#[tool_router]` impl block in `doxyde-shared/src/mcp/service.rs`:

```rust
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
```

Add the internal implementation method:

```rust
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
```

## Example Response

```json
[
  {
    "page": {
      "id": 1,
      "slug": "home",
      "title": "Home",
      "path": "/",
      "parent_id": null,
      "position": 0,
      "has_children": true,
      "template": "default"
    },
    "children": [
      {
        "page": {
          "id": 2,
          "slug": "about",
          "title": "About Us",
          "path": "/about",
          "parent_id": 1,
          "position": 0,
          "has_children": false,
          "template": "default"
        },
        "children": []
      }
    ]
  }
]
```

## Notes

- Returns a hierarchical structure with root pages at the top level
- Each page includes its children recursively
- The `path` field shows the full URL path to the page
- Pages are sorted by their position within each level

## Testing

```rust
#[cfg(test)]
mod list_pages_tests {
    use super::*;

    #[sqlx::test]
    async fn test_list_pages_tool(pool: SqlitePool) -> Result<()> {
        // Create test site and pages
        let service = DoxydeRmcpService::new(pool, 1);

        let result = service.list_pages().await;
        assert!(!result.contains("error"));

        // Parse and verify structure
        let pages: Vec<PageHierarchy> = serde_json::from_str(&result)?;
        assert!(!pages.is_empty());
        Ok(())
    }
}
```

WRITE UNIT TESTS - MAKE SURE EVERYTHING IS STILL PASSING.
