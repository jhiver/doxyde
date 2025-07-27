# Task 06: Get Page by Path Tool

## Overview
Implement the `get_page_by_path` tool to find a page by its URL path (e.g., '/about/team').

## Implementation

Add this tool to the `#[tool_router]` impl block:

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetPageByPathRequest {
    #[schemars(description = "The URL path to search for")]
    pub path: String,
}

#[tool_router]
impl DoxydeRmcpService {
    #[tool(description = "Find page by URL path (e.g., '/about/team')")]
    pub async fn get_page_by_path(&self, Parameters(req): Parameters<GetPageByPathRequest>) -> String {
        match self.internal_get_page_by_path(&req.path).await {
            Ok(page) => serde_json::to_string_pretty(&page).unwrap_or_else(|e| {
                format!("{{\"error\": \"Failed to serialize page: {}\"}}", e)
            }),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }
}
```

Add the internal implementation:

```rust
impl DoxydeRmcpService {
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
        let path_parts: Vec<&str> = normalized_path.split('/').collect();

        // Find page by matching the constructed path
        for page in &all_pages {
            let page_path = self.build_page_path(&all_pages, page).await?;
            if page_path.trim_matches('/') == normalized_path {
                return Ok(page.clone());
            }
        }

        Err(anyhow::anyhow!("Page not found at path: {}", path))
    }
}
```

## Example Request

```json
{
    "path": "/about/team"
}
```

## Example Response

```json
{
    "id": 5,
    "site_id": 1,
    "parent_page_id": 2,
    "slug": "team",
    "title": "Our Team",
    "template": "default",
    "position": 1,
    "description": "Meet our amazing team",
    "keywords": "team, about, staff",
    "created_at": "2024-01-01T00:00:00Z",
    "updated_at": "2024-01-01T00:00:00Z"
}
```

## Special Cases

- Root path: `"/"` or `""` returns the root page
- Path normalization: `/about/team/`, `about/team`, and `/about/team` all work the same

## Error Responses

- Page not found: `{"error": "Page not found at path: /invalid/path"}`
- Root page not found: `{"error": "Root page not found"}`

## Notes

- The path matching is case-sensitive
- Paths are normalized to remove leading/trailing slashes
- The tool reconstructs the full path for each page to find matches
- Only returns pages belonging to the current site

## Testing

```rust
#[cfg(test)]
mod get_page_by_path_tests {
    use super::*;

    #[sqlx::test]
    async fn test_get_page_by_path_root(pool: SqlitePool) -> Result<()> {
        let service = DoxydeRmcpService::new(pool, 1);

        let req = GetPageByPathRequest { path: "/".to_string() };
        let result = service.get_page_by_path(Parameters(req)).await;

        assert!(!result.contains("error"));
        let page: doxyde_core::models::Page = serde_json::from_str(&result)?;
        assert!(page.parent_page_id.is_none());
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_page_by_path_nested(pool: SqlitePool) -> Result<()> {
        // Create nested page structure
        let service = DoxydeRmcpService::new(pool, 1);

        let req = GetPageByPathRequest { path: "/about/team".to_string() };
        let result = service.get_page_by_path(Parameters(req)).await;

        assert!(!result.contains("error"));
        let page: doxyde_core::models::Page = serde_json::from_str(&result)?;
        assert_eq!(page.slug, "team");
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_page_by_path_not_found(pool: SqlitePool) -> Result<()> {
        let service = DoxydeRmcpService::new(pool, 1);

        let req = GetPageByPathRequest { path: "/does/not/exist".to_string() };
        let result = service.get_page_by_path(Parameters(req)).await;

        assert!(result.contains("error"));
        assert!(result.contains("Page not found at path"));
        Ok(())
    }
}
```

WRITE UNIT TESTS - MAKE SURE EVERYTHING IS STILL PASSING.
