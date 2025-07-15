use jsonrpc_core::{Error as JsonRpcError, ErrorCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", content = "params")]
pub enum McpRequest {
    #[serde(rename = "initialize")]
    Initialize(InitializeParams),

    #[serde(rename = "tools/list")]
    ListTools,

    #[serde(rename = "tools/call")]
    CallTool(CallToolParams),

    #[serde(rename = "resources/list")]
    ListResources,

    #[serde(rename = "resources/read")]
    ReadResource(ReadResourceParams),

    #[serde(rename = "logging/setLevel")]
    SetLoggingLevel(SetLoggingLevelParams),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeParams {
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
    pub client_info: ClientInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub roots: Option<RootsCapability>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling: Option<HashMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootsCapability {
    pub list_changed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolParams {
    pub name: String,
    #[serde(default)]
    pub arguments: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadResourceParams {
    pub uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetLoggingLevelParams {
    pub level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitializeResponse {
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    pub server_info: ServerInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<HashMap<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resources: Option<HashMap<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<HashMap<String, Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logging: Option<HashMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListToolsResponse {
    pub tools: Vec<Tool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallToolResponse {
    pub content: Vec<ToolContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ToolContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResourcesResponse {
    pub resources: Vec<Resource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadResourceResponse {
    pub contents: Vec<ResourceContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ResourceContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "blob")]
    Blob { data: String },
}

pub fn create_error_response(code: i64, message: String, data: Option<Value>) -> JsonRpcError {
    JsonRpcError {
        code: ErrorCode::ServerError(code),
        message,
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_initialize_params_serialization() {
        let params = InitializeParams {
            protocol_version: "1.0".to_string(),
            capabilities: ClientCapabilities {
                roots: None,
                sampling: None,
            },
            client_info: ClientInfo {
                name: "test-client".to_string(),
                version: "0.1.0".to_string(),
            },
        };

        let json = serde_json::to_value(&params).unwrap();
        assert_eq!(json["protocol_version"], "1.0");
        assert_eq!(json["client_info"]["name"], "test-client");
    }

    #[test]
    fn test_tool_serialization() {
        let tool = Tool {
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
        };

        let json = serde_json::to_value(&tool).unwrap();
        assert_eq!(json["name"], "list_pages");
        assert_eq!(json["input_schema"]["type"], "object");
    }

    #[test]
    fn test_call_tool_params_deserialization() {
        let json = json!({
            "name": "create_page",
            "arguments": {
                "title": "Test Page",
                "slug": "test-page"
            }
        });

        let params: CallToolParams = serde_json::from_value(json).unwrap();
        assert_eq!(params.name, "create_page");
        assert_eq!(params.arguments["title"], "Test Page");
    }
}
