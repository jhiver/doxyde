# Task 07: Search Pages Tool

## Overview
Implement the `search_pages` tool to search pages by title or content.

## Implementation

Add this tool to the `#[tool_router]` impl block:

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchPagesRequest {
    #[schemars(description = "Search query")]
    pub query: String,
}

#[tool_router]
impl DoxydeRmcpService {
    #[tool(description = "Search pages by title or content")]
    pub async fn search_pages(&self, Parameters(req): Parameters<SearchPagesRequest>) -> String {
        match self.internal_search_pages(&req.query).await {
            Ok(pages) => serde_json::to_string_pretty(&pages).unwrap_or_else(|e| {
                format!("{{\"error\": \"Failed to serialize search results: {}\"}}", e)
            }),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }
}
```

Add the internal implementation:

```rust
impl DoxydeRmcpService {
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
}
```

## Example Request

```json
{
    "query": "team"
}
```

## Example Response

```json
[
    {
        "id": 5,
        "slug": "team",
        "title": "Our Team",
        "path": "/about/team",
        "parent_id": 2,
        "position": 1,
        "has_children": false,
        "template": "default"
    },
    {
        "id": 8,
        "slug": "team-values",
        "title": "Team Values",
        "path": "/about/team-values",
        "parent_id": 2,
        "position": 2,
        "has_children": false,
        "template": "default"
    }
]
```

## Search Scope

The tool searches in:
1. Page title
2. Page slug
3. Page description (metadata)
4. Page keywords (metadata)
5. Component titles (in published content)
6. Component content (in published content)

## Notes

- Search is case-insensitive
- Returns results sorted alphabetically by title
- Only searches published content (not drafts)
- Returns PageInfo objects (not full Page models)
- Empty query returns empty results

## Testing

```rust
#[cfg(test)]
mod search_pages_tests {
    use super::*;

    #[sqlx::test]
    async fn test_search_pages_by_title(pool: SqlitePool) -> Result<()> {
        // Create pages with "team" in title
        let service = DoxydeRmcpService::new(pool, 1);

        let req = SearchPagesRequest { query: "team".to_string() };
        let result = service.search_pages(Parameters(req)).await;

        assert!(!result.contains("error"));
        let pages: Vec<PageInfo> = serde_json::from_str(&result)?;
        assert!(!pages.is_empty());
        assert!(pages.iter().all(|p| p.title.to_lowercase().contains("team")));
        Ok(())
    }

    #[sqlx::test]
    async fn test_search_pages_in_content(pool: SqlitePool) -> Result<()> {
        // Create page with "team" in component content
        let service = DoxydeRmcpService::new(pool, 1);

        let req = SearchPagesRequest { query: "collaboration".to_string() };
        let result = service.search_pages(Parameters(req)).await;

        assert!(!result.contains("error"));
        let pages: Vec<PageInfo> = serde_json::from_str(&result)?;
        // Should find pages with "collaboration" in their content
        Ok(())
    }

    #[sqlx::test]
    async fn test_search_pages_no_results(pool: SqlitePool) -> Result<()> {
        let service = DoxydeRmcpService::new(pool, 1);

        let req = SearchPagesRequest { query: "xyznonexistent".to_string() };
        let result = service.search_pages(Parameters(req)).await;

        assert!(!result.contains("error"));
        let pages: Vec<PageInfo> = serde_json::from_str(&result)?;
        assert!(pages.is_empty());
        Ok(())
    }
}
```

WRITE UNIT TESTS - MAKE SURE EVERYTHING IS STILL PASSING.
