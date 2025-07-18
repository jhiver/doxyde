use crate::services::McpService;
use crate::services::mcp_service::PageInfo;
use anyhow::Result;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use doxyde_core::models::Page;

// Define JSON-RPC types
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "jsonrpc")]
pub enum JsonRpcMessage {
    #[serde(rename = "2.0")]
    Request {
        id: Value,
        method: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        params: Option<Value>,
    },
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum JsonRpcResponse {
    Success {
        jsonrpc: String,
        id: Value,
        result: Value,
    },
    Error {
        jsonrpc: String,
        id: Option<Value>,
        error: ErrorData,
    },
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ErrorData {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

pub struct SimpleMcpServer {
    pool: SqlitePool,
    site_id: i64,
}

impl SimpleMcpServer {
    pub fn new(pool: SqlitePool, site_id: i64) -> Self {
        Self { pool, site_id }
    }

    pub async fn handle_request(&self, request: Value) -> Result<Value> {
        tracing::debug!(
            "SimpleMcpServer handling request: {}",
            serde_json::to_string_pretty(&request).unwrap_or_default()
        );

        let method = extract_method(&request);
        let id = extract_id(&request);
        let params = extract_params(&request);

        tracing::debug!(
            "Extracted method: {}, id: {:?}, params: {:?}",
            method,
            id,
            params
        );

        match method {
            "initialize" => self.handle_initialize(id),
            "tools/list" => self.handle_tools_list(id),
            "tools/call" => self.handle_tools_call(id, params).await,
            "resources/list" => self.handle_resources_list(id).await,
            "prompts/list" => self.handle_prompts_list(id),
            "notifications/initialized" => self.handle_notification_initialized(),
            _ => Ok(create_error_response(id, -32601, "Method not found")),
        }
    }

    fn handle_initialize(&self, id: Value) -> Result<Value> {
        Ok(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {},
                    "resources": {
                        "list": true
                    },
                    "prompts": {
                        "list": true
                    }
                },
                "serverInfo": {
                    "name": "doxyde-mcp",
                    "version": env!("CARGO_PKG_VERSION"),
                }
            }
        }))
    }

    fn handle_tools_list(&self, id: Value) -> Result<Value> {
        Ok(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": self.get_tool_definitions()
            }
        }))
    }

    async fn handle_tools_call(&self, id: Value, params: Option<Value>) -> Result<Value> {
        tracing::debug!(
            "handle_tools_call called with id: {:?}, params: {:?}",
            id,
            params
        );

        let tool_name = match extract_tool_name(&params) {
            Ok(name) => {
                tracing::debug!("Extracted tool name: {}", name);
                name
            }
            Err(e) => {
                tracing::error!("Failed to extract tool name: {}", e);
                return Ok(create_error_response(id, -32602, &e.to_string()));
            }
        };

        let arguments = match extract_tool_arguments(&params) {
            Ok(args) => {
                tracing::debug!(
                    "Extracted arguments: {}",
                    serde_json::to_string_pretty(&args).unwrap_or_default()
                );
                args
            }
            Err(e) => {
                tracing::error!("Failed to extract arguments: {}", e);
                return Ok(create_error_response(id, -32602, &e.to_string()));
            }
        };

        tracing::debug!("Calling tool '{}' with arguments", tool_name);

        match self.call_tool(tool_name, arguments).await {
            Ok(content) => {
                tracing::debug!("Tool call successful");
                Ok(json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": content
                    }
                }))
            }
            Err(e) => {
                tracing::error!("Tool call failed: {}", e);
                Ok(create_error_response(id, -32603, &e.to_string()))
            }
        }
    }

    fn handle_notification_initialized(&self) -> Result<Value> {
        // Client notification - no response needed
        Ok(json!({}))
    }

    async fn handle_resources_list(&self, id: Value) -> Result<Value> {
        // Get pages in breadth-first order with 100 page limit
        let pages = self.get_pages_breadth_first(100).await?;
        
        // Convert pages to MCP resources
        let resources: Vec<Value> = pages.into_iter().map(|page| {
            let page_type = if page.parent_id.is_none() {
                "Homepage"
            } else {
                "Page"
            };
            
            let description = format!(
                "{} • Template: {} • Path: {}",
                page_type,
                page.template.as_deref().unwrap_or("default"),
                page.path
            );
            
            json!({
                "uri": format!("page://{}", page.id),
                "name": page.title,
                "description": description,
                "mimeType": "text/html"
            })
        }).collect();
        
        Ok(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "resources": resources
            }
        }))
    }

    fn handle_prompts_list(&self, id: Value) -> Result<Value> {
        // Return empty prompts list for now
        Ok(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "prompts": []
            }
        }))
    }

    fn get_tool_definitions(&self) -> Vec<Value> {
        vec![
            self.create_flip_coin_tool(),
            self.create_get_current_time_tool(),
            self.create_list_pages_tool(),
            self.create_get_page_tool(),
            self.create_get_page_by_path_tool(),
            self.create_get_published_content_tool(),
            self.create_get_draft_content_tool(),
            self.create_search_pages_tool(),
            self.create_create_page_tool(),
            self.create_update_page_tool(),
            self.create_delete_page_tool(),
            self.create_move_page_tool(),
            self.create_component_markdown_tool(),
            self.create_update_component_markdown_tool(),
            self.create_delete_component_tool(),
            self.create_list_components_tool(),
            self.create_get_component_tool(),
            self.create_publish_draft_tool(),
            self.create_discard_draft_tool(),
        ]
    }

    async fn call_tool(&self, name: &str, arguments: Value) -> Result<Vec<Value>> {
        match name {
            "flip_coin" => self.handle_flip_coin(arguments),
            "get_current_time" => self.handle_get_current_time(arguments),
            "list_pages" => self.handle_list_pages().await,
            "get_page" => self.handle_get_page(arguments).await,
            "get_page_by_path" => self.handle_get_page_by_path(arguments).await,
            "get_published_content" => self.handle_get_published_content(arguments).await,
            "get_draft_content" => self.handle_get_draft_content(arguments).await,
            "search_pages" => self.handle_search_pages(arguments).await,
            "create_page" => self.handle_create_page(arguments).await,
            "update_page" => self.handle_update_page(arguments).await,
            "delete_page" => self.handle_delete_page(arguments).await,
            "move_page" => self.handle_move_page(arguments).await,
            "create_component_markdown" => self.handle_create_component_markdown(arguments).await,
            "update_component_markdown" => self.handle_update_component_markdown(arguments).await,
            "delete_component" => self.handle_delete_component(arguments).await,
            "list_components" => self.handle_list_components(arguments).await,
            "get_component" => self.handle_get_component(arguments).await,
            "publish_draft" => self.handle_publish_draft(arguments).await,
            "discard_draft" => self.handle_discard_draft(arguments).await,
            _ => Err(anyhow::anyhow!("Unknown tool: {}", name)),
        }
    }

    // Tool definition methods
    fn create_flip_coin_tool(&self) -> Value {
        json!({
            "name": "flip_coin",
            "description": "Flip a coin one or more times",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "times": {
                        "type": "integer",
                        "description": "Number of times to flip (default: 1, max: 10)",
                        "minimum": 1,
                        "maximum": 10,
                        "default": 1
                    }
                }
            }
        })
    }

    fn create_get_current_time_tool(&self) -> Value {
        json!({
            "name": "get_current_time",
            "description": "Get the current time in UTC or a specified timezone",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "timezone": {
                        "type": "string",
                        "description": "Timezone (e.g., 'America/New_York', 'Europe/London')",
                        "examples": ["UTC", "America/New_York", "Europe/London", "Asia/Tokyo"]
                    }
                }
            }
        })
    }

    fn create_list_pages_tool(&self) -> Value {
        json!({
            "name": "list_pages",
            "description": "Get all pages in the site with hierarchy",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        })
    }

    fn create_get_page_tool(&self) -> Value {
        json!({
            "name": "get_page",
            "description": "Get full page details by ID",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "page_id": {
                        "type": "integer",
                        "description": "The page ID"
                    }
                },
                "required": ["page_id"]
            }
        })
    }

    fn create_get_page_by_path_tool(&self) -> Value {
        json!({
            "name": "get_page_by_path",
            "description": "Find page by URL path (e.g., '/about/team')",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "The URL path to search for"
                    }
                },
                "required": ["path"]
            }
        })
    }

    fn create_get_published_content_tool(&self) -> Value {
        json!({
            "name": "get_published_content",
            "description": "Get published content of a page",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "page_id": {
                        "type": "integer",
                        "description": "The page ID"
                    }
                },
                "required": ["page_id"]
            }
        })
    }

    fn create_get_draft_content_tool(&self) -> Value {
        json!({
            "name": "get_draft_content",
            "description": "Get draft content of a page (if exists)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "page_id": {
                        "type": "integer",
                        "description": "The page ID"
                    }
                },
                "required": ["page_id"]
            }
        })
    }

    fn create_search_pages_tool(&self) -> Value {
        json!({
            "name": "search_pages",
            "description": "Search pages by title or content",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query"
                    }
                },
                "required": ["query"]
            }
        })
    }

    fn create_create_page_tool(&self) -> Value {
        json!({
            "name": "create_page",
            "description": "Create a new page",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "parent_page_id": {
                        "type": ["integer", "null"],
                        "description": "ID of the parent page (required - root pages cannot be created)"
                    },
                    "slug": {
                        "type": ["string", "null"],
                        "description": "Optional URL-friendly page identifier. If not provided, will be auto-generated from title"
                    },
                    "title": {
                        "type": "string",
                        "description": "Page title"
                    },
                    "template": {
                        "type": ["string", "null"],
                        "description": "Page template (default, full_width, landing, blog)"
                    }
                },
                "required": ["title"]
            }
        })
    }

    fn create_update_page_tool(&self) -> Value {
        json!({
            "name": "update_page",
            "description": "Update page title, slug, or template",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "page_id": {
                        "type": ["integer", "null"],
                        "description": "ID of the page to update (required)"
                    },
                    "title": {
                        "type": ["string", "null"],
                        "description": "New page title (optional)"
                    },
                    "slug": {
                        "type": ["string", "null"],
                        "description": "New URL-friendly identifier (optional)"
                    },
                    "template": {
                        "type": ["string", "null"],
                        "description": "New page template (optional)"
                    }
                },
                "required": ["page_id"]
            }
        })
    }

    fn create_delete_page_tool(&self) -> Value {
        json!({
            "name": "delete_page",
            "description": "Delete a page (cannot delete root pages or pages with children)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "page_id": {
                        "type": "integer",
                        "description": "ID of the page to delete"
                    }
                },
                "required": ["page_id"]
            }
        })
    }

    fn create_move_page_tool(&self) -> Value {
        json!({
            "name": "move_page",
            "description": "Move a page to a new parent (cannot move root pages or create circular references)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "page_id": {
                        "type": "integer",
                        "description": "ID of the page to move"
                    },
                    "new_parent_id": {
                        "type": "integer",
                        "description": "ID of the new parent page"
                    },
                    "position": {
                        "type": ["integer", "null"],
                        "description": "Optional position within siblings (0-based)"
                    }
                },
                "required": ["page_id", "new_parent_id"]
            }
        })
    }

    fn create_component_markdown_tool(&self) -> Value {
        json!({
            "name": "create_component_markdown",
            "description": "Create a markdown component on a page",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "page_id": {
                        "type": "integer",
                        "description": "ID of the page to add the component to"
                    },
                    "text": {
                        "type": "string",
                        "description": "Markdown content text"
                    },
                    "title": {
                        "type": ["string", "null"],
                        "description": "Optional title for the component"
                    },
                    "template": {
                        "type": ["string", "null"],
                        "description": "Display template (default, with_title, card, highlight, quote, hidden, hero)"
                    }
                },
                "required": ["page_id", "text"]
            }
        })
    }

    fn create_update_component_markdown_tool(&self) -> Value {
        json!({
            "name": "update_component_markdown",
            "description": "Update a markdown component",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "component_id": {
                        "type": "integer",
                        "description": "ID of the component to update"
                    },
                    "text": {
                        "type": "string",
                        "description": "New markdown content text"
                    },
                    "title": {
                        "type": ["string", "null"],
                        "description": "Optional new title for the component"
                    },
                    "template": {
                        "type": ["string", "null"],
                        "description": "New display template (default, with_title, card, highlight, quote, hidden, hero)"
                    }
                },
                "required": ["component_id", "text"]
            }
        })
    }

    fn create_delete_component_tool(&self) -> Value {
        json!({
            "name": "delete_component",
            "description": "Delete a component from a page",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "component_id": {
                        "type": "integer",
                        "description": "ID of the component to delete"
                    }
                },
                "required": ["component_id"]
            }
        })
    }

    fn create_list_components_tool(&self) -> Value {
        json!({
            "name": "list_components",
            "description": "List all components for a page",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "page_id": {
                        "type": "integer",
                        "description": "ID of the page to list components for"
                    }
                },
                "required": ["page_id"]
            }
        })
    }

    fn create_get_component_tool(&self) -> Value {
        json!({
            "name": "get_component",
            "description": "Get details of a specific component",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "component_id": {
                        "type": "integer",
                        "description": "ID of the component to retrieve"
                    }
                },
                "required": ["component_id"]
            }
        })
    }

    fn create_publish_draft_tool(&self) -> Value {
        json!({
            "name": "publish_draft",
            "description": "Publish the draft version of a page, making it the live version",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "page_id": {
                        "type": "integer",
                        "description": "ID of the page whose draft to publish"
                    }
                },
                "required": ["page_id"]
            }
        })
    }

    fn create_discard_draft_tool(&self) -> Value {
        json!({
            "name": "discard_draft",
            "description": "Discard the draft version of a page, reverting to the published version",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "page_id": {
                        "type": "integer",
                        "description": "ID of the page whose draft to discard"
                    }
                },
                "required": ["page_id"]
            }
        })
    }

    // Tool handler methods
    fn handle_flip_coin(&self, arguments: Value) -> Result<Vec<Value>> {
        let times = extract_flip_times(&arguments);
        let results = flip_coins(times);

        Ok(vec![json!({
            "type": "text",
            "text": format_flip_results(&results)
        })])
    }

    fn handle_get_current_time(&self, arguments: Value) -> Result<Vec<Value>> {
        let timezone = extract_timezone(&arguments);
        let time = get_time_in_timezone(&timezone)?;

        Ok(vec![json!({
            "type": "text",
            "text": time
        })])
    }

    async fn handle_list_pages(&self) -> Result<Vec<Value>> {
        let service = McpService::new(self.pool.clone(), self.site_id);
        let pages = service.list_pages().await?;

        Ok(vec![json!({
            "type": "text",
            "text": serde_json::to_string_pretty(&pages)?
        })])
    }

    async fn handle_get_page(&self, arguments: Value) -> Result<Vec<Value>> {
        let page_id = extract_page_id(&arguments)?;
        let service = McpService::new(self.pool.clone(), self.site_id);
        let page = service.get_page(page_id).await?;

        Ok(vec![json!({
            "type": "text",
            "text": serde_json::to_string_pretty(&page)?
        })])
    }

    async fn handle_get_page_by_path(&self, arguments: Value) -> Result<Vec<Value>> {
        let path = extract_path(&arguments)?;
        let service = McpService::new(self.pool.clone(), self.site_id);
        let page = service.get_page_by_path(&path).await?;

        Ok(vec![json!({
            "type": "text",
            "text": serde_json::to_string_pretty(&page)?
        })])
    }

    async fn handle_get_published_content(&self, arguments: Value) -> Result<Vec<Value>> {
        let page_id = extract_page_id(&arguments)?;
        let service = McpService::new(self.pool.clone(), self.site_id);
        let content = service.get_published_content(page_id).await?;

        Ok(vec![json!({
            "type": "text",
            "text": serde_json::to_string_pretty(&content)?
        })])
    }

    async fn handle_get_draft_content(&self, arguments: Value) -> Result<Vec<Value>> {
        let page_id = extract_page_id(&arguments)?;
        let service = McpService::new(self.pool.clone(), self.site_id);

        match service.get_draft_content(page_id).await? {
            Some(content) => Ok(vec![json!({
                "type": "text",
                "text": serde_json::to_string_pretty(&content)?
            })]),
            None => Ok(vec![json!({
                "type": "text",
                "text": "No draft version exists for this page"
            })]),
        }
    }

    async fn handle_search_pages(&self, arguments: Value) -> Result<Vec<Value>> {
        let query = extract_query(&arguments)?;
        let service = McpService::new(self.pool.clone(), self.site_id);
        let results = service.search_pages(&query).await?;

        Ok(vec![json!({
            "type": "text",
            "text": serde_json::to_string_pretty(&results)?
        })])
    }

    async fn handle_create_page(&self, arguments: Value) -> Result<Vec<Value>> {
        let params = extract_create_page_params(&arguments)?;
        let service = McpService::new(self.pool.clone(), self.site_id);

        let page_info = service
            .create_page(
                params.parent_page_id,
                params.slug,
                params.title,
                params.template,
            )
            .await?;

        Ok(vec![json!({
            "type": "text",
            "text": serde_json::to_string_pretty(&page_info)?
        })])
    }

    async fn handle_update_page(&self, arguments: Value) -> Result<Vec<Value>> {
        tracing::info!(
            "handle_update_page called with arguments: {}",
            serde_json::to_string_pretty(&arguments).unwrap_or_default()
        );

        let params = extract_update_page_params(&arguments)?;
        let service = McpService::new(self.pool.clone(), self.site_id);

        let page_info = service
            .update_page(params.page_id, params.title, params.slug, params.template)
            .await?;

        Ok(vec![json!({
            "type": "text",
            "text": serde_json::to_string_pretty(&page_info)?
        })])
    }

    async fn handle_delete_page(&self, arguments: Value) -> Result<Vec<Value>> {
        let page_id = extract_page_id(&arguments)?;
        let service = McpService::new(self.pool.clone(), self.site_id);

        service.delete_page(page_id).await?;

        Ok(vec![json!({
            "type": "text",
            "text": format!("Successfully deleted page with ID {}", page_id)
        })])
    }

    async fn handle_move_page(&self, arguments: Value) -> Result<Vec<Value>> {
        let params = extract_move_page_params(&arguments)?;
        let service = McpService::new(self.pool.clone(), self.site_id);

        let page_info = service
            .move_page(params.page_id, params.new_parent_id, params.position)
            .await?;

        Ok(vec![json!({
            "type": "text",
            "text": serde_json::to_string_pretty(&page_info)?
        })])
    }

    async fn handle_create_component_markdown(&self, arguments: Value) -> Result<Vec<Value>> {
        let params = extract_create_component_markdown_params(&arguments)?;
        let service = McpService::new(self.pool.clone(), self.site_id);

        let component_info = service
            .create_component_markdown(params.page_id, params.text, params.title, params.template)
            .await?;

        Ok(vec![json!({
            "type": "text",
            "text": serde_json::to_string_pretty(&component_info)?
        })])
    }

    async fn handle_update_component_markdown(&self, arguments: Value) -> Result<Vec<Value>> {
        let params = extract_update_component_markdown_params(&arguments)?;
        let service = McpService::new(self.pool.clone(), self.site_id);

        let component_info = service
            .update_component_markdown(params.component_id, params.text, params.title, params.template)
            .await?;

        Ok(vec![json!({
            "type": "text",
            "text": serde_json::to_string_pretty(&component_info)?
        })])
    }

    async fn handle_delete_component(&self, arguments: Value) -> Result<Vec<Value>> {
        let component_id = extract_component_id(&arguments)?;
        let service = McpService::new(self.pool.clone(), self.site_id);

        service.delete_component(component_id).await?;

        Ok(vec![json!({
            "type": "text",
            "text": format!("Successfully deleted component with ID {}", component_id)
        })])
    }

    async fn handle_list_components(&self, arguments: Value) -> Result<Vec<Value>> {
        let page_id = extract_page_id(&arguments)?;
        let service = McpService::new(self.pool.clone(), self.site_id);

        let components = service.list_components(page_id).await?;

        Ok(vec![json!({
            "type": "text",
            "text": serde_json::to_string_pretty(&components)?
        })])
    }

    async fn handle_get_component(&self, arguments: Value) -> Result<Vec<Value>> {
        let component_id = extract_component_id(&arguments)?;
        let service = McpService::new(self.pool.clone(), self.site_id);

        let component_info = service.get_component(component_id).await?;

        Ok(vec![json!({
            "type": "text",
            "text": serde_json::to_string_pretty(&component_info)?
        })])
    }

    async fn handle_publish_draft(&self, arguments: Value) -> Result<Vec<Value>> {
        let page_id = extract_page_id(&arguments)?;
        let service = McpService::new(self.pool.clone(), self.site_id);
        let draft_info = service.publish_draft(page_id).await?;
        Ok(vec![json!({
            "type": "text",
            "text": format!(
                "Successfully published draft for page {}. Version {} is now live.",
                draft_info.page_id,
                draft_info.version_number
            )
        })])
    }

    async fn handle_discard_draft(&self, arguments: Value) -> Result<Vec<Value>> {
        let page_id = extract_page_id(&arguments)?;
        let service = McpService::new(self.pool.clone(), self.site_id);
        service.discard_draft(page_id).await?;
        Ok(vec![json!({
            "type": "text",
            "text": format!("Successfully discarded draft for page {}", page_id)
        })])
    }

    // Helper method to get pages in breadth-first order with limit
    async fn get_pages_breadth_first(&self, limit: usize) -> Result<Vec<PageInfo>> {
        use doxyde_db::repositories::PageRepository;
        use std::collections::VecDeque;
        
        let page_repo = PageRepository::new(self.pool.clone());
        let all_pages = page_repo.list_by_site_id(self.site_id).await?;
        
        // Create a map of page_id to children
        let mut children_map: std::collections::HashMap<Option<i64>, Vec<&Page>> = std::collections::HashMap::new();
        
        for page in &all_pages {
            children_map.entry(page.parent_page_id).or_insert_with(Vec::new).push(page);
        }
        
        // Sort children by position
        for children in children_map.values_mut() {
            children.sort_by_key(|p| p.position);
        }
        
        // Breadth-first traversal
        let mut result = Vec::new();
        let mut queue = VecDeque::new();
        
        // Start with root pages (parent_page_id = None)
        if let Some(roots) = children_map.get(&None) {
            for root in roots {
                queue.push_back(root);
            }
        }
        
        // Process queue in breadth-first order
        while let Some(page) = queue.pop_front() {
            if result.len() >= limit {
                break;
            }
            
            // Convert to PageInfo
            let page_info = PageInfo {
                id: page.id.unwrap(),
                slug: page.slug.clone(),
                title: page.title.clone(),
                path: self.build_page_path(&all_pages, page).await?,
                parent_id: page.parent_page_id,
                position: page.position,
                has_children: children_map.contains_key(&page.id),
                template: Some(page.template.clone()),
            };
            
            result.push(page_info);
            
            // Add children to queue
            if let Some(children) = children_map.get(&page.id) {
                for child in children {
                    queue.push_back(child);
                }
            }
        }
        
        Ok(result)
    }
    
    // Helper to build page path
    async fn build_page_path(&self, all_pages: &[Page], page: &Page) -> Result<String> {
        // Special case for root page
        if page.parent_page_id.is_none() {
            return Ok("/".to_string());
        }
        
        let mut path_parts = vec![page.slug.clone()];
        let mut current_parent = page.parent_page_id;
        
        while let Some(parent_id) = current_parent {
            if let Some(parent) = all_pages.iter().find(|p| p.id == Some(parent_id)) {
                // Don't include root page slug in path
                if parent.parent_page_id.is_some() {
                    path_parts.push(parent.slug.clone());
                }
                current_parent = parent.parent_page_id;
            } else {
                break;
            }
        }
        
        path_parts.reverse();
        Ok(format!("/{}", path_parts.join("/")))
    }
}

