# Task 18: Move Page Tool

## Overview
Implement the `move_page` tool to move a page to a different parent or reorder within the same parent.

## Implementation

Add this tool to the `#[tool_router]` impl block:

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct MovePageRequest {
    #[schemars(description = "ID of the page to move")]
    pub page_id: i64,
    
    #[schemars(description = "ID of the new parent page (null for root level)")]
    pub new_parent_id: Option<i64>,
    
    #[schemars(description = "Position within the new parent (0-based). If not provided, page is added at the end.")]
    pub position: Option<i32>,
}

#[tool_router]
impl DoxydeRmcpService {
    #[tool(description = "Move a page to a different parent or reorder within the same parent. Cannot create circular references.")]
    pub async fn move_page(&self, Parameters(req): Parameters<MovePageRequest>) -> String {
        match self.internal_move_page(req).await {
            Ok(page_info) => serde_json::to_string_pretty(&page_info).unwrap_or_else(|e| {
                format!("{{\"error\": \"Failed to serialize page info: {}\"}}", e)
            }),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }
}
```

Add the internal implementation:

```rust
impl DoxydeRmcpService {
    async fn internal_move_page(&self, req: MovePageRequest) -> Result<PageInfo> {
        use doxyde_db::repositories::PageRepository;
        
        let page_repo = PageRepository::new(self.pool.clone());
        
        // Get the page to move
        let mut page = page_repo
            .find_by_id(req.page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page not found"))?;
        
        if page.site_id != self.site_id {
            return Err(anyhow::anyhow!("Page does not belong to this site"));
        }
        
        // Cannot move the root page
        if page.parent_page_id.is_none() && req.new_parent_id.is_none() {
            return Err(anyhow::anyhow!("Root page is already at the root level"));
        }
        
        if page.parent_page_id.is_none() && req.new_parent_id.is_some() {
            return Err(anyhow::anyhow!("Cannot move the root page under another page"));
        }
        
        // Verify new parent exists and belongs to same site
        if let Some(new_parent_id) = req.new_parent_id {
            let new_parent = page_repo
                .find_by_id(new_parent_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("New parent page not found"))?;
            
            if new_parent.site_id != self.site_id {
                return Err(anyhow::anyhow!("New parent page does not belong to this site"));
            }
            
            // Check for circular reference
            if self.would_create_circular_reference(req.page_id, new_parent_id).await? {
                return Err(anyhow::anyhow!("Cannot move page under its own descendant"));
            }
        }
        
        let old_parent_id = page.parent_page_id;
        let old_position = page.position;
        
        // Get siblings at destination
        let new_siblings = if let Some(parent_id) = req.new_parent_id {
            page_repo.list_by_parent(parent_id).await?
        } else {
            page_repo.list_by_site_id(self.site_id).await?
                .into_iter()
                .filter(|p| p.parent_page_id.is_none())
                .collect()
        };
        
        // Filter out the page being moved if it's already in this parent
        let mut new_siblings: Vec<_> = new_siblings
            .into_iter()
            .filter(|p| p.id != Some(req.page_id))
            .collect();
        new_siblings.sort_by_key(|p| p.position);
        
        // Determine target position
        let target_position = req.position.unwrap_or(new_siblings.len() as i32);
        let target_position = target_position.clamp(0, new_siblings.len() as i32);
        
        // Update the page
        page.parent_page_id = req.new_parent_id;
        page.position = target_position;
        page.updated_at = chrono::Utc::now();
        page_repo.update(&page).await?;
        
        // Update positions at old location (if changed parent)
        if old_parent_id != req.new_parent_id {
            let mut old_siblings = if let Some(parent_id) = old_parent_id {
                page_repo.list_by_parent(parent_id).await?
            } else {
                page_repo.list_by_site_id(self.site_id).await?
                    .into_iter()
                    .filter(|p| p.parent_page_id.is_none())
                    .collect()
            };
            old_siblings.sort_by_key(|p| p.position);
            
            for (idx, mut sibling) in old_siblings.into_iter().enumerate() {
                if sibling.position != idx as i32 {
                    sibling.position = idx as i32;
                    page_repo.update(&sibling).await?;
                }
            }
        }
        
        // Update positions at new location
        let mut all_siblings = if let Some(parent_id) = req.new_parent_id {
            page_repo.list_by_parent(parent_id).await?
        } else {
            page_repo.list_by_site_id(self.site_id).await?
                .into_iter()
                .filter(|p| p.parent_page_id.is_none())
                .collect()
        };
        all_siblings.sort_by_key(|p| {
            if p.id == Some(req.page_id) {
                target_position
            } else if p.position >= target_position {
                p.position + 1
            } else {
                p.position
            }
        });
        
        for (idx, mut sibling) in all_siblings.into_iter().enumerate() {
            if sibling.position != idx as i32 {
                sibling.position = idx as i32;
                page_repo.update(&sibling).await?;
            }
        }
        
        // Get updated page info
        let all_pages = page_repo.list_by_site_id(self.site_id).await?;
        let moved_page = page_repo.find_by_id(req.page_id).await?.unwrap();
        
        Ok(self.page_to_info(&all_pages, &moved_page).await?)
    }
    
    async fn would_create_circular_reference(&self, page_id: i64, new_parent_id: i64) -> Result<bool> {
        use doxyde_db::repositories::PageRepository;
        
        if page_id == new_parent_id {
            return Ok(true);
        }
        
        let page_repo = PageRepository::new(self.pool.clone());
        let all_pages = page_repo.list_by_site_id(self.site_id).await?;
        
        // Check if new_parent_id is a descendant of page_id
        let mut current_id = Some(new_parent_id);
        
        while let Some(id) = current_id {
            if id == page_id {
                return Ok(true);
            }
            
            current_id = all_pages
                .iter()
                .find(|p| p.id == Some(id))
                .and_then(|p| p.parent_page_id);
        }
        
        Ok(false)
    }
}
```

## Example Request

```json
{
    "page_id": 10,
    "new_parent_id": 5,
    "position": 1
}
```

## Example Response

```json
{
    "id": 10,
    "slug": "services",
    "title": "Our Services",
    "path": "/about/services",
    "parent_id": 5,
    "position": 1,
    "has_children": false,
    "template": "default"
}
```

## Validation Rules

1. **Page Existence**: Must exist and belong to current site
2. **Parent Validation**: New parent must exist and belong to same site
3. **Circular Reference**: Cannot move a page under its own descendant
4. **Root Page**: Cannot be moved
5. **Position**: Clamped to valid range (0 to sibling count)

## Error Cases

- Page not found
- Page belongs to different site
- New parent not found
- New parent belongs to different site
- Would create circular reference
- Attempting to move root page

## Behavior

1. Moves page to new parent (or root level if new_parent_id is null)
2. Updates positions at both old and new locations
3. Maintains continuous position numbering
4. Updates the page path to reflect new location

## Notes

- Position is 0-based
- If position is not provided, page is added at the end
- Moving within same parent just reorders
- Path automatically updates based on new parent hierarchy

## Testing

```rust
#[cfg(test)]
mod move_page_tests {
    use super::*;
    
