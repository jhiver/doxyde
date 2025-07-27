# Task 11: Get Component Tool

## Overview
Implement the `get_component` tool to get details of a specific component by ID.

## Implementation

Add this tool to the `#[tool_router]` impl block:

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetComponentRequest {
    #[schemars(description = "ID of the component to retrieve")]
    pub component_id: i64,
}

#[tool_router]
impl DoxydeRmcpService {
    #[tool(description = "Get details of a specific component")]
    pub async fn get_component(&self, Parameters(req): Parameters<GetComponentRequest>) -> String {
        match self.internal_get_component(req.component_id).await {
            Ok(component) => serde_json::to_string_pretty(&component).unwrap_or_else(|e| {
                format!("{{\"error\": \"Failed to serialize component: {}\"}}", e)
            }),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }
}
```

Add the internal implementation:

```rust
impl DoxydeRmcpService {
    async fn internal_get_component(&self, component_id: i64) -> Result<ComponentInfo> {
        use doxyde_db::repositories::{ComponentRepository, PageRepository, PageVersionRepository};

        let component_repo = ComponentRepository::new(self.pool.clone());

        // Get the component
        let component = component_repo
            .find_by_id(component_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Component not found"))?;

        // Get the page version to find the page
        let version_repo = PageVersionRepository::new(self.pool.clone());
        let version = version_repo
            .find_by_id(component.page_version_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page version not found"))?;

        // Verify the page belongs to this site
        let page_repo = PageRepository::new(self.pool.clone());
        let page = page_repo
            .find_by_id(version.page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page not found"))?;

        if page.site_id != self.site_id {
            return Err(anyhow::anyhow!("Component belongs to a page in a different site"));
        }

        Ok(self.component_to_info(component))
    }
}
```

## Example Request

```json
{
    "component_id": 42
}
```

## Example Response

```json
{
    "id": 42,
    "component_type": "markdown",
    "position": 2,
    "template": "card",
    "title": "Product Features",
    "content": {
        "text": "## Key Features\n\n- Feature 1: Lightning fast\n- Feature 2: Secure by default\n- Feature 3: AI-powered"
    },
    "created_at": "2024-01-15T10:30:00Z",
    "updated_at": "2024-01-16T14:45:00Z"
}
```

## Error Responses

- Component not found: `{"error": "Component not found"}`
- Page version not found: `{"error": "Page version not found"}`
- Page not found: `{"error": "Page not found"}`
- Wrong site: `{"error": "Component belongs to a page in a different site"}`

## Notes

- Verifies that the component belongs to a page in the current site
- Works for components in both draft and published versions
- Returns full component details including content
- Does not indicate whether the component is in a draft or published version

## Testing

```rust
#[cfg(test)]
mod get_component_tests {
    use super::*;

    #[sqlx::test]
    async fn test_get_component(pool: SqlitePool) -> Result<()> {
        // Create component
        let component_id = 1; // ... create component
        let service = DoxydeRmcpService::new(pool, 1);

        let req = GetComponentRequest { component_id };
        let result = service.get_component(Parameters(req)).await;

        assert!(!result.contains("error"));
        let component: ComponentInfo = serde_json::from_str(&result)?;
        assert_eq!(component.id, component_id);
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_component_not_found(pool: SqlitePool) -> Result<()> {
        let service = DoxydeRmcpService::new(pool, 1);

        let req = GetComponentRequest { component_id: 99999 };
        let result = service.get_component(Parameters(req)).await;

        assert!(result.contains("error"));
        assert!(result.contains("Component not found"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_component_wrong_site(pool: SqlitePool) -> Result<()> {
        // Create component in different site
        let component_id = 1; // ... create in site 2
        let service = DoxydeRmcpService::new(pool, 1); // Using site 1

        let req = GetComponentRequest { component_id };
        let result = service.get_component(Parameters(req)).await;

        assert!(result.contains("error"));
        assert!(result.contains("different site"));
        Ok(())
    }
}
```

WRITE UNIT TESTS - MAKE SURE EVERYTHING IS STILL PASSING.
