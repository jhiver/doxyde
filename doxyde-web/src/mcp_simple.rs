use crate::services::McpService;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::SqlitePool;

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
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
        // Parse the request
        let method = request.get("method").and_then(|v| v.as_str()).unwrap_or("");
        let id = request.get("id").cloned().unwrap_or(json!(null));
        let params = request.get("params").cloned();

        let response = match method {
            "initialize" => {
                json!({
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
                })
            }
            "tools/list" => {
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "tools": [
                            // Demo tools
                            {
                                "name": "flip_coin",
                                "description": "Flip a coin one or more times",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "times": {
                                            "type": "integer",
                                            "description": "Number of times to flip the coin (default: 1, max: 10)",
                                            "minimum": 1,
                                            "maximum": 10,
                                            "default": 1
                                        }
                                    }
                                }
                            },
                            {
                                "name": "get_current_time",
                                "description": "Get the current time in UTC or a specified timezone",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {
                                        "timezone": {
                                            "type": "string",
                                            "description": "Timezone (e.g., 'America/New_York', 'Europe/London'). Defaults to UTC.",
                                            "examples": ["UTC", "America/New_York", "Europe/London", "Asia/Tokyo"]
                                        }
                                    }
                                }
                            },
                            // Phase 1: Read-only operations
                            {
                                "name": "list_pages",
                                "description": "Get all pages in the site with hierarchy",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {}
                                }
                            },
                            {
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
                            },
                            {
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
                            },
                            {
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
                            },
                            {
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
                            },
                            {
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
                            }
                        ]
                    }
                })
            }
            "tools/call" => {
                let tool_name = params
                    .as_ref()
                    .and_then(|p| p.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("");

                let args = params
                    .as_ref()
                    .and_then(|p| p.get("arguments"))
                    .cloned()
                    .unwrap_or(json!({}));

                let content = self.call_tool(tool_name, args).await?;

                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "content": content
                    }
                })
            }
            "notifications/initialized" => {
                // Client notification - no response needed but we'll acknowledge it
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {}
                })
            }
            _ => {
                json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32601,
                        "message": format!("Method not found: {}", method)
                    }
                })
            }
        };

        Ok(response)
    }

    async fn call_tool(&self, name: &str, arguments: Value) -> Result<Vec<Value>> {
        match name {
            "flip_coin" => {
                let times = arguments
                    .get("times")
                    .and_then(|t| t.as_u64())
                    .unwrap_or(1)
                    .min(10)
                    .max(1) as usize;

                let mut results = Vec::new();
                for i in 1..=times {
                    let result = if rand::random::<bool>() {
                        "Heads"
                    } else {
                        "Tails"
                    };
                    results.push(format!("Flip {}: {}", i, result));
                }

                Ok(vec![json!({
                    "type": "text",
                    "text": results.join("\n")
                })])
            }
            "get_current_time" => {
                let now = chrono::Utc::now();

                let formatted =
                    if let Some(tz_str) = arguments.get("timezone").and_then(|t| t.as_str()) {
                        // Try to parse timezone
                        match tz_str.parse::<chrono_tz::Tz>() {
                            Ok(tz) => {
                                let local_time = now.with_timezone(&tz);
                                format!(
                                    "Current time in {}: {}",
                                    tz_str,
                                    local_time.format("%Y-%m-%d %H:%M:%S %Z")
                                )
                            }
                            Err(_) => {
                                format!(
                                    "Invalid timezone '{}'. Using UTC: {}",
                                    tz_str,
                                    now.format("%Y-%m-%d %H:%M:%S UTC")
                                )
                            }
                        }
                    } else {
                        format!("Current time (UTC): {}", now.format("%Y-%m-%d %H:%M:%S"))
                    };

                Ok(vec![json!({
                    "type": "text",
                    "text": formatted
                })])
            }
            // Phase 1: Read-only operations
            "list_pages" => {
                let service = McpService::new(self.pool.clone(), self.site_id);
                match service.list_pages().await {
                    Ok(pages) => Ok(vec![json!({
                        "type": "text",
                        "text": serde_json::to_string_pretty(&pages)?
                    })]),
                    Err(e) => Ok(vec![json!({
                        "type": "text",
                        "text": format!("Error listing pages: {}", e)
                    })]),
                }
            }
            "get_page" => {
                let page_id = arguments
                    .get("page_id")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| anyhow::anyhow!("page_id is required"))?;

                let service = McpService::new(self.pool.clone(), self.site_id);
                match service.get_page(page_id).await {
                    Ok(page) => Ok(vec![json!({
                        "type": "text",
                        "text": serde_json::to_string_pretty(&page)?
                    })]),
                    Err(e) => Ok(vec![json!({
                        "type": "text",
                        "text": format!("Error getting page: {}", e)
                    })]),
                }
            }
            "get_page_by_path" => {
                let path = arguments
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("path is required"))?;

                let service = McpService::new(self.pool.clone(), self.site_id);
                match service.get_page_by_path(path).await {
                    Ok(page) => Ok(vec![json!({
                        "type": "text",
                        "text": serde_json::to_string_pretty(&page)?
                    })]),
                    Err(e) => Ok(vec![json!({
                        "type": "text",
                        "text": format!("Error finding page: {}", e)
                    })]),
                }
            }
            "get_published_content" => {
                let page_id = arguments
                    .get("page_id")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| anyhow::anyhow!("page_id is required"))?;

                let service = McpService::new(self.pool.clone(), self.site_id);
                match service.get_published_content(page_id).await {
                    Ok(components) => Ok(vec![json!({
                        "type": "text",
                        "text": serde_json::to_string_pretty(&components)?
                    })]),
                    Err(e) => Ok(vec![json!({
                        "type": "text",
                        "text": format!("Error getting published content: {}", e)
                    })]),
                }
            }
            "get_draft_content" => {
                let page_id = arguments
                    .get("page_id")
                    .and_then(|v| v.as_i64())
                    .ok_or_else(|| anyhow::anyhow!("page_id is required"))?;

                let service = McpService::new(self.pool.clone(), self.site_id);
                match service.get_draft_content(page_id).await {
                    Ok(Some(components)) => Ok(vec![json!({
                        "type": "text",
                        "text": serde_json::to_string_pretty(&components)?
                    })]),
                    Ok(None) => Ok(vec![json!({
                        "type": "text",
                        "text": "No draft version exists for this page"
                    })]),
                    Err(e) => Ok(vec![json!({
                        "type": "text",
                        "text": format!("Error getting draft content: {}", e)
                    })]),
                }
            }
            "search_pages" => {
                let query = arguments
                    .get("query")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("query is required"))?;

                let service = McpService::new(self.pool.clone(), self.site_id);
                match service.search_pages(query).await {
                    Ok(results) => Ok(vec![json!({
                        "type": "text",
                        "text": serde_json::to_string_pretty(&results)?
                    })]),
                    Err(e) => Ok(vec![json!({
                        "type": "text",
                        "text": format!("Error searching pages: {}", e)
                    })]),
                }
            }
            _ => Ok(vec![json!({
                "type": "text",
                "text": format!("Unknown tool: {}", name)
            })]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert_eq!(tools.len(), 8); // 2 demo + 6 Phase 1 tools

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
}
