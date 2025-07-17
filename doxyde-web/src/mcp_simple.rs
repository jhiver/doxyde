use crate::services::McpService;
use anyhow::Result;
use serde_json::{json, Value};
use sqlx::SqlitePool;

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
                        "type": "string",
                        "description": "URL-friendly page identifier (letters, numbers, hyphens only)"
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
                "required": ["slug", "title"]
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
    slug: String,
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

fn extract_create_page_params(arguments: &Value) -> Result<CreatePageParams> {
    let parent_page_id = arguments.get("parent_page_id").and_then(|v| v.as_i64());

    let slug = arguments
        .get("slug")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("slug is required"))?
        .to_string();

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
        assert_eq!(params.slug, "test-page");
        assert_eq!(params.title, "Test Page");
        assert_eq!(params.template, Some("default".to_string()));

        let args = json!({
            "slug": "test-page",
            "title": "Test Page"
        });
        let params = extract_create_page_params(&args).unwrap();
        assert_eq!(params.parent_page_id, None);
        assert_eq!(params.template, None);

        let args = json!({"title": "Test Page"});
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
        assert_eq!(tools.len(), 12); // 2 demo + 6 Phase 1 tools + 4 write tools

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
}
