# Task 14: Discard Draft Tool

## Overview
Implement the `discard_draft` tool to discard the draft version of a page, reverting to the published version.

## Implementation

Add this tool to the `#[tool_router]` impl block:

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct DiscardDraftRequest {
    #[schemars(description = "ID of the page whose draft to discard")]
    pub page_id: i64,
}

#[tool_router]
impl DoxydeRmcpService {
    #[tool(description = "Discard the draft version of a page, reverting to the published version")]
    pub async fn discard_draft(&self, Parameters(req): Parameters<DiscardDraftRequest>) -> String {
        match self.internal_discard_draft(req.page_id).await {
            Ok(_) => format!("Successfully discarded draft for page {}", req.page_id),
            Err(e) => format!("{{\"error\": \"{}\"}}", e),
        }
    }
}
```

Add the internal implementation:

```rust
impl DoxydeRmcpService {
    async fn internal_discard_draft(&self, page_id: i64) -> Result<()> {
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
                "No draft version exists for this page. Drafts are created automatically when you start editing."
            ))?;

        // Delete the draft version
        version_repo.delete(draft.id.unwrap()).await?;

        Ok(())
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
Successfully discarded draft for page 1
```

## Example Response (Error)

```json
{
    "error": "No draft version exists for this page. Drafts are created automatically when you start editing."
}
```

## Behavior

1. Verifies the page exists and belongs to the current site
2. Checks that a draft version exists
3. Deletes the draft version and all its components (cascade delete)
4. Returns success message

## Error Cases

- Page not found
- Page belongs to different site
- No draft version exists

## Side Effects

- Deletes the draft version permanently
- Also deletes all components in the draft (cascade)
- Cannot be undone

## Notes

- Only affects unpublished drafts
- Cannot discard a published version
- After discarding, a new draft will be created on next edit
- All unsaved changes in the draft are lost permanently

## Testing

```rust
#[cfg(test)]
mod discard_draft_tests {
    use super::*;

    #[sqlx::test]
    async fn test_discard_draft(pool: SqlitePool) -> Result<()> {
        // Create page with draft
        let page_id = 1; // ... create page and draft
        let service = DoxydeRmcpService::new(pool.clone(), 1);

        let req = DiscardDraftRequest { page_id };
        let result = service.discard_draft(Parameters(req)).await;

        assert!(!result.contains("error"));
        assert!(result.contains("Successfully discarded"));

        // Verify draft is gone
        use doxyde_db::repositories::PageVersionRepository;
        let version_repo = PageVersionRepository::new(pool);
        let draft = version_repo.get_draft(page_id).await?;
        assert!(draft.is_none());
        Ok(())
    }

    #[sqlx::test]
    async fn test_discard_draft_no_draft(pool: SqlitePool) -> Result<()> {
        // Create page without draft
        let page_id = 1; // ... create page only
        let service = DoxydeRmcpService::new(pool, 1);

        let req = DiscardDraftRequest { page_id };
        let result = service.discard_draft(Parameters(req)).await;

        assert!(result.contains("error"));
        assert!(result.contains("No draft version exists"));
        Ok(())
    }

    #[sqlx::test]
    async fn test_discard_draft_cascade_delete(pool: SqlitePool) -> Result<()> {
        // Create page with draft containing components
        let page_id = 1; // ... create with components
        let service = DoxydeRmcpService::new(pool.clone(), 1);

        // Get component count before
        use doxyde_db::repositories::{ComponentRepository, PageVersionRepository};
        let version_repo = PageVersionRepository::new(pool.clone());
        let draft = version_repo.get_draft(page_id).await?.unwrap();
        let component_repo = ComponentRepository::new(pool.clone());
        let components_before = component_repo
            .list_by_page_version(draft.id.unwrap())
            .await?;
        assert!(!components_before.is_empty());

        // Discard draft
        let req = DiscardDraftRequest { page_id };
        service.discard_draft(Parameters(req)).await;

        // Verify components are also deleted
        let components_after = component_repo
            .list_by_page_version(draft.id.unwrap())
            .await?;
        assert!(components_after.is_empty());
        Ok(())
    }
}
```

WRITE UNIT TESTS - MAKE SURE EVERYTHING IS STILL PASSING.
