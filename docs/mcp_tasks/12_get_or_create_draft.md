# Task 12: Get or Create Draft Tool

## Overview
Implement the `get_or_create_draft` tool. This is the starting point for editing page content - it gets an existing draft or creates a new one.

## Implementation

Add this tool to the `#[tool_router]` impl block:

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetOrCreateDraftRequest {
    #[schemars(description = "The page ID to get or create draft for")]
    pub page_id: i64,
}

#[tool_router]
impl DoxydeRmcpService {
    #[tool(description = "Get existing draft or create a new one for a page. This is the starting point for editing page content. Returns draft version info and all components in the draft.")]
    pub async fn get_or_create_draft(&self, Parameters(req): Parameters<GetOrCreateDraftRequest>) -> String {
        match self.internal_get_or_create_draft(req.page_id).await {
            Ok(result) => serde_json::to_string_pretty(&result).unwrap_or_else(|e| {
                format!("{{\"error\": \"Failed to serialize result: {}\"}}", e)
            }),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }
}
```

Add the internal implementation:

```rust
impl DoxydeRmcpService {
    async fn internal_get_or_create_draft(&self, page_id: i64) -> Result<serde_json::Value> {
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

        let version_repo = PageVersionRepository::new(self.pool.clone());
        let component_repo = ComponentRepository::new(self.pool.clone());

        // Check if draft already exists
        let draft = if let Some(existing_draft) = version_repo.get_draft(page_id).await? {
            existing_draft
        } else {
            // Create new draft
            // First, check if there's a published version to copy from
            let new_version_number = if let Some(published) = version_repo.get_published(page_id).await? {
                // Copy components from published version
                let published_components = component_repo
                    .list_by_page_version(published.id.unwrap())
                    .await?;

                // Create new draft version
                let new_draft = version_repo
                    .create(page_id, published.version + 1, None)
                    .await?;

                // Copy components to new draft
                for component in published_components {
                    component_repo
                        .create(
                            new_draft.id.unwrap(),
                            &component.component_type,
                            component.position,
                            &component.template,
                            component.title.as_deref(),
                            &component.content,
                        )
                        .await?;
                }

                new_draft
            } else {
                // No published version, create version 1
                version_repo.create(page_id, 1, None).await?
            };

            new_version_number
        };

        // Get all components in the draft
        let components = component_repo
            .list_by_page_version(draft.id.unwrap())
            .await?;

        let component_infos: Vec<ComponentInfo> = components
            .into_iter()
            .map(|c| self.component_to_info(c))
            .collect();

        // Build response
        let is_new = draft.version == 1 && !draft.is_published;

        Ok(json!({
            "draft": {
                "version_id": draft.id.unwrap(),
                "version_number": draft.version,
                "is_published": draft.is_published,
                "is_new": is_new,
                "created_by": draft.created_by,
                "created_at": draft.created_at.to_rfc3339(),
            },
            "page": {
                "id": page.id.unwrap(),
                "title": page.title,
                "slug": page.slug,
                "template": page.template,
            },
            "components": component_infos,
            "component_count": component_infos.len(),
        }))
    }
}
```

## Example Request

```json
{
    "page_id": 1
}
```

## Example Response (New Draft Created)

```json
{
    "draft": {
        "version_id": 5,
        "version_number": 2,
        "is_published": false,
        "is_new": true,
        "created_by": null,
        "created_at": "2024-01-20T10:00:00Z"
    },
    "page": {
        "id": 1,
        "title": "Home",
        "slug": "home",
        "template": "default"
    },
    "components": [
        {
            "id": 10,
            "component_type": "markdown",
            "position": 0,
            "template": "default",
            "title": "Welcome",
            "content": {
                "text": "# Welcome\n\nThis content was copied from the published version."
            },
            "created_at": "2024-01-20T10:00:00Z",
            "updated_at": "2024-01-20T10:00:00Z"
        }
    ],
    "component_count": 1
}
```

## Example Response (Existing Draft)

```json
{
    "draft": {
        "version_id": 5,
        "version_number": 2,
        "is_published": false,
        "is_new": false,
        "created_by": null,
        "created_at": "2024-01-19T15:00:00Z"
    },
    "page": {
        "id": 1,
        "title": "Home",
        "slug": "home",
        "template": "default"
    },
    "components": [
        {
            "id": 10,
            "component_type": "markdown",
            "position": 0,
            "template": "default",
            "title": "Welcome (Draft)",
            "content": {
                "text": "# Welcome\n\nThis is the draft version with changes."
            },
            "created_at": "2024-01-19T15:00:00Z",
            "updated_at": "2024-01-19T16:30:00Z"
        }
    ],
    "component_count": 1
}
```

## Behavior

1. If a draft already exists, returns it with all components
2. If no draft exists but published version exists:
   - Creates new draft with version number = published version + 1
   - Copies all components from published version
3. If no versions exist at all:
   - Creates draft with version number = 1
   - No components to copy

## Notes

- This is the REQUIRED first step before any content editing
- The `is_new` flag helps UIs show appropriate messages
- Components are automatically copied when creating draft from published
- Always returns the complete draft state including all components

## Testing

```rust
#[cfg(test)]
mod get_or_create_draft_tests {
    use super::*;

    #[sqlx::test]
    async fn test_create_draft_from_published(pool: SqlitePool) -> Result<()> {
        // Create page with published version
        let page_id = 1; // ... create and publish
        let service = DoxydeRmcpService::new(pool, 1);

        let req = GetOrCreateDraftRequest { page_id };
        let result = service.get_or_create_draft(Parameters(req)).await;

        assert!(!result.contains("error"));
        let data: serde_json::Value = serde_json::from_str(&result)?;
        assert_eq!(data["draft"]["version_number"], 2);
        assert_eq!(data["draft"]["is_new"], true);
        Ok(())
    }

    #[sqlx::test]
    async fn test_get_existing_draft(pool: SqlitePool) -> Result<()> {
        // Create page with existing draft
        let page_id = 1; // ... create with draft
        let service = DoxydeRmcpService::new(pool, 1);

        // First call creates draft
        let req = GetOrCreateDraftRequest { page_id };
        service.get_or_create_draft(Parameters(req.clone())).await;

        // Second call gets existing
        let result = service.get_or_create_draft(Parameters(req)).await;

        assert!(!result.contains("error"));
        let data: serde_json::Value = serde_json::from_str(&result)?;
        assert_eq!(data["draft"]["is_new"], false);
        Ok(())
    }
}
```

WRITE UNIT TESTS - MAKE SURE EVERYTHING IS STILL PASSING.
