# Task 13: Publish Draft Tool

## Overview
Implement the `publish_draft` tool to publish the draft version of a page, making it the live version.

## Implementation

Add this tool to the `#[tool_router]` impl block:

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct PublishDraftRequest {
    #[schemars(description = "ID of the page whose draft to publish")]
    pub page_id: i64,
}

#[tool_router]
impl DoxydeRmcpService {
    #[tool(description = "Publish the draft version of a page, making it the live version")]
    pub async fn publish_draft(&self, Parameters(req): Parameters<PublishDraftRequest>) -> String {
        match self.internal_publish_draft(req.page_id).await {
            Ok(draft_info) => {
                format!(
                    "Successfully published draft for page {}. Version {} is now live.",
                    draft_info.page_id,
                    draft_info.version_number
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
    async fn internal_publish_draft(&self, page_id: i64) -> Result<DraftInfo> {
        use doxyde_db::repositories::{PageRepository, PageVersionRepository};

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

        // Get the draft version
        let draft = version_repo
            .get_draft(page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!(
                "No draft version exists for this page. Use get_or_create_draft first."
            ))?;

        // Unpublish current published version if exists
        if let Some(current_published) = version_repo.get_published(page_id).await? {
            version_repo.unpublish(current_published.id.unwrap()).await?;
        }

        // Publish the draft
        version_repo.publish(draft.id.unwrap()).await?;

        // Get component count for the draft
        use doxyde_db::repositories::ComponentRepository;
        let component_repo = ComponentRepository::new(self.pool.clone());
        let components = component_repo
            .list_by_page_version(draft.id.unwrap())
            .await?;

        Ok(DraftInfo {
            page_id,
            version_id: draft.id.unwrap(),
            version_number: draft.version,
            created_by: draft.created_by,
            is_published: true,
            component_count: components.len() as i32,
        })
    }
}
```

## Example Request

```json
{
    "page_id": 1
}
```

## Example Response (Success)

```
Successfully published draft for page 1. Version 2 is now live.
```

## Example Response (Error)

```json
{
    "error": "No draft version exists for this page. Use get_or_create_draft first."
}
```

## Behavior

1. Verifies the page exists and belongs to the current site
2. Checks that a draft version exists
3. Unpublishes the current published version (if any)
4. Marks the draft as published
5. Returns success message with version number

## Error Cases

- Page not found
- Page belongs to different site
- No draft version exists

## Notes

- Only one version can be published at a time
- Publishing a draft doesn't delete it - it just changes its status
- After publishing, editing requires creating a new draft
- The previous published version becomes unpublished (not deleted)

## Testing

```rust
#[cfg(test)]
mod publish_draft_tests {
    use super::*;

    #[sqlx::test]
    async fn test_publish_draft(pool: SqlitePool) -> Result<()> {
        // Create page with draft
        let page_id = 1; // ... create page and draft
        let service = DoxydeRmcpService::new(pool, 1);

        let req = PublishDraftRequest { page_id };
        let result = service.publish_draft(Parameters(req)).await;

        assert!(!result.contains("error"));
        assert!(result.contains("Successfully published"));
        assert!(result.contains("is now live"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_publish_draft_no_draft(pool: SqlitePool) -> Result<()> {
        // Create page without draft
        let page_id = 1; // ... create page only
        let service = DoxydeRmcpService::new(pool, 1);

        let req = PublishDraftRequest { page_id };
        let result = service.publish_draft(Parameters(req)).await;

        assert!(result.contains("error"));
        assert!(result.contains("No draft version exists"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_publish_draft_replaces_published(pool: SqlitePool) -> Result<()> {
        // Create page with published version and new draft
        let page_id = 1; // ... create, publish, then create new draft
        let service = DoxydeRmcpService::new(pool, 1);

        let req = PublishDraftRequest { page_id };
        let result = service.publish_draft(Parameters(req)).await;

        assert!(!result.contains("error"));

        // Verify old published is unpublished
        use doxyde_db::repositories::PageVersionRepository;
        let version_repo = PageVersionRepository::new(pool);
        let versions = version_repo.list_by_page(page_id).await?;
        let published_count = versions.iter().filter(|v| v.is_published).count();
        assert_eq!(published_count, 1);
        Ok(())
    }
}
```

WRITE UNIT TESTS - MAKE SURE EVERYTHING IS STILL PASSING.
