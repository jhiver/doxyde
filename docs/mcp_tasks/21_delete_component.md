# Task 21: Delete Component Tool

## Overview
Implement the `delete_component` tool to delete a component from a draft version.

## Implementation

Add this tool to the `#[tool_router]` impl block:

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeleteComponentRequest {
    #[schemars(description = "ID of the component to delete")]
    pub component_id: i64,
}

#[tool_router]
impl DoxydeRmcpService {
    #[tool(description = "Delete a component from a draft version. This operation cannot be undone.")]
    pub async fn delete_component(&self, Parameters(req): Parameters<DeleteComponentRequest>) -> String {
        match self.internal_delete_component(req.component_id).await {
            Ok(_) => format!("Successfully deleted component {}", req.component_id),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }
}
```

Add the internal implementation:

```rust
impl DoxydeRmcpService {
    async fn internal_delete_component(&self, component_id: i64) -> Result<()> {
        use doxyde_db::repositories::{ComponentRepository, PageRepository, PageVersionRepository};
        
        let component_repo = ComponentRepository::new(self.pool.clone());
        
        // Get the component
        let component = component_repo
            .find_by_id(component_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Component not found"))?;
        
        // Get the page version to verify it's a draft
        let version_repo = PageVersionRepository::new(self.pool.clone());
        let version = version_repo
            .find_by_id(component.page_version_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page version not found"))?;
        
        if version.is_published {
            return Err(anyhow::anyhow!(
                "Cannot delete component from published version. Create or edit a draft first."
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
        
        let deleted_position = component.position;
        
        // Delete the component
        component_repo.delete(component_id).await?;
        
        // Update positions of remaining components
        let mut remaining_components = component_repo
            .list_by_page_version(component.page_version_id)
            .await?;
        
        remaining_components.sort_by_key(|c| c.position);
        
        // Shift positions down for components after the deleted one
        for mut comp in remaining_components {
            if comp.position > deleted_position {
                comp.position -= 1;
                component_repo.update(&comp).await?;
            }
        }
        
        Ok(())
    }
}
```

## Example Request

```json
{
    "component_id": 100
}
```

## Example Response (Success)

```
Successfully deleted component 100
```

## Example Response (Error)

```json
{
    "error": "Cannot delete component from published version. Create or edit a draft first."
}
```

## Behavior

1. Verifies component exists
2. Ensures component is in a draft version (not published)
3. Verifies page belongs to current site
4. Deletes the component
5. Updates positions of remaining components to maintain continuity

## Error Cases

- Component not found
- Component is in published version
- Page belongs to different site

## Side Effects

- Permanently deletes the component
- Shifts positions of subsequent components down by 1
- Cannot be undone

## Notes

- Only works on draft versions
- Maintains continuous position numbering
- This is a destructive operation
- Consider implementing soft delete in the future

## Testing

```rust
#[cfg(test)]
mod delete_component_tests {
    use super::*;
    
    #[sqlx::test]
    async fn test_delete_component(pool: SqlitePool) -> Result<()> {
        // Create component in draft
        let component_id = 1; // ... create component
        let service = DoxydeRmcpService::new(pool.clone(), 1);
        
        let req = DeleteComponentRequest { component_id };
        let result = service.delete_component(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        assert!(result.contains("Successfully deleted"));
        
        // Verify component is gone
        use doxyde_db::repositories::ComponentRepository;
        let component_repo = ComponentRepository::new(pool);
        let component = component_repo.find_by_id(component_id).await?;
        assert!(component.is_none());
        Ok(())
    }
    
    #[sqlx::test]
    async fn test_delete_component_position_update(pool: SqlitePool) -> Result<()> {
        // Create 3 components at positions 0, 1, 2
        let version_id = 1; // ... create draft version
        let comp1_id = 1; // ... at position 0
        let comp2_id = 2; // ... at position 1
        let comp3_id = 3; // ... at position 2
        let service = DoxydeRmcpService::new(pool.clone(), 1);
        
        // Delete middle component
        let req = DeleteComponentRequest { component_id: comp2_id };
        service.delete_component(Parameters(req)).await;
        
        // Verify positions updated
        use doxyde_db::repositories::ComponentRepository;
        let component_repo = ComponentRepository::new(pool);
        let comp1 = component_repo.find_by_id(comp1_id).await?.unwrap();
        let comp3 = component_repo.find_by_id(comp3_id).await?.unwrap();
        
        assert_eq!(comp1.position, 0); // Should remain at 0
        assert_eq!(comp3.position, 1); // Should move from 2 to 1
        Ok(())
    }
    
    #[sqlx::test]
    async fn test_delete_component_published_fails(pool: SqlitePool) -> Result<()> {
        // Create component in published version
        let component_id = 1; // ... create in published version
        let service = DoxydeRmcpService::new(pool, 1);
        
        let req = DeleteComponentRequest { component_id };
        let result = service.delete_component(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("Cannot delete component from published version"));
        Ok(())
    }
    
    #[sqlx::test]
    async fn test_delete_component_wrong_site(pool: SqlitePool) -> Result<()> {
        // Create component in different site
        let component_id = 1; // ... create in site 2
        let service = DoxydeRmcpService::new(pool, 1); // Using site 1
        
        let req = DeleteComponentRequest { component_id };
        let result = service.delete_component(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("different site"));
        Ok(())
    }
}
```

WRITE UNIT TESTS - MAKE SURE EVERYTHING IS STILL PASSING.