// Helper functions
fn extract_method(request: &Value) -> &str {
    request.get("method").and_then(|v| v.as_str()).unwrap_or("")
}

fn extract_id(request: &Value) -> Value {
    request.get("id").cloned().unwrap_or(json!(null))
}

fn extract_params(request: &Value) -> Option<Value> {
    request.get("params").cloned()
}

fn extract_tool_name(params: &Option<Value>) -> Result<&str> {
    params
        .as_ref()
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .ok_or_else(|| anyhow::anyhow!("Tool name is required"))
}

fn extract_tool_arguments(params: &Option<Value>) -> Result<Value> {
    Ok(params
        .as_ref()
        .and_then(|p| p.get("arguments"))
        .cloned()
        .unwrap_or(json!({})))
}

fn extract_flip_times(arguments: &Value) -> usize {
    arguments
        .get("times")
        .and_then(|t| t.as_u64())
        .unwrap_or(1)
        .clamp(1, 10) as usize
}

fn flip_coins(times: usize) -> Vec<&'static str> {
    (0..times)
        .map(|_| {
            if rand::random::<bool>() {
                "heads"
            } else {
                "tails"
            }
        })
        .collect()
}

fn format_flip_results(results: &[&str]) -> String {
    if results.len() == 1 {
        format!("The coin landed on: {}", results[0])
    } else {
        let mut output = format!("Flipped {} times:\n", results.len());
        for (i, result) in results.iter().enumerate() {
            output.push_str(&format!("Flip {}: {}\n", i + 1, result));
        }
        output
    }
}

