# Task 16: Update Page Tool

## Overview
Implement the `update_page` tool to update page metadata (slug, title, SEO fields, template).

## Implementation

Add this tool to the `#[tool_router]` impl block:

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdatePageRequest {
    #[schemars(description = "ID of the page to update")]
    pub page_id: i64,
    
    #[schemars(description = "New URL-friendly page identifier (optional). Will update the page path.")]
    pub slug: Option<String>,
    
    #[schemars(description = "New page title (optional)")]
    pub title: Option<String>,
    
    #[schemars(description = "New page description for SEO (optional)")]
    pub description: Option<String>,
    
    #[schemars(description = "New comma-separated keywords for SEO (optional)")]
    pub keywords: Option<String>,
    
    #[schemars(description = "New page template (optional)")]
    pub template: Option<String>,
}

#[tool_router]
impl DoxydeRmcpService {
    #[tool(description = "Update page metadata including slug, title, SEO fields, and template. Only provided fields will be updated.")]
    pub async fn update_page(&self, Parameters(req): Parameters<UpdatePageRequest>) -> String {
        match self.internal_update_page(req).await {
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
    async fn internal_update_page(&self, req: UpdatePageRequest) -> Result<PageInfo> {
        use doxyde_db::repositories::PageRepository;
        
        let page_repo = PageRepository::new(self.pool.clone());
        
        // Get the page
        let mut page = page_repo
            .find_by_id(req.page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page not found"))?;
        
        if page.site_id != self.site_id {
            return Err(anyhow::anyhow!("Page does not belong to this site"));
        }
        
        // Track if anything changed
        let mut changed = false;
        
        // Update slug if provided
        if let Some(new_slug) = req.slug {
            if new_slug != page.slug {
                // Validate slug uniqueness within same parent
                let siblings = if let Some(parent_id) = page.parent_page_id {
                    page_repo.list_by_parent(parent_id).await?
                } else {
                    page_repo.list_by_site_id(self.site_id).await?
                        .into_iter()
                        .filter(|p| p.parent_page_id.is_none())
                        .collect()
                };
                
                if siblings.iter().any(|p| p.slug == new_slug && p.id != page.id) {
                    return Err(anyhow::anyhow!("A page with slug '{}' already exists at this level", new_slug));
                }
                
                page.slug = new_slug;
                changed = true;
            }
        }
        
        // Update title if provided
        if let Some(new_title) = req.title {
            if new_title != page.title {
                page.title = new_title;
                changed = true;
            }
        }
        
        // Update description if provided
        if let Some(new_description) = req.description {
            if page.description.as_ref() != Some(&new_description) {
                page.description = Some(new_description);
                changed = true;
            }
        }
        
        // Update keywords if provided
        if let Some(new_keywords) = req.keywords {
            if page.keywords.as_ref() != Some(&new_keywords) {
                page.keywords = Some(new_keywords);
                changed = true;
            }
        }
        
        // Update template if provided
        if let Some(new_template) = req.template {
            if new_template != page.template {
                page.template = new_template;
                changed = true;
            }
        }
        
        // Only update if something changed
        if changed {
            page.updated_at = chrono::Utc::now();
            page_repo.update(&page).await?;
        }
        
        // Get all pages to build path
        let all_pages = page_repo.list_by_site_id(self.site_id).await?;
        let updated_page = page_repo.find_by_id(req.page_id).await?.unwrap();
        
        Ok(self.page_to_info(&all_pages, &updated_page).await?)
    }
}
```

## Example Request

```json
{
    "page_id": 10,
    "slug": "our-services",
    "title": "Our Professional Services",
    "description": "Discover our comprehensive range of professional services tailored to your business needs",
    "keywords": "services, consulting, development, support, enterprise",
    "template": "landing"
}
```

## Example Response

```json
{
    "id": 10,
    "slug": "our-services",
    "title": "Our Professional Services",
    "path": "/our-services",
    "parent_id": 1,
    "position": 3,
    "has_children": false,
    "template": "landing"
}
```

## Validation Rules

1. **Page Existence**: Must exist and belong to current site
2. **Slug Uniqueness**: Must be unique within same parent level
3. **Partial Updates**: Only provided fields are updated
4. **No-op Updates**: If no fields change, database is not updated

## Error Cases

- Page not found
- Page belongs to different site
- Slug already exists at same level
- Invalid template name (future validation)

## Notes

- Only updates metadata, not content (use component tools for content)
- Changing slug updates the page's URL path
- Updates the `updated_at` timestamp only if changes are made
- Response includes the updated path reflecting any slug changes

## Testing

```rust
#[cfg(test)]
mod update_page_tests {
    use super::*;
    
    #[sqlx::test]
    async fn test_update_page_all_fields(pool: SqlitePool) -> Result<()> {
        let page_id = 1; // ... create page
        let service = DoxydeRmcpService::new(pool, 1);
        
        let req = UpdatePageRequest {
            page_id,
            slug: Some("updated-slug".to_string()),
            title: Some("Updated Title".to_string()),
            description: Some("Updated description".to_string()),
            keywords: Some("updated, keywords".to_string()),
            template: Some("landing".to_string()),
        };
        
        let result = service.update_page(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        let page_info: PageInfo = serde_json::from_str(&result)?;
        assert_eq!(page_info.slug, "updated-slug");
        assert_eq!(page_info.title, "Updated Title");
        assert_eq!(page_info.template, "landing");
        Ok(())
    }
    
    #[sqlx::test]
    async fn test_update_page_partial(pool: SqlitePool) -> Result<()> {
        let page_id = 1; // ... create page
        let service = DoxydeRmcpService::new(pool, 1);
        
        // Only update title
        let req = UpdatePageRequest {
            page_id,
            slug: None,
            title: Some("New Title Only".to_string()),
            description: None,
            keywords: None,
            template: None,
        };
        
        let result = service.update_page(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        let page_info: PageInfo = serde_json::from_str(&result)?;
        assert_eq!(page_info.title, "New Title Only");
        // Other fields remain unchanged
        Ok(())
    }
    
    #[sqlx::test]
    async fn test_update_page_duplicate_slug(pool: SqlitePool) -> Result<()> {
        let page_id1 = 1; // ... create first page
        let page_id2 = 2; // ... create second page with slug "existing"
        let service = DoxydeRmcpService::new(pool, 1);
        
        let req = UpdatePageRequest {
            page_id: page_id1,
            slug: Some("existing".to_string()),
            title: None,
            description: None,
            keywords: None,
            template: None,
        };
        
        let result = service.update_page(Parameters(req)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("already exists"));
        Ok(())
    }
}
```

WRITE UNIT TESTS - MAKE SURE EVERYTHING IS STILL PASSING.