# Task 05: Get Page Tool

## Overview
Implement the `get_page` tool to get full page details by ID.

## Implementation

Add this tool to the `#[tool_router]` impl block:

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetPageRequest {
    #[schemars(description = "The page ID")]
    pub page_id: i64,
}

#[tool_router]
impl DoxydeRmcpService {
    #[tool(description = "Get full page details by ID")]
    pub async fn get_page(&self, Parameters(req): Parameters<GetPageRequest>) -> String {
        match self.internal_get_page(req.page_id).await {
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
}
```

## Example Request

```json
{
    "page_id": 1
}
```

## Example Response

```json
{
    "id": 1,
    "site_id": 1,
    "parent_page_id": null,
    "slug": "home",
    "title": "Home",
    "template": "default",
    "position": 0,
    "description": "Welcome to our website",
    "keywords": "home, welcome",
    "created_at": "2024-01-01T00:00:00Z",
    "updated_at": "2024-01-01T00:00:00Z"
}
```

## Error Responses

- Page not found: `{"error": "Page not found"}`
- Page belongs to different site: `{"error": "Page does not belong to this site"}`

## Notes

- The tool verifies that the requested page belongs to the current site
- Returns the full page model including metadata fields
- Does not include page content (components) - use `get_published_content` or `get_draft_content` for that

## Testing

```rust
#[cfg(test)]
mod get_page_tests {
    use super::*;

    #[sqlx::test]
    async fn test_get_page_tool(pool: SqlitePool) -> Result<()> {
        // Create test page
        let page_id = 1; // ... create page
        let service = DoxydeRmcpService::new(pool, 1);

        let req = GetPageRequest { page_id };
        let result = service.get_page(Parameters(req)).await;

        assert!(!result.contains("error"));
        let page: doxyde_core::models::Page = serde_json::from_str(&result)?;
        assert_eq!(page.id.unwrap(), page_id);
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_page_not_found(pool: SqlitePool) -> Result<()> {
        let service = DoxydeRmcpService::new(pool, 1);

        let req = GetPageRequest { page_id: 99999 };
        let result = service.get_page(Parameters(req)).await;

        assert!(result.contains("error"));
        assert!(result.contains("Page not found"));
        Ok(())
    }
}
```

WRITE UNIT TESTS - MAKE SURE EVERYTHING IS STILL PASSING.
