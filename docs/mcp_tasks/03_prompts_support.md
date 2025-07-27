# Task 03: Prompts Support Implementation

## Overview
Implement prompts/list functionality. For now, we'll return an empty list as the original implementation did, but the infrastructure will be in place for future prompt templates.

## Implementation Steps

### 1. Add Prompts Trait Implementation

Add this to `doxyde-shared/src/mcp/service.rs`:

```rust
use rmcp::handler::server::prompts::{Prompt, PromptsProvider};

#[async_trait]
impl PromptsProvider for DoxydeRmcpService {
    async fn list_prompts(&self) -> Result<Vec<Prompt>> {
        // Return empty prompts list for now
        // In the future, we could add prompts like:
        // - "Create a blog post"
        // - "Update page SEO metadata"
        // - "Generate component content"
        Ok(vec![])
    }
}
```

## Future Prompt Ideas

When we decide to implement prompts, here are some ideas:

```rust
async fn list_prompts(&self) -> Result<Vec<Prompt>> {
    Ok(vec![
        Prompt {
            name: "create_blog_post".to_string(),
            description: Some("Create a new blog post with SEO-optimized content".to_string()),
            arguments: vec![
                rmcp::model::PromptArgument {
                    name: "topic".to_string(),
                    description: Some("The topic or title of the blog post".to_string()),
                    required: true,
                },
                rmcp::model::PromptArgument {
                    name: "keywords".to_string(),
                    description: Some("Target SEO keywords".to_string()),
                    required: false,
                },
            ],
        },
        Prompt {
            name: "update_page_seo".to_string(),
            description: Some("Update page title, description, and keywords for better SEO".to_string()),
            arguments: vec![
                rmcp::model::PromptArgument {
                    name: "page_id".to_string(),
                    description: Some("The ID of the page to update".to_string()),
                    required: true,
                },
            ],
        },
    ])
}
```

## Notes

- Prompts are pre-defined templates that help users perform common tasks
- The empty list maintains compatibility with the original implementation
- The PromptsProvider trait is automatically used by rmcp when prompts are enabled in capabilities
- Future prompts could integrate with AI to generate content based on templates

## Testing

```rust
#[cfg(test)]
mod prompts_tests {
    use super::*;
    use sqlx::SqlitePool;

    #[sqlx::test]
    async fn test_list_prompts_empty(pool: SqlitePool) -> Result<()> {
        let service = DoxydeRmcpService::new(pool, 1);
        let prompts = service.list_prompts().await?;
        assert_eq!(prompts.len(), 0);
        Ok(())
    }
}
```

## Next Steps

WRITE UNIT TESTS - MAKE SURE EVERYTHING IS STILL PASSING.

With the core infrastructure complete (protocol handling, resources, and prompts), we can now start implementing the actual MCP tools for content management.