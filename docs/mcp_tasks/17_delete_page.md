# Task 17: Delete Page Tool

## Overview
Implement the `delete_page` tool to delete a page and all its children, versions, and components.

## Implementation

Add this tool to the `#[tool_router]` impl block:

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeletePageRequest {
    #[schemars(description = "ID of the page to delete")]
    pub page_id: i64,
}

#[tool_router]
impl DoxydeRmcpService {
    #[tool(description = "Delete a page and all its children. This is a destructive operation that cannot be undone.")]
    pub async fn delete_page(&self, Parameters(req): Parameters<DeletePageRequest>) -> String {
        match self.internal_delete_page(req.page_id).await {
            Ok(deleted_count) => {
                format!(
                    "Successfully deleted page {} and {} child page(s)",
                    req.page_id,
                    deleted_count - 1  // Subtract 1 for the main page
                )
            },
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }
}
```

Add the internal implementation:

```rust
impl DoxydeRmcpService {
    async fn internal_delete_page(&self, page_id: i64) -> Result<usize> {
        use doxyde_db::repositories::PageRepository;
        
        let page_repo = PageRepository::new(self.pool.clone());
        
        // Verify page exists and belongs to this site
        let page = page_repo
            .find_by_id(page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page not found"))?;
        
        if page.site_id != self.site_id {
            return Err(anyhow::anyhow!("Page does not belong to this site"));
        }
        
        // Don't allow deleting the root page
        if page.parent_page_id.is_none() {
            return Err(anyhow::anyhow!("Cannot delete the root page"));
        }
        
        // Get all pages to find descendants
        let all_pages = page_repo.list_by_site_id(self.site_id).await?;
        
        // Collect page and all descendants
        let mut pages_to_delete = vec![page_id];
        let mut pages_to_check = vec![page_id];
        
        while !pages_to_check.is_empty() {
            let current_id = pages_to_check.pop().unwrap();
            
            // Find children of current page
            for p in &all_pages {
                if p.parent_page_id == Some(current_id) {
                    pages_to_delete.push(p.id.unwrap());
                    pages_to_check.push(p.id.unwrap());
                }
            }
        }
        
        // Delete all pages (cascade will handle versions and components)
        for id in &pages_to_delete {
            page_repo.delete(*id).await?;
        }
        
        // Update positions of remaining siblings
        if let Some(parent_id) = page.parent_page_id {
            let mut siblings = page_repo.list_by_parent(parent_id).await?;
            siblings.sort_by_key(|p| p.position);
            
            for (idx, mut sibling) in siblings.into_iter().enumerate() {
                if sibling.position != idx as i32 {
                    sibling.position = idx as i32;
                    page_repo.update(&sibling).await?;
                }
            }
        }
        
        Ok(pages_to_delete.len())
    }
}
```

## Example Request

```json
{
    "page_id": 10
}
```

## Example Response (Success)

```
Successfully deleted page 10 and 3 child page(s)
```

## Example Response (Error)

```json
{
    "error": "Cannot delete the root page"
}
```

## Behavior

1. **Cascade Delete**: Deletes the page and ALL its descendants recursively
2. **Automatic Cleanup**: Database cascade deletes all versions and components
3. **Position Update**: Reorders remaining siblings to maintain continuous positions
4. **Root Protection**: Cannot delete the root page of a site

## Error Cases

- Page not found
- Page belongs to different site
- Attempting to delete root page

## Side Effects

- Deletes all child pages (recursively)
- Deletes all page versions (cascade)
- Deletes all components in all versions (cascade)
- Updates positions of sibling pages

## Notes

- This is a destructive operation that cannot be undone
- Consider implementing soft delete in the future
- The response indicates how many pages were deleted in total
- Positions are automatically adjusted to remain continuous

## Testing

```rust
#[cfg(test)]
mod delete_page_tests {
    use super::*;
    
    #[sqlx::test]
    async fn test_delete_page_simple(pool: SqlitePool) -> Result<()> {
        // Create root and child page
        let root_id = 1; // ... create root
        let child_id = 2; // ... create child
        let service = DoxydeRmcpService::new(pool.clone(), 1);
        
        let req = DeletePageRequest { page_id: child_id };
        let result = service.delete_page(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        assert!(result.contains("Successfully deleted"));
        
        // Verify page is gone
        use doxyde_db::repositories::PageRepository;
        let page_repo = PageRepository::new(pool);
        let page = page_repo.find_by_id(child_id).await?;
        assert!(page.is_none());
        Ok(())
    }
    
    #[sqlx::test]
    async fn test_delete_page_with_children(pool: SqlitePool) -> Result<()> {
        // Create page hierarchy: root -> parent -> child1, child2
        let root_id = 1; // ... create
        let parent_id = 2; // ... create
        let child1_id = 3; // ... create
        let child2_id = 4; // ... create
        let service = DoxydeRmcpService::new(pool.clone(), 1);
        
        let req = DeletePageRequest { page_id: parent_id };
        let result = service.delete_page(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        assert!(result.contains("and 2 child page(s)"));
        
        // Verify all are gone
        use doxyde_db::repositories::PageRepository;
        let page_repo = PageRepository::new(pool);
        assert!(page_repo.find_by_id(parent_id).await?.is_none());
        assert!(page_repo.find_by_id(child1_id).await?.is_none());
        assert!(page_repo.find_by_id(child2_id).await?.is_none());
        Ok(())
    }
    
    #[sqlx::test]
    async fn test_delete_root_page_fails(pool: SqlitePool) -> Result<()> {
        let root_id = 1; // ... create root
        let service = DoxydeRmcpService::new(pool, 1);
        
        let req = DeletePageRequest { page_id: root_id };
        let result = service.delete_page(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("Cannot delete the root page"));
        Ok(())
    }
    
    #[sqlx::test]
    async fn test_delete_page_updates_positions(pool: SqlitePool) -> Result<()> {
        // Create pages: root -> page1(pos=0), page2(pos=1), page3(pos=2)
        let root_id = 1; // ... create
        let page1_id = 2; // ... create at position 0
        let page2_id = 3; // ... create at position 1
        let page3_id = 4; // ... create at position 2
        let service = DoxydeRmcpService::new(pool.clone(), 1);
        
        // Delete middle page
        let req = DeletePageRequest { page_id: page2_id };
        service.delete_page(Parameters(req)).await;
        
        // Check positions are updated
        use doxyde_db::repositories::PageRepository;
        let page_repo = PageRepository::new(pool);
        let page1 = page_repo.find_by_id(page1_id).await?.unwrap();
        let page3 = page_repo.find_by_id(page3_id).await?.unwrap();
        
        assert_eq!(page1.position, 0);
        assert_eq!(page3.position, 1); // Should move from 2 to 1
        Ok(())
    }
}
```

WRITE UNIT TESTS - MAKE SURE EVERYTHING IS STILL PASSING.