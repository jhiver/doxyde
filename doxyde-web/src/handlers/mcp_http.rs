use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
    Json,
};
use doxyde_db::repositories::McpTokenRepository;
use serde_json::Value;

use crate::{error::AppError, mcp_simple::SimpleMcpServer, state::AppState};

/// MCP HTTP endpoint handler for JSON-RPC
pub async fn mcp_http_handler(
    State(state): State<AppState>,
    Path(token_id): Path<String>,
    _headers: HeaderMap,
    Json(request): Json<Value>,
) -> Result<Response, AppError> {
    // Debug log the incoming request
    tracing::debug!(
        "MCP request received: {}",
        serde_json::to_string_pretty(&request).unwrap_or_default()
    );
    tracing::debug!("Request headers: {:?}", _headers);

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

    // Return regular JSON response
    let server = SimpleMcpServer::new(state.db.clone(), token.site_id);

    let response = match server.handle_request(request.clone()).await {
        Ok(response) => response,
        Err(e) => {
            // Extract the request ID if possible
            let id = request
                .get("id")
                .cloned()
                .unwrap_or(serde_json::Value::Null);

            serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -32603,
                    "message": format!("Internal error: {}", e)
                }
            })
        }
    };

    Ok(Json(response).into_response())
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
    async fn test_mcp_http_json_response() -> Result<()> {
        let state = create_test_app_state().await?;
        let pool = state.db.clone();
        let user = create_test_user(&pool, "testuser", "test@example.com", false).await?;
        let site = create_test_site(&pool, "example.com", "Example Site").await?;

        // Create a valid token
        let token = McpToken::new(user.id.unwrap(), site.id.unwrap(), "Test Token".to_string());
        let token_repo = McpTokenRepository::new(pool);
        token_repo.create(&token).await?;

        let app = crate::routes::create_router(state);

        let request = Request::builder()
            .method("POST")
            .uri(&format!("/.mcp/{}", token.id))
            .header("content-type", "application/json")
            .header("accept", "application/json")
            .header("host", "example.com")
            .body(Body::from(
                serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "initialize",
                    "params": {}
                })
                .to_string(),
            ))?;

        let response = app.oneshot(request).await?;
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        Ok(())
    }
}