    #[sqlx::test]
    async fn test_move_page_different_parent(pool: SqlitePool) -> Result<()> {
        // Create pages: root -> parent1 -> child, parent2
        let root_id = 1; // ... create
        let parent1_id = 2; // ... create
        let parent2_id = 3; // ... create
        let child_id = 4; // ... create under parent1
        let service = DoxydeRmcpService::new(pool, 1);
        
        let req = MovePageRequest {
            page_id: child_id,
            new_parent_id: Some(parent2_id),
            position: Some(0),
        };
        
        let result = service.move_page(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        let page_info: PageInfo = serde_json::from_str(&result)?;
        assert_eq!(page_info.parent_id, Some(parent2_id));
        assert_eq!(page_info.position, 0);
        Ok(())
    }
    
    #[sqlx::test]
    async fn test_move_page_reorder_same_parent(pool: SqlitePool) -> Result<()> {
        // Create pages: root -> page1(0), page2(1), page3(2)
        let root_id = 1; // ... create
        let page1_id = 2; // ... at position 0
        let page2_id = 3; // ... at position 1
        let page3_id = 4; // ... at position 2
        let service = DoxydeRmcpService::new(pool, 1);
        
        // Move page3 to position 0
        let req = MovePageRequest {
            page_id: page3_id,
            new_parent_id: Some(root_id),
            position: Some(0),
        };
        
        let result = service.move_page(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        let page_info: PageInfo = serde_json::from_str(&result)?;
        assert_eq!(page_info.position, 0);
        
        // Verify other positions updated
        use doxyde_db::repositories::PageRepository;
        let page_repo = PageRepository::new(pool);
        let page1 = page_repo.find_by_id(page1_id).await?.unwrap();
        let page2 = page_repo.find_by_id(page2_id).await?.unwrap();
        assert_eq!(page1.position, 1);
        assert_eq!(page2.position, 2);
        Ok(())
    }
    
    #[sqlx::test]
    async fn test_move_page_circular_reference(pool: SqlitePool) -> Result<()> {
        // Create pages: root -> parent -> child
        let root_id = 1; // ... create
        let parent_id = 2; // ... create
        let child_id = 3; // ... create under parent
        let service = DoxydeRmcpService::new(pool, 1);
        
        // Try to move parent under child
        let req = MovePageRequest {
            page_id: parent_id,
            new_parent_id: Some(child_id),
            position: None,
        };
        
        let result = service.move_page(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("Cannot move page under its own descendant"));
        Ok(())
    }
    
    #[sqlx::test]
    async fn test_move_root_page_fails(pool: SqlitePool) -> Result<()> {
        let root_id = 1; // ... create root
        let other_id = 2; // ... create other page
        let service = DoxydeRmcpService::new(pool, 1);
        
        let req = MovePageRequest {
            page_id: root_id,
            new_parent_id: Some(other_id),
            position: None,
        };
        
        let result = service.move_page(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("Cannot move the root page"));
        Ok(())
    }
}
```

WRITE UNIT TESTS - MAKE SURE EVERYTHING IS STILL PASSING.