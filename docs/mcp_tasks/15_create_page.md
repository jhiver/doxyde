# Task 15: Create Page Tool

## Overview
Implement the `create_page` tool to create a new page with metadata for SEO.

## Implementation

Add this tool to the `#[tool_router]` impl block:

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreatePageRequest {
    #[schemars(description = "ID of the parent page (required - root pages cannot be created)")]
    pub parent_page_id: Option<i64>,
    
    #[schemars(description = "Optional URL-friendly page identifier. If not provided, will be auto-generated from title")]
    pub slug: Option<String>,
    
    #[schemars(description = "Page title")]
    pub title: String,
    
    #[schemars(description = "Page description/summary for SEO (recommended 150-160 characters). This appears in search results.")]
    pub description: Option<String>,
    
    #[schemars(description = "Comma-separated keywords for SEO (e.g., 'cms, content management, rust')")]
    pub keywords: Option<String>,
    
    #[schemars(description = "Page template (default, full_width, landing, blog)")]
    pub template: Option<String>,
}

#[tool_router]
impl DoxydeRmcpService {
    #[tool(description = "Create a new page with metadata for SEO. Always provide meaningful description and relevant keywords for better search engine visibility.")]
    pub async fn create_page(&self, Parameters(req): Parameters<CreatePageRequest>) -> String {
        match self.internal_create_page(req).await {
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
    async fn internal_create_page(&self, req: CreatePageRequest) -> Result<PageInfo> {
        use doxyde_db::repositories::{PageRepository, PageVersionRepository};
        
        let page_repo = PageRepository::new(self.pool.clone());
        
        // Verify parent page exists and belongs to this site (if provided)
        if let Some(parent_id) = req.parent_page_id {
            let parent = page_repo
                .find_by_id(parent_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Parent page not found"))?;
            
            if parent.site_id != self.site_id {
                return Err(anyhow::anyhow!("Parent page does not belong to this site"));
            }
        } else {
            // Check if root page already exists
            let existing_pages = page_repo.list_by_site_id(self.site_id).await?;
            if existing_pages.iter().any(|p| p.parent_page_id.is_none()) {
                return Err(anyhow::anyhow!("Root page already exists. New pages must have a parent."));
            }
        }
        
        // Generate slug if not provided
        let slug = req.slug.unwrap_or_else(|| {
            req.title
                .to_lowercase()
                .chars()
                .map(|c| if c.is_alphanumeric() { c } else { '-' })
                .collect::<String>()
                .split('-')
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join("-")
        });
        
        // Validate slug uniqueness within parent
        let siblings = if let Some(parent_id) = req.parent_page_id {
            page_repo.list_by_parent(parent_id).await?
        } else {
            page_repo.list_by_site_id(self.site_id).await?
                .into_iter()
                .filter(|p| p.parent_page_id.is_none())
                .collect()
        };
        
        if siblings.iter().any(|p| p.slug == slug) {
            return Err(anyhow::anyhow!("A page with slug '{}' already exists at this level", slug));
        }
        
        // Determine position (at the end)
        let position = siblings.len() as i32;
        
        // Create the page
        let template = req.template.unwrap_or_else(|| "default".to_string());
        
        let new_page = doxyde_core::models::Page {
            id: None,
            site_id: self.site_id,
            parent_page_id: req.parent_page_id,
            slug: slug.clone(),
            title: req.title.clone(),
            template: template.clone(),
            position,
            description: req.description,
            keywords: req.keywords,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };
        
        let page_id = page_repo.create(&new_page).await?;
        
        // Create initial empty version
        let version_repo = PageVersionRepository::new(self.pool.clone());
        version_repo.create(page_id, 1, None).await?;
        
        // Get all pages to build path
        let all_pages = page_repo.list_by_site_id(self.site_id).await?;
        let created_page = page_repo.find_by_id(page_id).await?.unwrap();
        
        Ok(self.page_to_info(&all_pages, &created_page).await?)
    }
}
```

## Example Request

```json
{
    "parent_page_id": 1,
    "slug": "services",
    "title": "Our Services",
    "description": "Explore our comprehensive range of professional services designed to help your business grow",
    "keywords": "services, consulting, development, support",
    "template": "default"
}
```

## Example Response

```json
{
    "id": 10,
    "slug": "services",
    "title": "Our Services",
    "path": "/services",
    "parent_id": 1,
    "position": 3,
    "has_children": false,
    "template": "default"
}
```

## Validation Rules

1. **Parent Page**: Must exist and belong to current site
2. **Root Pages**: Cannot be created if one already exists
3. **Slug**: 
   - Auto-generated from title if not provided
   - Must be unique within the same parent
   - Converted to lowercase, alphanumeric with hyphens
4. **Template**: Defaults to "default" if not provided

## Error Cases

- Parent page not found
- Parent page belongs to different site
- Root page already exists (when parent_page_id is null)
- Slug already exists at the same level

## Notes

- Creates an initial empty version (version 1) automatically
- Page is positioned at the end of its siblings
- SEO fields (description, keywords) are optional but recommended
- The response includes the generated path

## Testing

```rust
#[cfg(test)]
mod create_page_tests {
    use super::*;
    
    #[sqlx::test]
    async fn test_create_page(pool: SqlitePool) -> Result<()> {
        // Get root page ID
        let root_id = 1; // ... get root
        let service = DoxydeRmcpService::new(pool, 1);
        
        let req = CreatePageRequest {
            parent_page_id: Some(root_id),
            slug: Some("test-page".to_string()),
            title: "Test Page".to_string(),
            description: Some("Test description".to_string()),
            keywords: Some("test, page".to_string()),
            template: Some("default".to_string()),
        };
        
        let result = service.create_page(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        let page_info: PageInfo = serde_json::from_str(&result)?;
        assert_eq!(page_info.slug, "test-page");
        assert_eq!(page_info.title, "Test Page");
        Ok(())
    }
    
    #[sqlx::test]
    async fn test_create_page_auto_slug(pool: SqlitePool) -> Result<()> {
        let root_id = 1; // ... get root
        let service = DoxydeRmcpService::new(pool, 1);
        
        let req = CreatePageRequest {
            parent_page_id: Some(root_id),
            slug: None,
            title: "Test Page With Spaces!".to_string(),
            description: None,
            keywords: None,
            template: None,
        };
        
        let result = service.create_page(Parameters(req)).await;
        
        assert!(!result.contains("error"));
        let page_info: PageInfo = serde_json::from_str(&result)?;
        assert_eq!(page_info.slug, "test-page-with-spaces");
        Ok(())
    }
    
    #[sqlx::test]
    async fn test_create_page_duplicate_slug(pool: SqlitePool) -> Result<()> {
        let root_id = 1; // ... get root
        let service = DoxydeRmcpService::new(pool, 1);
        
        // Create first page
        let req1 = CreatePageRequest {
            parent_page_id: Some(root_id),
            slug: Some("duplicate".to_string()),
            title: "First Page".to_string(),
            description: None,
            keywords: None,
            template: None,
        };
        service.create_page(Parameters(req1)).await;
        
        // Try to create second with same slug
        let req2 = CreatePageRequest {
            parent_page_id: Some(root_id),
            slug: Some("duplicate".to_string()),
            title: "Second Page".to_string(),
            description: None,
            keywords: None,
            template: None,
        };
        let result = service.create_page(Parameters(req2)).await;
        
        assert!(result.contains("error"));
        assert!(result.contains("already exists"));
        Ok(())
    }
}
```