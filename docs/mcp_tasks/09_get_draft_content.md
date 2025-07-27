# Task 09: Get Draft Content Tool

## Overview
Implement the `get_draft_content` tool to get the draft content of a page (if it exists).

## Implementation

Add this tool to the `#[tool_router]` impl block:

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetDraftContentRequest {
    #[schemars(description = "The page ID")]
    pub page_id: i64,
}

#[tool_router]
impl DoxydeRmcpService {
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
}
```

Add the internal implementation:

```rust
impl DoxydeRmcpService {
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
}
```

## Example Request

```json
{
    "page_id": 1
}
```

## Example Response (Draft Exists)

```json
[
    {
        "id": 10,
        "component_type": "markdown",
        "position": 0,
        "template": "default",
        "title": "Draft Welcome",
        "content": {
            "text": "# Welcome to our website (DRAFT)\n\nThis is the draft version of the home page."
        },
        "created_at": "2024-01-02T00:00:00Z",
        "updated_at": "2024-01-02T00:00:00Z"
    }
]
```

## Example Response (No Draft)

```json
null
```

## Error Responses

- Page not found: `{"error": "Page not found"}`
- Wrong site: `{"error": "Page does not belong to this site"}`

## Notes

- Returns `null` (not an error) when no draft exists
- Only returns unpublished draft versions
- Components are sorted by position
- Uses the same ComponentInfo format as published content

## Testing

```rust
#[cfg(test)]
mod get_draft_content_tests {
    use super::*;

    #[sqlx::test]
    async fn test_get_draft_content_exists(pool: SqlitePool) -> Result<()> {
        // Create page with draft
        let page_id = 1; // ... create page and draft
        let service = DoxydeRmcpService::new(pool, 1);

        let req = GetDraftContentRequest { page_id };
        let result = service.get_draft_content(Parameters(req)).await;

        assert!(!result.contains("error"));
        assert!(result != "null");
        let components: Vec<ComponentInfo> = serde_json::from_str(&result)?;
        assert!(!components.is_empty());
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_draft_content_no_draft(pool: SqlitePool) -> Result<()> {
        // Create page without draft
        let page_id = 1; // ... create page only
        let service = DoxydeRmcpService::new(pool, 1);

        let req = GetDraftContentRequest { page_id };
        let result = service.get_draft_content(Parameters(req)).await;

        assert!(!result.contains("error"));
        assert_eq!(result, "null");
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_draft_content_page_not_found(pool: SqlitePool) -> Result<()> {
        let service = DoxydeRmcpService::new(pool, 1);

        let req = GetDraftContentRequest { page_id: 99999 };
        let result = service.get_draft_content(Parameters(req)).await;

        assert!(result.contains("error"));
        assert!(result.contains("Page not found"));
        Ok(())
    }
}
```

WRITE UNIT TESTS - MAKE SURE EVERYTHING IS STILL PASSING.