fn extract_timezone(arguments: &Value) -> Option<String> {
    arguments
        .get("timezone")
        .and_then(|v| v.as_str())
        .map(String::from)
}

fn get_time_in_timezone(timezone: &Option<String>) -> Result<String> {
    use chrono::Utc;

    match timezone {
        None => Ok(format!(
            "Current UTC time: {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        )),
        Some(tz_str) => {
            use chrono_tz::Tz;
            use std::str::FromStr;

            let tz = Tz::from_str(tz_str)
                .map_err(|_| anyhow::anyhow!("Invalid timezone: {}", tz_str))?;
            let now = Utc::now().with_timezone(&tz);
            Ok(format!(
                "Current time in {}: {}",
                tz_str,
                now.format("%Y-%m-%d %H:%M:%S %Z")
            ))
        }
    }
}

fn extract_page_id(arguments: &Value) -> Result<i64> {
    arguments
        .get("page_id")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| anyhow::anyhow!("page_id is required"))
}

fn extract_path(arguments: &Value) -> Result<String> {
    arguments
        .get("path")
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| anyhow::anyhow!("path is required"))
}

fn extract_query(arguments: &Value) -> Result<String> {
    arguments
        .get("query")
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| anyhow::anyhow!("query is required"))
}

struct CreatePageParams {
    parent_page_id: Option<i64>,
    slug: Option<String>,
    title: String,
    template: Option<String>,
}

