use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::{header, HeaderMap},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    Json,
};
use doxyde_db::repositories::McpTokenRepository;
use serde_json::Value;
use std::convert::Infallible;
use std::time::Duration;
use tokio_stream::{self as stream};

use crate::{error::AppError, mcp_simple::SimpleMcpServer, state::AppState};

/// MCP HTTP endpoint handler that supports both regular JSON-RPC and SSE
pub async fn mcp_http_handler(
    State(state): State<AppState>,
    Path(token_id): Path<String>,
    headers: HeaderMap,
    Json(request): Json<Value>,
) -> Result<Response, AppError> {
    // Debug log the incoming request
    tracing::debug!(
        "MCP request received: {}",
        serde_json::to_string_pretty(&request).unwrap_or_default()
    );
    tracing::debug!("Request headers: {:?}", headers);

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

    // Check Accept header to determine response type
    let accept_header = headers
        .get(header::ACCEPT)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    if accept_header.contains("text/event-stream") {
        // Return SSE response
        let server = SimpleMcpServer::new(state.db.clone(), token.site_id);

        // Process the request
        let response_result = server.handle_request(request).await;

        // Create a stream that sends the response
        let stream = stream::once(match response_result {
            Ok(response) => {
                let event = Event::default()
                    .json_data(response)
                    .unwrap_or_else(|_| Event::default());
                Ok::<_, Infallible>(event)
            }
            Err(e) => {
                let error_response = serde_json::json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": -32603,
                        "message": format!("Internal error: {}", e)
                    }
                });
                let event = Event::default()
                    .json_data(error_response)
                    .unwrap_or_else(|_| Event::default());
                Ok(event)
            }
        });

        let sse = Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(30)));

        Ok(sse.into_response())
    } else {
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
