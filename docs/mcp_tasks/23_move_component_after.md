# Task 23: Move Component After Tool

## Overview
Implement the `move_component_after` tool to move a component after another component in the same draft version.

## Implementation

Add this tool to the `#[tool_router]` impl block:

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct MoveComponentAfterRequest {
    #[schemars(description = "ID of the component to move")]
    pub component_id: i64,
    
    #[schemars(description = "ID of the component to move after")]
    pub after_component_id: i64,
}

#[tool_router]
impl DoxydeRmcpService {
    #[tool(description = "Move a component after another component in the same draft version")]
    pub async fn move_component_after(&self, Parameters(req): Parameters<MoveComponentAfterRequest>) -> String {
        match self.internal_move_component_after(req).await {
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
    async fn internal_move_component_after(&self, req: MoveComponentAfterRequest) -> Result<ComponentInfo> {
        use doxyde_db::repositories::{ComponentRepository, PageRepository, PageVersionRepository};
        
        if req.component_id == req.after_component_id {
            return Err(anyhow::anyhow!("Cannot move component after itself"));
        }
        
        let component_repo = ComponentRepository::new(self.pool.clone());
        
        // Get both components
        let component = component_repo
            .find_by_id(req.component_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Component to move not found"))?;
        
        let after_component = component_repo
            .find_by_id(req.after_component_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Target component not found"))?;
        
        // Verify they're in the same page version
        if component.page_version_id != after_component.page_version_id {
            return Err(anyhow::anyhow!("Components must be in the same page version"));
        }
        
        // Verify it's a draft version
        let version_repo = PageVersionRepository::new(self.pool.clone());
        let version = version_repo
            .find_by_id(component.page_version_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page version not found"))?;
        
        if version.is_published {
            return Err(anyhow::anyhow!(
                "Cannot move components in published version. Create or edit a draft first."
            ));
        }
        
        // Verify the page belongs to this site
        let page_repo = PageRepository::new(self.pool.clone());
        let page = page_repo
            .find_by_id(version.page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page not found"))?;
        
        if page.site_id != self.site_id {
            return Err(anyhow::anyhow!("Components belong to a page in a different site"));
        }
        
        // Get all components in the version
        let mut all_components = component_repo
            .list_by_page_version(component.page_version_id)
            .await?;
        
        all_components.sort_by_key(|c| c.position);
        
        let old_position = component.position;
        let new_position = after_component.position;
        
        // If already in correct position, no-op
        if old_position == new_position + 1 {
            return Ok(self.component_to_info(component));
        }
        
        // Reorder components
        if old_position < new_position {
            // Moving down: shift components between old and new position up
            for mut comp in all_components {
                if comp.position > old_position && comp.position <= new_position {
                    comp.position -= 1;
                    component_repo.update(&comp).await?;
                } else if comp.id == Some(req.component_id) {
                    comp.position = new_position;
                    component_repo.update(&comp).await?;
                }
            }
        } else {
            // Moving up: shift components between new and old position down
            for mut comp in all_components {
                if comp.position > new_position && comp.position < old_position {
                    comp.position += 1;
                    component_repo.update(&comp).await?;
                } else if comp.id == Some(req.component_id) {
                    comp.position = new_position + 1;
                    component_repo.update(&comp).await?;
                }
            }
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
    "after_component_id": 102
}
```

## Example Response

```json
{
    "id": 100,
    "component_type": "markdown",
    "position": 2,
    "template": "default",
    "title": "Moved Component",
    "content": {
        "text": "This component has been moved"
    },
    "created_at": "2024-01-20T10:30:00Z",
    "updated_at": "2024-01-20T12:00:00Z"
}
```

## Validation Rules

1. **Component Existence**: Both components must exist
2. **Same Version**: Components must be in the same page version
3. **Draft Only**: Version must be a draft (not published)
4. **Site Ownership**: Page must belong to current site
5. **Not Self**: Cannot move component after itself

## Error Cases

- Component to move not found
- Target component not found
- Components in different page versions
- Components in published version
- Page belongs to different site
- Trying to move component after itself

## Behavior

1. Moves component to position immediately after target
2. Adjusts positions of affected components
3. Maintains continuous position numbering
4. No-op if already in correct position

## Notes

- Only works on draft versions
- Positions are automatically adjusted to remain continuous
- Moving "after" means the component will have a higher position number
- This is the complement to `move_component_before`

## Testing

```rust
#[cfg(test)]
mod move_component_after_tests {
    use super::*;
    
    #[sqlx::test]
    async fn test_move_component_after_down(pool: SqlitePool) -> Result<()> {
        // Create components: 0, 1, 2, 3
        let comp0_id = 100; // position 0
        let comp1_id = 101; // position 1
        let comp2_id = 102; // position 2
        let comp3_id = 103; // position 3
        let service = DoxydeRmcpService::new(pool.clone(), 1);
        
        // Move component 0 after component 2 (moving down)
        let req = MoveComponentAfterRequest {
            component_id: comp0_id,
            after_component_id: comp2_id,
        };
        
        let result = service.move_component_after(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        let moved: ComponentInfo = serde_json::from_str(&result)?;
        assert_eq!(moved.position, 2); // Should be at position 2 (after old 2)
        
        // Verify new order: 1, 2, 0, 3
        use doxyde_db::repositories::ComponentRepository;
        let component_repo = ComponentRepository::new(pool);
        assert_eq!(component_repo.find_by_id(comp1_id).await?.unwrap().position, 0);
        assert_eq!(component_repo.find_by_id(comp2_id).await?.unwrap().position, 1);
        assert_eq!(component_repo.find_by_id(comp0_id).await?.unwrap().position, 2);
        assert_eq!(component_repo.find_by_id(comp3_id).await?.unwrap().position, 3);
        Ok(())
    }
    
    #[sqlx::test]
    async fn test_move_component_after_up(pool: SqlitePool) -> Result<()> {
        // Create components: 0, 1, 2, 3
        let comp0_id = 100; // position 0
        let comp1_id = 101; // position 1
        let comp2_id = 102; // position 2
        let comp3_id = 103; // position 3
        let service = DoxydeRmcpService::new(pool.clone(), 1);
        
        // Move component 3 after component 0 (moving up)
        let req = MoveComponentAfterRequest {
            component_id: comp3_id,
            after_component_id: comp0_id,
        };
        
        let result = service.move_component_after(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        let moved: ComponentInfo = serde_json::from_str(&result)?;
        assert_eq!(moved.position, 1); // Should be at position 1 (after 0)
        
        // Verify new order: 0, 3, 1, 2
        use doxyde_db::repositories::ComponentRepository;
        let component_repo = ComponentRepository::new(pool);
        assert_eq!(component_repo.find_by_id(comp0_id).await?.unwrap().position, 0);
        assert_eq!(component_repo.find_by_id(comp3_id).await?.unwrap().position, 1);
        assert_eq!(component_repo.find_by_id(comp1_id).await?.unwrap().position, 2);
        assert_eq!(component_repo.find_by_id(comp2_id).await?.unwrap().position, 3);
        Ok(())
    }
    
    #[sqlx::test]
    async fn test_move_component_after_self_fails(pool: SqlitePool) -> Result<()> {
        let component_id = 100;
        let service = DoxydeRmcpService::new(pool, 1);
        
        let req = MoveComponentAfterRequest {
            component_id,
            after_component_id: component_id,
        };
        
        let result = service.move_component_after(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("Cannot move component after itself"));
        Ok(())
    }
    
    #[sqlx::test]
    async fn test_move_component_after_last(pool: SqlitePool) -> Result<()> {
        // Create components: 0, 1, 2
        let comp0_id = 100; // position 0
        let comp1_id = 101; // position 1
        let comp2_id = 102; // position 2
        let service = DoxydeRmcpService::new(pool.clone(), 1);
        
        // Move component 0 after component 2 (to end)
        let req = MoveComponentAfterRequest {
            component_id: comp0_id,
            after_component_id: comp2_id,
        };
        
        let result = service.move_component_after(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        let moved: ComponentInfo = serde_json::from_str(&result)?;
        assert_eq!(moved.position, 2); // Should be at last position
        
        // Verify new order: 1, 2, 0
        use doxyde_db::repositories::ComponentRepository;
        let component_repo = ComponentRepository::new(pool);
        assert_eq!(component_repo.find_by_id(comp1_id).await?.unwrap().position, 0);
        assert_eq!(component_repo.find_by_id(comp2_id).await?.unwrap().position, 1);
        assert_eq!(component_repo.find_by_id(comp0_id).await?.unwrap().position, 2);
        Ok(())
    }
}
```

WRITE UNIT TESTS - MAKE SURE EVERYTHING IS STILL PASSING.