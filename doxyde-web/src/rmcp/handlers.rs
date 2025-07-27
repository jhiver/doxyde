// Doxyde - A modern, AI-native CMS built with Rust
// Copyright (C) 2025 Doxyde Project Contributors
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use axum::{
    extract::{State, Request},
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    response::Response,
    Json,
};
use rmcp::{
    transport::sse_server::{SseServer, SseServerConfig},
    Service,
};
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};

use super::{oauth::validate_token, service::DoxydeRmcpService};
use crate::AppState;

fn extract_bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
}

// SSE handler that validates OAuth and then delegates to RMCP's SSE handler
pub async fn handle_sse(
    State(state): State<AppState>,
    headers: HeaderMap,
    _req: Request,
) -> Result<Response, StatusCode> {
    // Validate OAuth token
    if let Some(token) = extract_bearer_token(&headers) {
        match validate_token(&state.db, token).await {
            Ok(Some(_token_info)) => {
                info!("Valid OAuth token for SSE connection");
            }
            Ok(None) => {
                error!("Invalid OAuth token");
                return Err(StatusCode::UNAUTHORIZED);
            }
            Err(e) => {
                error!("Token validation error: {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    } else {
        error!("Missing Authorization header");
        return Err(StatusCode::UNAUTHORIZED);
    }

    // Create SSE server config
    let config = SseServerConfig {
        bind: "127.0.0.1:0".parse::<SocketAddr>().unwrap(), // Not used in our case
        sse_path: "/.mcp/sse".to_string(),
        post_path: "/.mcp/sse/message".to_string(),
        ct: CancellationToken::new(),
        sse_keep_alive: Some(Duration::from_secs(30)),
    };

    // Create SSE server and router
    let (sse_server, _router) = SseServer::new(config);
    
    // Spawn a task to handle the service
    let _service_handle = sse_server.with_service(|| DoxydeRmcpService::new());
    
    // For now, return not implemented until we properly integrate the SSE handler
    // The proper implementation would need to extract the SSE handler from the router
    // and use it to handle this request
    error!("SSE endpoint reached but full integration not yet implemented");
    Err(StatusCode::NOT_IMPLEMENTED)
}

// HTTP handler for MCP
pub async fn handle_http(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    // Extract request ID early (before authentication)
    let request_id = body.get("id").cloned();
    
    // Helper function to create error response with proper ID handling
    let create_error_response = |code: i32, message: &str| -> Json<Value> {
        let mut response = json!({
            "jsonrpc": "2.0",
            "error": {
                "code": code,
                "message": message
            }
        });
        
        // Only include id if it exists and is not null
        if let Some(id) = &request_id {
            if !id.is_null() {
                response["id"] = id.clone();
            }
        }
        
        Json(response)
    };
    
    // Validate OAuth token
    if let Some(token) = extract_bearer_token(&headers) {
        match validate_token(&state.db, token).await {
            Ok(Some(_token_info)) => {
                debug!("Valid OAuth token for HTTP request");
            }
            Ok(None) => {
                return Ok(create_error_response(-32603, "Invalid token"));
            }
            Err(e) => {
                error!("Token validation error: {}", e);
                return Ok(create_error_response(-32603, "Token validation failed"));
            }
        }
    } else {
        return Ok(create_error_response(-32603, "Authorization required"));
    }

    // Extract method from request
    let method = body.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let id = request_id.unwrap_or(Value::Null);

    debug!("MCP HTTP request: method={}", method);

    // Create the service
    let service = DoxydeRmcpService::new();

    // Handle different methods
    match method {
        "initialize" => {
            let result = service.get_info();
            Ok(Json(json!({
                "jsonrpc": "2.0",
                "result": {
                    "protocolVersion": result.protocol_version,
                    "capabilities": result.capabilities,
                    "serverInfo": result.server_info,
                    "instructions": result.instructions
                },
                "id": id
            })))
        }
        "tools/list" => {
            Ok(Json(json!({
                "jsonrpc": "2.0",
                "result": {
                    "tools": [{
                        "name": "time",
                        "description": "Get the current time in a specified timezone",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "timezone": {
                                    "type": "string",
                                    "description": "Timezone name (e.g., 'America/New_York', 'UTC')"
                                }
                            }
                        }
                    }]
                },
                "id": id
            })))
        }
        "tools/call" => {
            let params = body.get("params").cloned().unwrap_or(json!({}));
            let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let arguments = params.get("arguments").cloned();

            if tool_name == "time" {
                let timezone = arguments
                    .as_ref()
                    .and_then(|args| args.get("timezone"))
                    .and_then(|tz| tz.as_str())
                    .map(String::from);

                let result = service.time(rmcp::handler::server::tool::Parameters(
                    super::service::TimeRequest { timezone }
                ));

                // Parse the JSON string result
                match serde_json::from_str::<Value>(&result) {
                    Ok(_content) => Ok(Json(json!({
                        "jsonrpc": "2.0",
                        "result": {
                            "content": [{"type": "text", "text": result}],
                            "isError": false
                        },
                        "id": id
                    }))),
                    Err(_) => Ok(Json(json!({
                        "jsonrpc": "2.0",
                        "result": {
                            "content": [{"type": "text", "text": result}],
                            "isError": result.starts_with("Error:")
                        },
                        "id": id
                    })))
                }
            } else {
                Ok(Json(json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": -32601,
                        "message": format!("Unknown tool: {}", tool_name)
                    },
                    "id": id
                })))
            }
        }
        _ => {
            Ok(Json(json!({
                "jsonrpc": "2.0",
                "error": {
                    "code": -32601,
                    "message": format!("Method not found: {}", method)
                },
                "id": id
            })))
        }
    }
}