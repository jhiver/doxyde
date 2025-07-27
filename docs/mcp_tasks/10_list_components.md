# Task 10: List Components Tool

## Overview
Implement the `list_components` tool to list all components for a page (from the current draft if exists, otherwise from published version).

## Implementation

Add this tool to the `#[tool_router]` impl block:

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListComponentsRequest {
    #[schemars(description = "ID of the page to list components for")]
    pub page_id: i64,
}

#[tool_router]
impl DoxydeRmcpService {
    #[tool(description = "List all components for a page")]
    pub async fn list_components(&self, Parameters(req): Parameters<ListComponentsRequest>) -> String {
        match self.internal_list_components(req.page_id).await {
            Ok(components) => serde_json::to_string_pretty(&components).unwrap_or_else(|e| {
                format!("{{\"error\": \"Failed to serialize components: {}\"}}", e)
            }),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }
}
```

Add the internal implementation:

```rust
impl DoxydeRmcpService {
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
[
    {
        "id": 1,
        "component_type": "markdown",
        "position": 0,
        "template": "hero",
        "title": "Welcome Hero",
        "content": {
            "text": "# Welcome to Doxyde\n\nYour AI-native CMS"
        },
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T00:00:00Z"
    },
    {
        "id": 2,
        "component_type": "markdown",
        "position": 1,
        "template": "default",
        "title": "Introduction",
        "content": {
            "text": "Doxyde is a modern content management system..."
        },
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T00:00:00Z"
    }
]
```

## Priority Logic

1. If a draft version exists, returns components from the draft
2. If no draft but published version exists, returns components from published
3. If no versions exist, returns empty array

## Error Responses

- Page not found: `{"error": "Page not found"}`
- Wrong site: `{"error": "Page does not belong to this site"}`

## Notes

- This tool is useful for seeing the current state of a page
- Always prefers draft over published when both exist
- Components are sorted by position
- Returns empty array (not an error) when page has no versions

## Testing

```rust
#[cfg(test)]
mod list_components_tests {
    use super::*;

    #[sqlx::test]
    async fn test_list_components_draft(pool: SqlitePool) -> Result<()> {
        // Create page with both draft and published versions
        let page_id = 1; // ... create with draft
        let service = DoxydeRmcpService::new(pool, 1);

        let req = ListComponentsRequest { page_id };
        let result = service.list_components(Parameters(req)).await;

        assert!(!result.contains("error"));
        let components: Vec<ComponentInfo> = serde_json::from_str(&result)?;
        // Should return draft components
        Ok(())
    }

    #[sqlx::test]
    async fn test_list_components_published_only(pool: SqlitePool) -> Result<()> {
        // Create page with only published version
        let page_id = 1; // ... create and publish
        let service = DoxydeRmcpService::new(pool, 1);

        let req = ListComponentsRequest { page_id };
        let result = service.list_components(Parameters(req)).await;

        assert!(!result.contains("error"));
        let components: Vec<ComponentInfo> = serde_json::from_str(&result)?;
        // Should return published components
        Ok(())
    }

    #[sqlx::test]
    async fn test_list_components_no_versions(pool: SqlitePool) -> Result<()> {
        // Create page without any versions
        let page_id = 1; // ... create page only
        let service = DoxydeRmcpService::new(pool, 1);

        let req = ListComponentsRequest { page_id };
        let result = service.list_components(Parameters(req)).await;

        assert!(!result.contains("error"));
        let components: Vec<ComponentInfo> = serde_json::from_str(&result)?;
        assert!(components.is_empty());
        Ok(())
    }
}
```

WRITE UNIT TESTS - MAKE SURE EVERYTHING IS STILL PASSING.