struct UpdatePageParams {
    page_id: i64,
    title: Option<String>,
    slug: Option<String>,
    template: Option<String>,
}

struct MovePageParams {
    page_id: i64,
    new_parent_id: i64,
    position: Option<i32>,
}

struct CreateComponentMarkdownParams {
    page_id: i64,
    text: String,
    title: Option<String>,
    template: Option<String>,
}

struct UpdateComponentMarkdownParams {
    component_id: i64,
    text: String,
    title: Option<String>,
    template: Option<String>,
}

fn extract_create_page_params(arguments: &Value) -> Result<CreatePageParams> {
    let parent_page_id = arguments.get("parent_page_id").and_then(|v| v.as_i64());

    let slug = arguments
        .get("slug")
        .and_then(|v| v.as_str())
        .map(String::from);

    let title = arguments
        .get("title")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("title is required"))?
        .to_string();

    let template = arguments
        .get("template")
        .and_then(|v| v.as_str())
        .map(String::from);

    Ok(CreatePageParams {
        parent_page_id,
        slug,
        title,
        template,
    })
}

fn extract_update_page_params(arguments: &Value) -> Result<UpdatePageParams> {
    tracing::info!(
        "extract_update_page_params - Raw arguments: {}",
        serde_json::to_string_pretty(arguments).unwrap_or_default()
    );

    let page_id = extract_page_id(&arguments)?;

    let title = arguments
        .get("title")
        .and_then(|v| v.as_str())
        .map(String::from);

    let slug = arguments
        .get("slug")
        .and_then(|v| v.as_str())
        .map(String::from);

    let template = arguments
        .get("template")
        .and_then(|v| v.as_str())
        .map(String::from);

    Ok(UpdatePageParams {
        page_id,
        title,
        slug,
        template,
    })
}

fn extract_move_page_params(arguments: &Value) -> Result<MovePageParams> {
    let page_id = arguments
        .get("page_id")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| anyhow::anyhow!("page_id is required"))?;

    let new_parent_id = arguments
        .get("new_parent_id")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| anyhow::anyhow!("new_parent_id is required"))?;

    let position = arguments
        .get("position")
        .and_then(|v| v.as_i64())
        .map(|p| p as i32);

    Ok(MovePageParams {
        page_id,
        new_parent_id,
        position,
    })
}

fn extract_component_id(arguments: &Value) -> Result<i64> {
    arguments
        .get("component_id")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| anyhow::anyhow!("component_id is required"))
}

fn extract_create_component_markdown_params(arguments: &Value) -> Result<CreateComponentMarkdownParams> {
    let page_id = arguments
        .get("page_id")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| anyhow::anyhow!("page_id is required"))?;

    let text = arguments
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("text is required"))?
        .to_string();

    let title = arguments
        .get("title")
        .and_then(|v| v.as_str())
        .map(String::from);

    let template = arguments
        .get("template")
        .and_then(|v| v.as_str())
        .map(String::from);

    Ok(CreateComponentMarkdownParams {
        page_id,
        text,
        title,
        template,
    })
}

fn extract_update_component_markdown_params(arguments: &Value) -> Result<UpdateComponentMarkdownParams> {
    let component_id = arguments
        .get("component_id")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| anyhow::anyhow!("component_id is required"))?;

    let text = arguments
        .get("text")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("text is required"))?
        .to_string();

    let title = arguments
        .get("title")
        .and_then(|v| v.as_str())
        .map(String::from);

    let template = arguments
        .get("template")
        .and_then(|v| v.as_str())
        .map(String::from);

    Ok(UpdateComponentMarkdownParams {
        component_id,
        text,
        title,
        template,
    })
}

