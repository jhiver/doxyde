# Task 20: Update Component Markdown Tool

## Overview
Implement the `update_component_markdown` tool to update the content, title, or template of a markdown component in a draft version.

## Implementation

Add this tool to the `#[tool_router]` impl block:

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateComponentMarkdownRequest {
    #[schemars(description = "ID of the component to update")]
    pub component_id: i64,
    
    #[schemars(description = "New markdown content (optional)")]
    pub content: Option<String>,
    
    #[schemars(description = "New component title (optional)")]
    pub title: Option<String>,
    
    #[schemars(description = "New template (optional)")]
    pub template: Option<String>,
}

#[tool_router]
impl DoxydeRmcpService {
    #[tool(description = "Update the content, title, or template of a markdown component. Component must be in a draft version.")]
    pub async fn update_component_markdown(&self, Parameters(req): Parameters<UpdateComponentMarkdownRequest>) -> String {
        match self.internal_update_component_markdown(req).await {
            Ok(component_info) => serde_json::to_string_pretty(&component_info).unwrap_or_else(|e| {
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
    async fn internal_update_component_markdown(&self, req: UpdateComponentMarkdownRequest) -> Result<ComponentInfo> {
        use doxyde_db::repositories::{ComponentRepository, PageRepository, PageVersionRepository};
        
        let component_repo = ComponentRepository::new(self.pool.clone());
        
        // Get the component
        let mut component = component_repo
            .find_by_id(req.component_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Component not found"))?;
        
        // Verify it's a markdown component
        if component.component_type != "markdown" {
            return Err(anyhow::anyhow!(
                "Component is not a markdown component (type: {})",
                component.component_type
            ));
        }
        
        // Get the page version to verify it's a draft
        let version_repo = PageVersionRepository::new(self.pool.clone());
        let version = version_repo
            .find_by_id(component.page_version_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page version not found"))?;
        
        if version.is_published {
            return Err(anyhow::anyhow!(
                "Cannot update component in published version. Create or edit a draft first."
            ));
        }
        
        // Verify the page belongs to this site
        let page_repo = PageRepository::new(self.pool.clone());
        let page = page_repo
            .find_by_id(version.page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page not found"))?;
        
        if page.site_id != self.site_id {
            return Err(anyhow::anyhow!("Component belongs to a page in a different site"));
        }
        
        // Track if anything changed
        let mut changed = false;
        
        // Update content if provided
        if let Some(new_content) = req.content {
            if let Some(text) = component.data.get("text").and_then(|v| v.as_str()) {
                if text != new_content {
                    component.data = serde_json::json!({
                        "text": new_content
                    });
                    changed = true;
                }
            }
        }
        
        // Update title if provided
        if let Some(new_title) = req.title {
            if component.title.as_ref() != Some(&new_title) {
                component.title = Some(new_title);
                changed = true;
            }
        }
        
        // Update template if provided
        if let Some(new_template) = req.template {
            if component.template != new_template {
                component.template = new_template;
                changed = true;
            }
        }
        
        // Only update if something changed
        if changed {
            component.updated_at = chrono::Utc::now();
            component_repo.update(&component).await?;
        }
        
        // Get updated component
        let updated_component = component_repo
            .find_by_id(req.component_id)
            .await?
            .unwrap();
        
        Ok(self.component_to_info(updated_component))
    }
}
```

## Example Request

```json
{
    "component_id": 100,
    "content": "# Updated Welcome\n\nThis content has been updated!\n\n## New Features\n\n- Better performance\n- Enhanced security\n- Modern design",
    "title": "Updated Welcome Message",
    "template": "highlight"
}
```

## Example Response

```json
{
    "id": 100,
    "component_type": "markdown",
    "position": 0,
    "template": "highlight",
    "title": "Updated Welcome Message",
    "content": {
        "text": "# Updated Welcome\n\nThis content has been updated!\n\n## New Features\n\n- Better performance\n- Enhanced security\n- Modern design"
    },
    "created_at": "2024-01-20T10:30:00Z",
    "updated_at": "2024-01-20T11:45:00Z"
}
```

## Validation Rules

1. **Component Type**: Must be a markdown component
2. **Draft Only**: Component must be in a draft version
3. **Site Ownership**: Component's page must belong to current site
4. **Partial Updates**: Only provided fields are updated

## Error Cases

- Component not found
- Component is not markdown type
- Component is in published version
- Page belongs to different site

## Behavior

1. Verifies component is markdown type
2. Ensures component is in a draft version
3. Updates only the provided fields
4. Maintains other fields unchanged
5. Updates the `updated_at` timestamp

## Notes

- Cannot update components in published versions
- All fields are optional - provide only what needs updating
- No-op if no changes are made
- Content is stored in data.text field

## Testing

```rust
#[cfg(test)]
mod update_component_markdown_tests {
    use super::*;
    
    #[sqlx::test]
    async fn test_update_component_markdown_all_fields(pool: SqlitePool) -> Result<()> {
        // Create component in draft
        let component_id = 1; // ... create markdown component
        let service = DoxydeRmcpService::new(pool, 1);
        
        let req = UpdateComponentMarkdownRequest {
            component_id,
            content: Some("Updated content".to_string()),
            title: Some("Updated title".to_string()),
            template: Some("highlight".to_string()),
        };
        
        let result = service.update_component_markdown(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        let component: ComponentInfo = serde_json::from_str(&result)?;
        assert_eq!(component.template, "highlight");
        assert_eq!(component.title, Some("Updated title".to_string()));
        
        let content = component.content.get("text").unwrap().as_str().unwrap();
        assert_eq!(content, "Updated content");
        Ok(())
    }
    
    #[sqlx::test]
    async fn test_update_component_markdown_partial(pool: SqlitePool) -> Result<()> {
        // Create component with initial values
        let component_id = 1; // ... create with title "Original", template "default"
        let service = DoxydeRmcpService::new(pool, 1);
        
        // Only update content
        let req = UpdateComponentMarkdownRequest {
            component_id,
            content: Some("New content only".to_string()),
            title: None,
            template: None,
        };
        
        let result = service.update_component_markdown(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        let component: ComponentInfo = serde_json::from_str(&result)?;
        
        // Title and template should remain unchanged
        assert_eq!(component.title, Some("Original".to_string()));
        assert_eq!(component.template, "default");
        
        // Content should be updated
        let content = component.content.get("text").unwrap().as_str().unwrap();
        assert_eq!(content, "New content only");
        Ok(())
    }
    
    #[sqlx::test]
    async fn test_update_component_published_fails(pool: SqlitePool) -> Result<()> {
        // Create component in published version
        let component_id = 1; // ... create in published version
        let service = DoxydeRmcpService::new(pool, 1);
        
        let req = UpdateComponentMarkdownRequest {
            component_id,
            content: Some("Should fail".to_string()),
            title: None,
            template: None,
        };
        
        let result = service.update_component_markdown(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("Cannot update component in published version"));
        Ok(())
    }
    
    #[sqlx::test]
    async fn test_update_non_markdown_component_fails(pool: SqlitePool) -> Result<()> {
        // Create non-markdown component (e.g., image)
        let component_id = 1; // ... create image component
        let service = DoxydeRmcpService::new(pool, 1);
        
        let req = UpdateComponentMarkdownRequest {
            component_id,
            content: Some("Should fail".to_string()),
            title: None,
            template: None,
        };
        
        let result = service.update_component_markdown(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("not a markdown component"));
        Ok(())
    }
}
```

WRITE UNIT TESTS - MAKE SURE EVERYTHING IS STILL PASSING.