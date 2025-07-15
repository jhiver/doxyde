use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::auth::AuthClient;
use crate::messages::{Tool, ToolContent};

pub struct ToolHandler {
    _auth_client: AuthClient,
}

impl ToolHandler {
    pub fn new(auth_client: AuthClient) -> Self {
        Self {
            _auth_client: auth_client,
        }
    }

    pub fn list_tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                name: "list_sites".to_string(),
                description: "List all sites the user has access to".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
            Tool {
                name: "get_site".to_string(),
                description: "Get details about a specific site".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "domain": {
                            "type": "string",
                            "description": "The domain of the site"
                        }
                    },
                    "required": ["domain"]
                }),
            },
            Tool {
                name: "list_pages".to_string(),
                description: "List all pages in a site".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "site_id": {
                            "type": "integer",
                            "description": "The ID of the site"
                        }
                    },
                    "required": ["site_id"]
                }),
            },
            Tool {
                name: "get_page".to_string(),
                description: "Get details about a specific page including its components"
                    .to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "page_id": {
                            "type": "integer",
                            "description": "The ID of the page"
                        }
                    },
                    "required": ["page_id"]
                }),
            },
            Tool {
                name: "create_page".to_string(),
                description: "Create a new page in a site".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "site_id": {
                            "type": "integer",
                            "description": "The ID of the site"
                        },
                        "slug": {
                            "type": "string",
                            "description": "The URL slug for the page"
                        },
                        "title": {
                            "type": "string",
                            "description": "The title of the page"
                        },
                        "parent_page_id": {
                            "type": "integer",
                            "description": "Optional parent page ID for hierarchical structure"
                        }
                    },
                    "required": ["site_id", "slug", "title"]
                }),
            },
            Tool {
                name: "update_page".to_string(),
                description: "Update a page's properties".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "page_id": {
                            "type": "integer",
                            "description": "The ID of the page to update"
                        },
                        "title": {
                            "type": "string",
                            "description": "New title for the page"
                        },
                        "description": {
                            "type": "string",
                            "description": "New description for the page"
                        },
                        "keywords": {
                            "type": "string",
                            "description": "New keywords for the page"
                        }
                    },
                    "required": ["page_id"]
                }),
            },
            Tool {
                name: "add_markdown_component".to_string(),
                description: "Add a markdown component to a page".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "page_id": {
                            "type": "integer",
                            "description": "The ID of the page"
                        },
                        "content": {
                            "type": "string",
                            "description": "The markdown content"
                        },
                        "template": {
                            "type": "string",
                            "description": "Component template: default, card, highlight, quote, hero, with_title",
                            "enum": ["default", "card", "highlight", "quote", "hero", "with_title"]
                        },
                        "title": {
                            "type": "string",
                            "description": "Optional title for the component"
                        }
                    },
                    "required": ["page_id", "content"]
                }),
            },
            Tool {
                name: "search_content".to_string(),
                description: "Search for content across all pages".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query"
                        },
                        "site_id": {
                            "type": "integer",
                            "description": "Optional: limit search to specific site"
                        }
                    },
                    "required": ["query"]
                }),
            },
        ]
    }

    pub async fn call_tool(
        &self,
        name: &str,
        args: HashMap<String, Value>,
    ) -> Result<Vec<ToolContent>> {
        // For now, return mock responses
        // In a real implementation, these would make API calls to Doxyde
        match name {
            "list_sites" => {
                let text =
                    "Available sites:\n1. localhost:3000 - My Site\n2. example.com - Example Site";
                Ok(vec![ToolContent::Text {
                    text: text.to_string(),
                }])
            }
            "get_site" => {
                let domain = args
                    .get("domain")
                    .and_then(|v| v.as_str())
                    .context("Missing required parameter: domain")?;

                let text = format!("Site: {}\nTitle: Example Site\nCreated: 2025-01-01", domain);
                Ok(vec![ToolContent::Text { text }])
            }
            "list_pages" => {
                let site_id = args
                    .get("site_id")
                    .and_then(|v| v.as_i64())
                    .context("Missing required parameter: site_id")?;

                let text = format!(
                    "Pages in site {}:\n- / (Home)\n- /about (About Us)\n- /contact (Contact)",
                    site_id
                );
                Ok(vec![ToolContent::Text { text }])
            }
            "get_page" => {
                let page_id = args
                    .get("page_id")
                    .and_then(|v| v.as_i64())
                    .context("Missing required parameter: page_id")?;

                let text = format!("Page ID: {}\nTitle: About Us\nSlug: /about\n\nComponents:\n1. Markdown (hero template): # Welcome to Our Company\n2. Markdown (default): We are a leading provider of...", page_id);
                Ok(vec![ToolContent::Text { text }])
            }
            "create_page" => {
                let title = args
                    .get("title")
                    .and_then(|v| v.as_str())
                    .context("Missing required parameter: title")?;
                let slug = args
                    .get("slug")
                    .and_then(|v| v.as_str())
                    .context("Missing required parameter: slug")?;

                let text = format!("Created page '{}' with slug '{}'", title, slug);
                Ok(vec![ToolContent::Text { text }])
            }
            "update_page" => {
                let page_id = args
                    .get("page_id")
                    .and_then(|v| v.as_i64())
                    .context("Missing required parameter: page_id")?;

                let text = format!("Updated page {}", page_id);
                Ok(vec![ToolContent::Text { text }])
            }
            "add_markdown_component" => {
                let page_id = args
                    .get("page_id")
                    .and_then(|v| v.as_i64())
                    .context("Missing required parameter: page_id")?;
                let content = args
                    .get("content")
                    .and_then(|v| v.as_str())
                    .context("Missing required parameter: content")?;
                let template = args
                    .get("template")
                    .and_then(|v| v.as_str())
                    .unwrap_or("default");
                let title = args.get("title").and_then(|v| v.as_str());

                let text = format!(
                    "Added markdown component to page {} with template '{}': {}{}",
                    page_id,
                    template,
                    if let Some(t) = title {
                        format!("Title: '{}', ", t)
                    } else {
                        String::new()
                    },
                    if content.len() > 50 {
                        &content[..50]
                    } else {
                        content
                    }
                );
                Ok(vec![ToolContent::Text { text }])
            }
            "search_content" => {
                let query = args
                    .get("query")
                    .and_then(|v| v.as_str())
                    .context("Missing required parameter: query")?;

                let text = format!("Search results for '{}':\n- Page: About Us (2 matches)\n- Page: Contact (1 match)", query);
                Ok(vec![ToolContent::Text { text }])
            }
            _ => {
                anyhow::bail!("Unknown tool: {}", name)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthCredentials;
    use reqwest::Url;

    #[test]
    fn test_list_tools() {
        let base_url = Url::parse("http://localhost:3000").unwrap();
        let creds = AuthCredentials {
            username: "test".to_string(),
            password: "test".to_string(),
        };
        let auth_client = AuthClient::new(base_url, creds).unwrap();
        let handler = ToolHandler::new(auth_client);

        let tools = handler.list_tools();
        assert!(!tools.is_empty());
        assert!(tools.iter().any(|t| t.name == "list_sites"));
    }

    #[tokio::test]
    async fn test_call_list_sites() {
        let base_url = Url::parse("http://localhost:3000").unwrap();
        let creds = AuthCredentials {
            username: "test".to_string(),
            password: "test".to_string(),
        };
        let auth_client = AuthClient::new(base_url, creds).unwrap();
        let handler = ToolHandler::new(auth_client);

        let result = handler.call_tool("list_sites", HashMap::new()).await;
        assert!(result.is_ok());
        let content = result.unwrap();
        assert!(!content.is_empty());
    }
}
