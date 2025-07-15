use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use doxyde_db::repositories::McpTokenRepository;
use serde_json::{json, Value};

use crate::{error::AppError, state::AppState};

/// MCP server endpoint handler
pub async fn mcp_server_handler(
    State(state): State<AppState>,
    Path(token_id): Path<String>,
    _headers: HeaderMap,
    Json(request): Json<Value>,
) -> Result<impl IntoResponse, AppError> {
    // Validate token
    let token_repo = McpTokenRepository::new(state.db.clone());
    let token = token_repo
        .find_by_id(&token_id)
        .await?
        .ok_or(AppError::not_found("Token not found"))?;

    // Check if token is valid
    if !token.is_valid() {
        return Err(AppError::forbidden("Token has been revoked"));
    }

    // Update last used
    let _ = token_repo.update_last_used(&token_id).await;

    // For now, return a simple response that shows the MCP server is working
    let method = request.get("method").and_then(|v| v.as_str()).unwrap_or("");

    let response = match method {
        "initialize" => {
            json!({
                "jsonrpc": "2.0",
                "id": request.get("id").cloned().unwrap_or(json!(null)),
                "result": {
                    "protocolVersion": "2025-07-01",
                    "capabilities": {
                        "tools": {},
                        "resources": {},
                        "logging": {}
                    },
                    "serverInfo": {
                        "name": "doxyde-mcp",
                        "version": "0.1.0"
                    }
                }
            })
        }
        "tools/list" => {
            json!({
                "jsonrpc": "2.0",
                "id": request.get("id").cloned().unwrap_or(json!(null)),
                "result": {
                    "tools": [
                        {
                            "name": "list_sites",
                            "description": "List all sites the user has access to",
                            "inputSchema": {
                                "type": "object",
                                "properties": {}
                            }
                        },
                        {
                            "name": "get_page",
                            "description": "Get details about a specific page",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "site_id": {
                                        "type": "integer",
                                        "description": "Site ID"
                                    },
                                    "page_id": {
                                        "type": "integer",
                                        "description": "Page ID"
                                    }
                                },
                                "required": ["site_id", "page_id"]
                            }
                        }
                    ]
                }
            })
        }
        "tools/call" => {
            // For now, return mock responses
            let tool_name = request
                .get("params")
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or("");

            let result = match tool_name {
                "list_sites" => {
                    vec![json!({
                        "type": "text",
                        "text": format!("User has access to site ID {} (token restricted)", token.site_id)
                    })]
                }
                _ => {
                    vec![json!({
                        "type": "text",
                        "text": format!("Tool '{}' called (mock response)", tool_name)
                    })]
                }
            };

            json!({
                "jsonrpc": "2.0",
                "id": request.get("id").cloned().unwrap_or(json!(null)),
                "result": {
                    "content": result
                }
            })
        }
        "notifications/initialized" => {
            // Client is ready after initialization handshake
            // This is a notification, so no response is expected
            json!({
                "jsonrpc": "2.0",
                "id": request.get("id").cloned().unwrap_or(json!(null)),
                "result": {}
            })
        }
        "notifications/cancelled" => {
            // Request cancellation notification
            json!({
                "jsonrpc": "2.0",
                "id": request.get("id").cloned().unwrap_or(json!(null)),
                "result": {}
            })
        }
        "completion/complete" => {
            // Autocomplete support (optional)
            json!({
                "jsonrpc": "2.0",
                "id": request.get("id").cloned().unwrap_or(json!(null)),
                "result": {
                    "completion": {
                        "values": []
                    }
                }
            })
        }
        _ => {
            json!({
                "jsonrpc": "2.0",
                "id": request.get("id").cloned().unwrap_or(json!(null)),
                "error": {
                    "code": -32601,
                    "message": "Method not found"
                }
            })
        }
    };

    // Return JSON response with session header
    let mut headers = HeaderMap::new();
    headers.insert("content-type", "application/json".parse().unwrap());
    headers.insert("mcp-session-id", token.id.parse().unwrap());

    Ok((StatusCode::OK, headers, Json(response)).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{create_test_app_state, create_test_site, create_test_user};
    use axum::body::Body;
    use axum::http::Request;
    use doxyde_core::models::McpToken;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_mcp_server_invalid_token() -> Result<()> {
        let state = create_test_app_state().await?;
        let app = crate::routes::create_router(state);

        let request = Request::builder()
            .method("POST")
            .uri("/.mcp/invalid-token-id")
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "jsonrpc": "2.0",
                    "method": "initialize",
                    "params": {},
                    "id": 1
                })
                .to_string(),
            ))?;

        let response = app.oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        Ok(())
    }

    #[tokio::test]
    async fn test_mcp_server_revoked_token() -> Result<()> {
        let state = create_test_app_state().await?;
        let pool = state.db.clone();
        let user = create_test_user(&pool, "testuser", "test@example.com", false).await?;
        let site = create_test_site(&pool, "example.com", "Example Site").await?;

        // Create and revoke a token
        let token = McpToken::new(user.id.unwrap(), site.id.unwrap(), "Test Token".to_string());
        let token_repo = McpTokenRepository::new(pool);
        token_repo.create(&token).await?;
        token_repo.revoke(&token.id).await?;

        let app = crate::routes::create_router(state);

        let request = Request::builder()
            .method("POST")
            .uri(&format!("/.mcp/{}", token.id))
            .header("content-type", "application/json")
            .body(Body::from(
                json!({
                    "jsonrpc": "2.0",
                    "method": "initialize",
                    "params": {},
                    "id": 1
                })
                .to_string(),
            ))?;

        let response = app.oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

        Ok(())
    }
}