fn create_error_response(id: Value, code: i32, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_method() {
        let request = json!({"method": "test_method"});
        assert_eq!(extract_method(&request), "test_method");

        let request = json!({});
        assert_eq!(extract_method(&request), "");

        let request = json!({"method": 123});
        assert_eq!(extract_method(&request), "");
    }

    #[test]
    fn test_extract_id() {
        let request = json!({"id": 123});
        assert_eq!(extract_id(&request), json!(123));

        let request = json!({"id": "test"});
        assert_eq!(extract_id(&request), json!("test"));

        let request = json!({});
        assert_eq!(extract_id(&request), json!(null));
    }

    #[test]
    fn test_extract_params() {
        let request = json!({"params": {"key": "value"}});
        assert_eq!(extract_params(&request), Some(json!({"key": "value"})));

        let request = json!({});
        assert_eq!(extract_params(&request), None);
    }

    #[test]
    fn test_extract_tool_name() {
        let params = Some(json!({"name": "test_tool"}));
        assert_eq!(extract_tool_name(&params).unwrap(), "test_tool");

        let params = Some(json!({}));
        assert!(extract_tool_name(&params).is_err());

        let params = None;
        assert!(extract_tool_name(&params).is_err());
    }

    #[test]
    fn test_extract_flip_times() {
        let args = json!({"times": 5});
        assert_eq!(extract_flip_times(&args), 5);

        let args = json!({"times": 0});
        assert_eq!(extract_flip_times(&args), 1);

        let args = json!({"times": 20});
        assert_eq!(extract_flip_times(&args), 10);

        let args = json!({});
        assert_eq!(extract_flip_times(&args), 1);
    }

    #[test]
    fn test_format_flip_results() {
        let results = vec!["heads"];
        assert_eq!(format_flip_results(&results), "The coin landed on: heads");

        let results = vec!["heads", "tails"];
        let formatted = format_flip_results(&results);
        assert!(formatted.contains("Flipped 2 times:"));
        assert!(formatted.contains("Flip 1: heads"));
        assert!(formatted.contains("Flip 2: tails"));
    }

    #[test]
    fn test_extract_page_id() {
        let args = json!({"page_id": 123});
        assert_eq!(extract_page_id(&args).unwrap(), 123);

        let args = json!({});
        assert!(extract_page_id(&args).is_err());

        let args = json!({"page_id": "not_a_number"});
        assert!(extract_page_id(&args).is_err());
    }

    #[test]
    fn test_extract_path() {
        let args = json!({"path": "/about"});
        assert_eq!(extract_path(&args).unwrap(), "/about");

        let args = json!({});
        assert!(extract_path(&args).is_err());
    }

    #[test]
    fn test_extract_query() {
        let args = json!({"query": "search term"});
        assert_eq!(extract_query(&args).unwrap(), "search term");

        let args = json!({});
        assert!(extract_query(&args).is_err());
    }

    #[test]
    fn test_extract_create_page_params() {
        let args = json!({
            "parent_page_id": 1,
            "slug": "test-page",
            "title": "Test Page",
            "template": "default"
        });
        let params = extract_create_page_params(&args).unwrap();
        assert_eq!(params.parent_page_id, Some(1));
        assert_eq!(params.slug, Some("test-page".to_string()));
        assert_eq!(params.title, "Test Page");
        assert_eq!(params.template, Some("default".to_string()));

        let args = json!({
            "slug": "test-page",
            "title": "Test Page"
        });
        let params = extract_create_page_params(&args).unwrap();
        assert_eq!(params.parent_page_id, None);
        assert_eq!(params.template, None);

        // Test without slug - should work now
        let args = json!({"title": "Test Page"});
        let params = extract_create_page_params(&args).unwrap();
        assert_eq!(params.slug, None);
        assert_eq!(params.title, "Test Page");

        // Test missing title - should fail
        let args = json!({"slug": "test"});
        assert!(extract_create_page_params(&args).is_err());
    }

    #[test]
    fn test_extract_update_page_params() {
        let args = json!({
            "page_id": 1,
            "title": "New Title",
            "slug": "new-slug",
            "template": "blog"
        });
        let params = extract_update_page_params(&args).unwrap();
        assert_eq!(params.page_id, 1);
        assert_eq!(params.title, Some("New Title".to_string()));
        assert_eq!(params.slug, Some("new-slug".to_string()));
        assert_eq!(params.template, Some("blog".to_string()));

        // Test with only page_id
        let args = json!({"page_id": 2});
        let params = extract_update_page_params(&args).unwrap();
        assert_eq!(params.page_id, 2);
        assert_eq!(params.title, None);
        assert_eq!(params.slug, None);
        assert_eq!(params.template, None);

        // Test missing page_id
        let args = json!({"title": "New Title"});
        assert!(extract_update_page_params(&args).is_err());
    }

    #[test]
    fn test_create_error_response() {
        let response = create_error_response(json!(123), -32601, "Method not found");
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 123);
        assert_eq!(response["error"]["code"], -32601);
        assert_eq!(response["error"]["message"], "Method not found");
    }

    // Integration tests
    use crate::test_helpers::{create_test_app_state, create_test_site};

    async fn create_test_server() -> Result<SimpleMcpServer> {
        let state = create_test_app_state().await?;
        let site = create_test_site(&state.db, "test.com", "Test Site").await?;
        Ok(SimpleMcpServer::new(state.db, site.id.unwrap()))
    }

    #[tokio::test]
    async fn test_initialize() -> Result<()> {
        let server = create_test_server().await?;
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        });

        let response = server.handle_request(request).await?;
        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 1);
        assert!(response["result"]["serverInfo"].is_object());

        Ok(())
    }

    #[tokio::test]
    async fn test_list_tools() -> Result<()> {
        let server = create_test_server().await?;
        let request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list"
        });

        let response = server.handle_request(request).await?;
        let tools = response["result"]["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 19); // 2 demo + 6 Phase 1 tools + 4 write tools + 5 component tools + 2 draft tools

        Ok(())
    }

    #[tokio::test]
    async fn test_flip_coin() -> Result<()> {
        let server = create_test_server().await?;
        let request = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "flip_coin",
                "arguments": {"times": 2}
            }
        });

        let response = server.handle_request(request).await?;
        let content = &response["result"]["content"][0];
        assert_eq!(content["type"], "text");
        assert!(content["text"].as_str().unwrap().contains("Flip 1:"));
        assert!(content["text"].as_str().unwrap().contains("Flip 2:"));

        Ok(())
    }

    #[tokio::test]
    async fn test_create_page() -> Result<()> {
        let server = create_test_server().await?;

        // First, get the root page ID
        let list_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "list_pages",
                "arguments": {}
            }
        });

        let list_response = server.handle_request(list_request).await?;
        let pages_text = list_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let pages: Vec<serde_json::Value> = serde_json::from_str(pages_text)?;
        let root_page_id = pages[0]["page"]["id"].as_i64().unwrap();

        // Now create a new page
        let request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "create_page",
                "arguments": {
                    "parent_page_id": root_page_id,
                    "slug": "test-page",
                    "title": "Test Page",
                    "template": "default"
                }
            }
        });

        let response = server.handle_request(request).await?;
        let content = &response["result"]["content"][0];
        assert_eq!(content["type"], "text");

        let page_info_text = content["text"].as_str().unwrap();
        let page_info: serde_json::Value = serde_json::from_str(page_info_text)?;

        assert_eq!(page_info["slug"], "test-page");
        assert_eq!(page_info["title"], "Test Page");
        assert_eq!(page_info["path"], "/test-page");
        assert_eq!(page_info["parent_id"], root_page_id);
        assert_eq!(page_info["template"], "default");

        Ok(())
    }

    #[tokio::test]
    async fn test_update_page() -> Result<()> {
        let server = create_test_server().await?;

        // First, get the root page and create a test page
        let list_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "list_pages",
                "arguments": {}
            }
        });

        let list_response = server.handle_request(list_request).await?;
        let pages_text = list_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let pages: Vec<serde_json::Value> = serde_json::from_str(pages_text)?;
        let root_page_id = pages[0]["page"]["id"].as_i64().unwrap();

        // Create a page to update
        let create_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "create_page",
                "arguments": {
                    "parent_page_id": root_page_id,
                    "slug": "page-to-update",
                    "title": "Original Title",
                    "template": "default"
                }
            }
        });

        let create_response = server.handle_request(create_request).await?;
        let page_text = create_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let created_page: serde_json::Value = serde_json::from_str(page_text)?;
        let page_id = created_page["id"].as_i64().unwrap();

        // Now update the page
        let update_request = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "update_page",
                "arguments": {
                    "page_id": page_id,
                    "title": "Updated Title",
                    "slug": "updated-slug",
                    "template": "full_width"
                }
            }
        });

        let update_response = server.handle_request(update_request).await?;
        let content = &update_response["result"]["content"][0];
        assert_eq!(content["type"], "text");

        let updated_page_text = content["text"].as_str().unwrap();
        let updated_page: serde_json::Value = serde_json::from_str(updated_page_text)?;

        assert_eq!(updated_page["id"], page_id);
        assert_eq!(updated_page["slug"], "updated-slug");
        assert_eq!(updated_page["title"], "Updated Title");
        assert_eq!(updated_page["path"], "/updated-slug");
        assert_eq!(updated_page["template"], "full_width");

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_page() -> Result<()> {
        let server = create_test_server().await?;

        // First, get the root page to create test pages under
        let list_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "list_pages",
                "arguments": {}
            }
        });

        let list_response = server.handle_request(list_request).await?;
        let pages_text = list_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let pages: Vec<serde_json::Value> = serde_json::from_str(pages_text)?;
        let root_page_id = pages[0]["page"]["id"].as_i64().unwrap();

        // Create a page to delete
        let create_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "create_page",
                "arguments": {
                    "parent_page_id": root_page_id,
                    "slug": "page-to-delete",
                    "title": "Page to Delete",
                    "template": "default"
                }
            }
        });

        let create_response = server.handle_request(create_request).await?;
        let page_text = create_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let created_page: serde_json::Value = serde_json::from_str(page_text)?;
        let page_id = created_page["id"].as_i64().unwrap();

        // Now delete the page
        let delete_request = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "delete_page",
                "arguments": {
                    "page_id": page_id
                }
            }
        });

        let delete_response = server.handle_request(delete_request).await?;
        let content = &delete_response["result"]["content"][0];
        assert_eq!(content["type"], "text");
        assert!(content["text"]
            .as_str()
            .unwrap()
            .contains("Successfully deleted"));

        // Verify page is deleted by trying to get it
        let get_request = json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "get_page",
                "arguments": {
                    "page_id": page_id
                }
            }
        });

        let get_response = server.handle_request(get_request).await?;
        assert!(get_response.get("error").is_some());

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_page_with_children_fails() -> Result<()> {
        let server = create_test_server().await?;

        // Get root page
        let list_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "list_pages",
                "arguments": {}
            }
        });

        let list_response = server.handle_request(list_request).await?;
        let pages_text = list_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let pages: Vec<serde_json::Value> = serde_json::from_str(pages_text)?;
        let root_page_id = pages[0]["page"]["id"].as_i64().unwrap();

        // Create parent page
        let create_parent_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "create_page",
                "arguments": {
                    "parent_page_id": root_page_id,
                    "slug": "parent-page",
                    "title": "Parent Page",
                    "template": "default"
                }
            }
        });

        let create_parent_response = server.handle_request(create_parent_request).await?;
        let parent_text = create_parent_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let parent_page: serde_json::Value = serde_json::from_str(parent_text)?;
        let parent_id = parent_page["id"].as_i64().unwrap();

        // Create child page
        let create_child_request = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "create_page",
                "arguments": {
                    "parent_page_id": parent_id,
                    "slug": "child-page",
                    "title": "Child Page",
                    "template": "default"
                }
            }
        });

        server.handle_request(create_child_request).await?;

        // Try to delete parent page - should fail
        let delete_request = json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "delete_page",
                "arguments": {
                    "page_id": parent_id
                }
            }
        });

        let delete_response = server.handle_request(delete_request).await?;
        assert!(delete_response.get("error").is_some());
        let error_msg = delete_response["error"]["message"].as_str().unwrap();
        assert!(error_msg.contains("Cannot delete page") && error_msg.contains("child"));

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_root_page_fails() -> Result<()> {
        let server = create_test_server().await?;

        // Get root page
        let list_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "list_pages",
                "arguments": {}
            }
        });

        let list_response = server.handle_request(list_request).await?;
        let pages_text = list_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let pages: Vec<serde_json::Value> = serde_json::from_str(pages_text)?;
        let root_page_id = pages[0]["page"]["id"].as_i64().unwrap();

        // Try to delete root page - should fail
        let delete_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "delete_page",
                "arguments": {
                    "page_id": root_page_id
                }
            }
        });

        let delete_response = server.handle_request(delete_request).await?;
        assert!(delete_response.get("error").is_some());
        let error_msg = delete_response["error"]["message"].as_str().unwrap();
        assert!(error_msg.contains("Cannot delete root page"));

        Ok(())
    }

    #[tokio::test]
    async fn test_move_page() -> Result<()> {
        let server = create_test_server().await?;

        // Get root page to create test pages
        let list_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "list_pages",
                "arguments": {}
            }
        });

        let list_response = server.handle_request(list_request).await?;
        let pages_text = list_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let pages: Vec<serde_json::Value> = serde_json::from_str(pages_text)?;
        let root_page_id = pages[0]["page"]["id"].as_i64().unwrap();

        // Create two pages
        let create_page1 = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "create_page",
                "arguments": {
                    "parent_page_id": root_page_id,
                    "slug": "page1",
                    "title": "Page 1"
                }
            }
        });

        let page1_response = server.handle_request(create_page1).await?;
        let page1_text = page1_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let page1: serde_json::Value = serde_json::from_str(page1_text)?;
        let page1_id = page1["id"].as_i64().unwrap();

        let create_page2 = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "create_page",
                "arguments": {
                    "parent_page_id": root_page_id,
                    "slug": "page2",
                    "title": "Page 2"
                }
            }
        });

        let page2_response = server.handle_request(create_page2).await?;
        let page2_text = page2_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let page2: serde_json::Value = serde_json::from_str(page2_text)?;
        let page2_id = page2["id"].as_i64().unwrap();

        // Move page1 under page2
        let move_request = json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "move_page",
                "arguments": {
                    "page_id": page1_id,
                    "new_parent_id": page2_id
                }
            }
        });

        let move_response = server.handle_request(move_request).await?;
        let content = &move_response["result"]["content"][0];
        assert_eq!(content["type"], "text");

        let moved_page_text = content["text"].as_str().unwrap();
        let moved_page: serde_json::Value = serde_json::from_str(moved_page_text)?;

        assert_eq!(moved_page["id"], page1_id);
        assert_eq!(moved_page["parent_id"], page2_id);
        assert_eq!(moved_page["path"], "/page2/page1");

        Ok(())
    }

    #[tokio::test]
    async fn test_move_page_root_fails() -> Result<()> {
        let server = create_test_server().await?;

        // Get root page
        let list_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "list_pages",
                "arguments": {}
            }
        });

        let list_response = server.handle_request(list_request).await?;
        let pages_text = list_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let pages: Vec<serde_json::Value> = serde_json::from_str(pages_text)?;
        let root_page_id = pages[0]["page"]["id"].as_i64().unwrap();

        // Create a page to serve as potential parent
        let create_page = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "create_page",
                "arguments": {
                    "parent_page_id": root_page_id,
                    "slug": "page1",
                    "title": "Page 1"
                }
            }
        });

        let page_response = server.handle_request(create_page).await?;
        let page_text = page_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let page: serde_json::Value = serde_json::from_str(page_text)?;
        let page_id = page["id"].as_i64().unwrap();

        // Try to move root page - should fail
        let move_request = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "move_page",
                "arguments": {
                    "page_id": root_page_id,
                    "new_parent_id": page_id
                }
            }
        });

        let move_response = server.handle_request(move_request).await?;
        assert!(move_response.get("error").is_some());
        let error_msg = move_response["error"]["message"].as_str().unwrap();
        assert!(error_msg.contains("Cannot move root page"));

        Ok(())
    }

    #[tokio::test]
    async fn test_move_page_circular_reference_fails() -> Result<()> {
        let server = create_test_server().await?;

        // Get root page
        let list_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "list_pages",
                "arguments": {}
            }
        });

        let list_response = server.handle_request(list_request).await?;
        let pages_text = list_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let pages: Vec<serde_json::Value> = serde_json::from_str(pages_text)?;
        let root_page_id = pages[0]["page"]["id"].as_i64().unwrap();

        // Create parent page
        let create_parent = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "create_page",
                "arguments": {
                    "parent_page_id": root_page_id,
                    "slug": "parent",
                    "title": "Parent"
                }
            }
        });

        let parent_response = server.handle_request(create_parent).await?;
        let parent_text = parent_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let parent: serde_json::Value = serde_json::from_str(parent_text)?;
        let parent_id = parent["id"].as_i64().unwrap();

        // Create child page
        let create_child = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "create_page",
                "arguments": {
                    "parent_page_id": parent_id,
                    "slug": "child",
                    "title": "Child"
                }
            }
        });

        let child_response = server.handle_request(create_child).await?;
        let child_text = child_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let child: serde_json::Value = serde_json::from_str(child_text)?;
        let child_id = child["id"].as_i64().unwrap();

        // Try to move parent under child - should fail
        let move_request = json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "move_page",
                "arguments": {
                    "page_id": parent_id,
                    "new_parent_id": child_id
                }
            }
        });

        let move_response = server.handle_request(move_request).await?;
        assert!(move_response.get("error").is_some());
        let error_msg = move_response["error"]["message"].as_str().unwrap();
        assert!(error_msg.contains("Cannot move page to one of its descendants"));

        Ok(())
    }

    #[tokio::test]
    async fn test_create_component_markdown() -> Result<()> {
        let server = create_test_server().await?;

        // Get root page to create a test page
        let list_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "list_pages",
                "arguments": {}
            }
        });

        let list_response = server.handle_request(list_request).await?;
        let pages_text = list_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let pages: Vec<serde_json::Value> = serde_json::from_str(pages_text)?;
        let root_page_id = pages[0]["page"]["id"].as_i64().unwrap();

        // Create a page to add components to
        let create_page = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "create_page",
                "arguments": {
                    "parent_page_id": root_page_id,
                    "slug": "test-page",
                    "title": "Test Page"
                }
            }
        });

        let page_response = server.handle_request(create_page).await?;
        let page_text = page_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let page: serde_json::Value = serde_json::from_str(page_text)?;
        let page_id = page["id"].as_i64().unwrap();

        // Create a markdown component
        let create_component = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "create_component_markdown",
                "arguments": {
                    "page_id": page_id,
                    "text": "# Hello World\n\nThis is a **markdown** component.",
                    "title": "Welcome Section",
                    "template": "hero"
                }
            }
        });

        let component_response = server.handle_request(create_component).await?;
        let content = &component_response["result"]["content"][0];
        assert_eq!(content["type"], "text");

        let component_text = content["text"].as_str().unwrap();
        let component: serde_json::Value = serde_json::from_str(component_text)?;

        assert_eq!(component["component_type"], "markdown");
        assert_eq!(component["title"], "Welcome Section");
        assert_eq!(component["template"], "hero");
        assert_eq!(component["content"]["text"], "# Hello World\n\nThis is a **markdown** component.");

        Ok(())
    }

    #[tokio::test]
    async fn test_list_components() -> Result<()> {
        let server = create_test_server().await?;

        // Get root page
        let list_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "list_pages",
                "arguments": {}
            }
        });

        let list_response = server.handle_request(list_request).await?;
        let pages_text = list_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let pages: Vec<serde_json::Value> = serde_json::from_str(pages_text)?;
        let root_page_id = pages[0]["page"]["id"].as_i64().unwrap();

        // List components for root page (should be empty initially)
        let list_components = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "list_components",
                "arguments": {
                    "page_id": root_page_id
                }
            }
        });

        let components_response = server.handle_request(list_components).await?;
        let content = &components_response["result"]["content"][0];
        assert_eq!(content["type"], "text");

        let components_text = content["text"].as_str().unwrap();
        let components: Vec<serde_json::Value> = serde_json::from_str(components_text)?;
        assert_eq!(components.len(), 0); // No components initially

        Ok(())
    }

    #[tokio::test]
    async fn test_publish_draft() -> Result<()> {
        let server = create_test_server().await?;

        // First get the root page
        let list_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "list_pages",
                "arguments": {}
            }
        });
        let list_response = server.handle_request(list_request).await?;
        let pages_text = list_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let pages: Vec<serde_json::Value> = serde_json::from_str(pages_text)?;
        let root_page_id = pages[0]["page"]["id"].as_i64().unwrap();
        
        // Create a test page
        let create_page_req = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "create_page",
                "arguments": {
                    "parent_page_id": root_page_id,
                    "slug": "test-publish",
                    "title": "Test Publish Page"
                }
            }
        });
        
        let create_resp = server.handle_request(create_page_req).await?;
        let page_text = create_resp["result"]["content"][0]["text"].as_str().unwrap();
        let page_info: serde_json::Value = serde_json::from_str(page_text)?;
        let page_id = page_info["id"].as_i64().unwrap();

        // Create a component to generate a draft
        let create_request = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "create_component_markdown",
                "arguments": {
                    "page_id": page_id,
                    "text": "# Draft Content\n\nThis is draft content to be published.",
                    "title": "Draft Test"
                }
            }
        });

        let create_response = server.handle_request(create_request).await?;
        // Response contains the component info, not a success message
        let component_text = create_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let _component_info: serde_json::Value = serde_json::from_str(component_text)?;

        // Publish the draft
        let publish_request = json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "publish_draft",
                "arguments": {
                    "page_id": page_id
                }
            }
        });

        let publish_response = server.handle_request(publish_request).await?;
        let publish_text = publish_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        assert!(publish_text.contains("Successfully published draft"));
        assert!(publish_text.contains("is now live"));

        Ok(())
    }

    #[tokio::test]
    async fn test_discard_draft() -> Result<()> {
        let server = create_test_server().await?;

        // First get the root page
        let list_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "list_pages",
                "arguments": {}
            }
        });
        let list_response = server.handle_request(list_request).await?;
        let pages_text = list_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let pages: Vec<serde_json::Value> = serde_json::from_str(pages_text)?;
        let root_page_id = pages[0]["page"]["id"].as_i64().unwrap();
        
        // Create a test page
        let create_page_req = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "create_page",
                "arguments": {
                    "parent_page_id": root_page_id,
                    "slug": "test-discard",
                    "title": "Test Discard Page"
                }
            }
        });
        
        let create_resp = server.handle_request(create_page_req).await?;
        let page_text = create_resp["result"]["content"][0]["text"].as_str().unwrap();
        let page_info: serde_json::Value = serde_json::from_str(page_text)?;
        let page_id = page_info["id"].as_i64().unwrap();

        // Create a component to generate a draft
        let create_request = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "create_component_markdown",
                "arguments": {
                    "page_id": page_id,
                    "text": "This content should be discarded",
                    "title": "To Be Discarded"
                }
            }
        });

        server.handle_request(create_request).await?;

        // Discard the draft
        let discard_request = json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "discard_draft",
                "arguments": {
                    "page_id": page_id
                }
            }
        });

        let discard_response = server.handle_request(discard_request).await?;
        let discard_text = discard_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        assert!(discard_text.contains("Successfully discarded draft"));

        // Try to discard again - should fail
        let discard_again = server.handle_request(json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "discard_draft",
                "arguments": {
                    "page_id": page_id
                }
            }
        })).await?;

        assert!(discard_again.get("error").is_some());
        let error_msg = discard_again["error"]["message"].as_str().unwrap();
        assert!(error_msg.contains("No draft version found"));

        Ok(())
    }

    #[tokio::test]
    async fn test_update_component_markdown() -> Result<()> {
        let server = create_test_server().await?;

        // First get the root page
        let list_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "list_pages",
                "arguments": {}
            }
        });
        let list_response = server.handle_request(list_request).await?;
        let pages_text = list_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let pages: Vec<serde_json::Value> = serde_json::from_str(pages_text)?;
        let root_page_id = pages[0]["page"]["id"].as_i64().unwrap();
        
        // Create a test page
        let create_page_req = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "create_page",
                "arguments": {
                    "parent_page_id": root_page_id,
                    "slug": "test-update",
                    "title": "Test Update Page"
                }
            }
        });
        
        let create_resp = server.handle_request(create_page_req).await?;
        let page_text = create_resp["result"]["content"][0]["text"].as_str().unwrap();
        let page_info: serde_json::Value = serde_json::from_str(page_text)?;
        let page_id = page_info["id"].as_i64().unwrap();

        // Create a component first
        let create_request = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "create_component_markdown",
                "arguments": {
                    "page_id": page_id,
                    "text": "Original content",
                    "title": "Original Title"
                }
            }
        });

        let create_response = server.handle_request(create_request).await?;
        let create_text = create_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let component_info: serde_json::Value = serde_json::from_str(create_text)?;
        let component_id = component_info["id"].as_i64().unwrap();

        // Update the component
        let update_request = json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "update_component_markdown",
                "arguments": {
                    "component_id": component_id,
                    "text": "Updated content with **bold** text",
                    "title": "Updated Title",
                    "template": "card"
                }
            }
        });

        let update_response = server.handle_request(update_request).await?;
        assert!(update_response["result"]["content"][0]["text"].is_string());

        // Get the component to verify update
        let get_request = json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "get_component",
                "arguments": {
                    "component_id": component_id
                }
            }
        });

        let get_response = server.handle_request(get_request).await?;
        let component_text = get_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let updated_component: serde_json::Value = serde_json::from_str(component_text)?;
        
        assert_eq!(updated_component["title"], "Updated Title");
        assert_eq!(updated_component["template"], "card");
        assert_eq!(updated_component["content"]["text"], "Updated content with **bold** text");

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_component() -> Result<()> {
        let server = create_test_server().await?;

        // First get the root page
        let list_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "list_pages",
                "arguments": {}
            }
        });
        let list_response = server.handle_request(list_request).await?;
        let pages_text = list_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let pages: Vec<serde_json::Value> = serde_json::from_str(pages_text)?;
        let root_page_id = pages[0]["page"]["id"].as_i64().unwrap();
        
        // Create a test page
        let create_page_req = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "create_page",
                "arguments": {
                    "parent_page_id": root_page_id,
                    "slug": "test-delete",
                    "title": "Test Delete Page"
                }
            }
        });
        
        let create_resp = server.handle_request(create_page_req).await?;
        let page_text = create_resp["result"]["content"][0]["text"].as_str().unwrap();
        let page_info: serde_json::Value = serde_json::from_str(page_text)?;
        let page_id = page_info["id"].as_i64().unwrap();

        // Create a component
        let create_request = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": {
                "name": "create_component_markdown",
                "arguments": {
                    "page_id": page_id,
                    "text": "Component to be deleted"
                }
            }
        });

        let create_response = server.handle_request(create_request).await?;
        let create_text = create_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let component_info: serde_json::Value = serde_json::from_str(create_text)?;
        let component_id = component_info["id"].as_i64().unwrap();

        // Delete the component
        let delete_request = json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "delete_component",
                "arguments": {
                    "component_id": component_id
                }
            }
        });

        let delete_response = server.handle_request(delete_request).await?;
        let delete_text = delete_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        assert!(delete_text.contains(&format!("Successfully deleted component with ID {}", component_id)));

        // Try to get the deleted component - should fail
        let get_request = json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "get_component",
                "arguments": {
                    "component_id": component_id
                }
            }
        });

        let get_response = server.handle_request(get_request).await?;
        assert!(get_response.get("error").is_some());
        let error_msg = get_response["error"]["message"].as_str().unwrap();
        assert!(error_msg.contains("Component not found"));

        Ok(())
    }

    #[tokio::test]
    async fn test_resources_list() -> Result<()> {
        let server = create_test_server().await?;

        // First create some pages to have resources
        let list_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "list_pages",
                "arguments": {}
            }
        });

        let list_response = server.handle_request(list_request).await?;
        let pages_text = list_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let pages: Vec<serde_json::Value> = serde_json::from_str(pages_text)?;
        let root_page_id = pages[0]["page"]["id"].as_i64().unwrap();

        // Create a few test pages
        for i in 1..=3 {
            let create_request = json!({
                "jsonrpc": "2.0",
                "id": i + 1,
                "method": "tools/call",
                "params": {
                    "name": "create_page",
                    "arguments": {
                        "parent_page_id": root_page_id,
                        "slug": format!("page-{}", i),
                        "title": format!("Page {}", i),
                        "template": "default"
                    }
                }
            });
            server.handle_request(create_request).await?;
        }

        // Now test resources/list
        let resources_request = json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "resources/list"
        });

        let resources_response = server.handle_request(resources_request).await?;
        assert_eq!(resources_response["jsonrpc"], "2.0");
        assert_eq!(resources_response["id"], 10);
        
        let resources = resources_response["result"]["resources"].as_array().unwrap();
        assert!(resources.len() >= 4); // At least root + 3 created pages
        
        // Check first resource (should be root page)
        let root_resource = &resources[0];
        assert!(root_resource["uri"].as_str().unwrap().starts_with("page://"));
        assert!(root_resource["description"].as_str().unwrap().contains("Homepage"));
        assert_eq!(root_resource["mimeType"], "text/html");
        
        // Check that pages are in breadth-first order
        // Root should come first, then its children
        assert!(resources[0]["description"].as_str().unwrap().contains("Path: /"));
        assert!(resources[1]["description"].as_str().unwrap().contains("Path: /page-"));
        
        Ok(())
    }

    #[tokio::test]
    async fn test_resources_list_limit() -> Result<()> {
        let server = create_test_server().await?;

        // Get root page
        let list_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {
                "name": "list_pages",
                "arguments": {}
            }
        });

        let list_response = server.handle_request(list_request).await?;
        let pages_text = list_response["result"]["content"][0]["text"]
            .as_str()
            .unwrap();
        let pages: Vec<serde_json::Value> = serde_json::from_str(pages_text)?;
        let root_page_id = pages[0]["page"]["id"].as_i64().unwrap();

        // Create many pages to test the 100 page limit
        // Create 10 parent pages
        let mut parent_ids = vec![];
        for i in 1..=10 {
            let create_request = json!({
                "jsonrpc": "2.0",
                "id": i + 1,
                "method": "tools/call",
                "params": {
                    "name": "create_page",
                    "arguments": {
                        "parent_page_id": root_page_id,
                        "slug": format!("parent-{}", i),
                        "title": format!("Parent {}", i),
                        "template": "default"
                    }
                }
            });
            let response = server.handle_request(create_request).await?;
            let page_text = response["result"]["content"][0]["text"].as_str().unwrap();
            let page: serde_json::Value = serde_json::from_str(page_text)?;
            parent_ids.push(page["id"].as_i64().unwrap());
        }

        // For each parent, create 10 children (total 100 children + 10 parents + 1 root = 111 pages)
        for (p_idx, parent_id) in parent_ids.iter().enumerate() {
            for i in 1..=10 {
                let create_request = json!({
                    "jsonrpc": "2.0",
                    "id": 100 + p_idx * 10 + i,
                    "method": "tools/call",
                    "params": {
                        "name": "create_page",
                        "arguments": {
                            "parent_page_id": parent_id,
                            "slug": format!("child-{}-{}", p_idx + 1, i),
                            "title": format!("Child {} of Parent {}", i, p_idx + 1),
                            "template": "default"
                        }
                    }
                });
                server.handle_request(create_request).await?;
            }
        }

        // Test resources/list
        let resources_request = json!({
            "jsonrpc": "2.0",
            "id": 200,
            "method": "resources/list"
        });

        let resources_response = server.handle_request(resources_request).await?;
        let resources = resources_response["result"]["resources"].as_array().unwrap();
        
        // Should be limited to 100 pages
        assert_eq!(resources.len(), 100);
        
        // Verify breadth-first order:
        // 1. Root page should be first
        assert!(resources[0]["description"].as_str().unwrap().contains("Homepage"));
        
        // 2. All parent pages should come before any child pages
        let mut found_child = false;
        for (i, resource) in resources.iter().enumerate() {
            let desc = resource["description"].as_str().unwrap();
            if desc.contains("/child-") {
                found_child = true;
            }
            if found_child && desc.contains("/parent-") && !desc.contains("/child-") {
                panic!("Found parent page at index {} after child pages", i);
            }
        }
        
        Ok(())
    }

}
