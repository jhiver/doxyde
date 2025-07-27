# Task 08: Get Published Content Tool

## Overview
Implement the `get_published_content` tool to get the published content (components) of a page.

## Implementation

Add this tool to the `#[tool_router]` impl block:

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetPublishedContentRequest {
    #[schemars(description = "The page ID")]
    pub page_id: i64,
}

#[tool_router]
impl DoxydeRmcpService {
    #[tool(description = "Get published content of a page")]
    pub async fn get_published_content(&self, Parameters(req): Parameters<GetPublishedContentRequest>) -> String {
        match self.internal_get_published_content(req.page_id).await {
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
    async fn internal_get_published_content(&self, page_id: i64) -> Result<Vec<ComponentInfo>> {
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

        // Get published version
        let version_repo = PageVersionRepository::new(self.pool.clone());
        let version = version_repo
            .get_published(page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("No published version exists for this page"))?;

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

        Ok(component_infos)
    }

    fn component_to_info(&self, component: doxyde_core::models::Component) -> ComponentInfo {
        ComponentInfo {
            id: component.id.unwrap(),
            component_type: component.component_type,
            position: component.position,
            template: component.template,
            title: component.title,
            content: component.content,
            created_at: component.created_at.to_rfc3339(),
            updated_at: component.updated_at.to_rfc3339(),
        }
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
        "template": "default",
        "title": "Welcome",
        "content": {
            "text": "# Welcome to our website\n\nThis is the home page content."
        },
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T00:00:00Z"
    },
    {
        "id": 2,
        "component_type": "markdown",
        "position": 1,
        "template": "card",
        "title": "Features",
        "content": {
            "text": "## Our Features\n\n- Feature 1\n- Feature 2\n- Feature 3"
        },
        "created_at": "2024-01-01T00:00:00Z",
        "updated_at": "2024-01-01T00:00:00Z"
    }
]
```

## Error Responses

- Page not found: `{"error": "Page not found"}`
- No published version: `{"error": "No published version exists for this page"}`
- Wrong site: `{"error": "Page does not belong to this site"}`

## Notes

- Only returns components from the published version
- Components are sorted by position
- Returns empty array if published version has no components
- Includes component metadata (type, template, timestamps)

## Testing

```rust
#[cfg(test)]
mod get_published_content_tests {
    use super::*;

    #[sqlx::test]
    async fn test_get_published_content(pool: SqlitePool) -> Result<()> {
        // Create page with published content
        let page_id = 1; // ... create and publish
        let service = DoxydeRmcpService::new(pool, 1);

        let req = GetPublishedContentRequest { page_id };
        let result = service.get_published_content(Parameters(req)).await;

        assert!(!result.contains("error"));
        let components: Vec<ComponentInfo> = serde_json::from_str(&result)?;
        assert!(!components.is_empty());
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_published_content_no_version(pool: SqlitePool) -> Result<()> {
        // Create page without published version
        let page_id = 1; // ... create but don't publish
        let service = DoxydeRmcpService::new(pool, 1);

        let req = GetPublishedContentRequest { page_id };
        let result = service.get_published_content(Parameters(req)).await;

        assert!(result.contains("error"));
        assert!(result.contains("No published version exists"));
        Ok(())
    }
}
```

WRITE UNIT TESTS - MAKE SURE EVERYTHING IS STILL PASSING.
