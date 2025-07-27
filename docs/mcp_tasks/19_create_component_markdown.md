# Task 19: Create Component Markdown Tool

## Overview
Implement the `create_component_markdown` tool to create a new markdown component in a page's draft version.

## Implementation

Add this tool to the `#[tool_router]` impl block:

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateComponentMarkdownRequest {
    #[schemars(description = "ID of the page to add the component to")]
    pub page_id: i64,
    
    #[schemars(description = "Position in the component list (0-based). If not provided, component is added at the end.")]
    pub position: Option<i32>,
    
    #[schemars(description = "Component template (default, card, highlight, quote)")]
    pub template: Option<String>,
    
    #[schemars(description = "Optional component title")]
    pub title: Option<String>,
    
    #[schemars(description = "Markdown content of the component")]
    pub content: String,
}

#[tool_router]
impl DoxydeRmcpService {
    #[tool(description = "Create a new markdown component in a page's draft version. Automatically creates a draft if none exists.")]
    pub async fn create_component_markdown(&self, Parameters(req): Parameters<CreateComponentMarkdownRequest>) -> String {
        match self.internal_create_component_markdown(req).await {
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
    async fn internal_create_component_markdown(&self, req: CreateComponentMarkdownRequest) -> Result<ComponentInfo> {
        use doxyde_db::repositories::{ComponentRepository, PageRepository, PageVersionRepository};
        
        // Verify page exists and belongs to this site
        let page_repo = PageRepository::new(self.pool.clone());
        let page = page_repo
            .find_by_id(req.page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page not found"))?;
        
        if page.site_id != self.site_id {
            return Err(anyhow::anyhow!("Page does not belong to this site"));
        }
        
        // Get or create draft version
        let draft_info = self.internal_get_or_create_draft(req.page_id).await?;
        
        // Determine position
        let component_repo = ComponentRepository::new(self.pool.clone());
        let existing_components = component_repo
            .list_by_page_version(draft_info.version_id)
            .await?;
        
        let target_position = req.position
            .unwrap_or(existing_components.len() as i32)
            .clamp(0, existing_components.len() as i32);
        
        // Shift existing components if needed
        if target_position < existing_components.len() as i32 {
            for mut comp in existing_components {
                if comp.position >= target_position {
                    comp.position += 1;
                    component_repo.update(&comp).await?;
                }
            }
        }
        
        // Create component data
        let component_data = serde_json::json!({
            "text": req.content
        });
        
        // Create the component
        let new_component = doxyde_core::models::Component {
            id: None,
            page_version_id: draft_info.version_id,
            component_type: "markdown".to_string(),
            position: target_position,
            template: req.template.unwrap_or_else(|| "default".to_string()),
            title: req.title,
            data: component_data,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        
        let component_id = component_repo.create(&new_component).await?;
        
        // Get the created component
        let created_component = component_repo
            .find_by_id(component_id)
            .await?
            .unwrap();
        
        Ok(self.component_to_info(created_component))
    }
}
```

## Example Request

```json
{
    "page_id": 1,
    "position": 0,
    "template": "card",
    "title": "Welcome Message",
    "content": "# Welcome to Our Website\n\nWe're glad you're here! This is our brand new homepage.\n\n## What We Offer\n\n- Professional services\n- Expert consultation\n- 24/7 support"
}
```

## Example Response

```json
{
    "id": 100,
    "component_type": "markdown",
    "position": 0,
    "template": "card",
    "title": "Welcome Message",
    "content": {
        "text": "# Welcome to Our Website\n\nWe're glad you're here! This is our brand new homepage.\n\n## What We Offer\n\n- Professional services\n- Expert consultation\n- 24/7 support"
    },
    "created_at": "2024-01-20T10:30:00Z",
    "updated_at": "2024-01-20T10:30:00Z"
}
```

## Validation Rules

1. **Page Existence**: Must exist and belong to current site
2. **Draft Creation**: Automatically creates draft if none exists
3. **Position**: Clamped to valid range (0 to component count)
4. **Template**: Defaults to "default" if not provided

## Error Cases

- Page not found
- Page belongs to different site

## Behavior

1. Creates a draft version if none exists
2. Inserts component at specified position
3. Shifts existing components down if inserting in middle
4. Returns the created component with generated ID

## Notes

- Always works on draft version (never published)
- Markdown content is stored in data.text field
- Position is 0-based
- Components maintain continuous position numbering

## Testing

```rust
#[cfg(test)]
mod create_component_markdown_tests {
    use super::*;
    
    #[sqlx::test]
    async fn test_create_component_markdown(pool: SqlitePool) -> Result<()> {
        let page_id = 1; // ... create page
        let service = DoxydeRmcpService::new(pool, 1);
        
        let req = CreateComponentMarkdownRequest {
            page_id,
            position: Some(0),
            template: Some("card".to_string()),
            title: Some("Test Component".to_string()),
            content: "# Test Content\n\nThis is a test.".to_string(),
        };
        
        let result = service.create_component_markdown(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        let component: ComponentInfo = serde_json::from_str(&result)?;
        assert_eq!(component.component_type, "markdown");
        assert_eq!(component.position, 0);
        assert_eq!(component.template, "card");
        assert_eq!(component.title, Some("Test Component".to_string()));
        
        let content = component.content.get("text").unwrap().as_str().unwrap();
        assert!(content.contains("# Test Content"));
        Ok(())
    }
    
    #[sqlx::test]
    async fn test_create_component_auto_draft(pool: SqlitePool) -> Result<()> {
        // Create page without draft
        let page_id = 1; // ... create page
        let service = DoxydeRmcpService::new(pool.clone(), 1);
        
        let req = CreateComponentMarkdownRequest {
            page_id,
            position: None,
            template: None,
            title: None,
            content: "Simple content".to_string(),
        };
        
        let result = service.create_component_markdown(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        
        // Verify draft was created
        use doxyde_db::repositories::PageVersionRepository;
        let version_repo = PageVersionRepository::new(pool);
        let draft = version_repo.get_draft(page_id).await?;
        assert!(draft.is_some());
        Ok(())
    }
    
    #[sqlx::test]
    async fn test_create_component_position_shift(pool: SqlitePool) -> Result<()> {
        let page_id = 1; // ... create page with 3 components at positions 0,1,2
        let service = DoxydeRmcpService::new(pool.clone(), 1);
        
        // Insert at position 1
        let req = CreateComponentMarkdownRequest {
            page_id,
            position: Some(1),
            template: None,
            title: None,
            content: "Inserted content".to_string(),
        };
        
        service.create_component_markdown(Parameters(req)).await;
        
        // Verify positions
        use doxyde_db::repositories::{ComponentRepository, PageVersionRepository};
        let version_repo = PageVersionRepository::new(pool.clone());
        let draft = version_repo.get_draft(page_id).await?.unwrap();
        let component_repo = ComponentRepository::new(pool);
        let components = component_repo.list_by_page_version(draft.id.unwrap()).await?;
        
        assert_eq!(components.len(), 4);
        assert_eq!(components[0].position, 0); // Original first
        assert_eq!(components[1].position, 1); // New component
        assert_eq!(components[2].position, 2); // Original second (shifted)
        assert_eq!(components[3].position, 3); // Original third (shifted)
        Ok(())
    }
}
```

WRITE UNIT TESTS - MAKE SURE EVERYTHING IS STILL PASSING.