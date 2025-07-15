use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use doxyde_db::repositories::McpTokenRepository;
use serde_json::Value;

use crate::{error::AppError, mcp_simple::SimpleMcpServer, state::AppState};

/// Simplified MCP server endpoint handler
pub async fn mcp_server_handler_simple(
    State(state): State<AppState>,
    Path(token_id): Path<String>,
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

    // Create MCP server and handle request
    let server = SimpleMcpServer::new(state.db.clone(), token.site_id);
    let response = server
        .handle_request(request)
        .await
        .map_err(|e| AppError::internal_server_error(&format!("MCP processing error: {}", e)))?;

    Ok((StatusCode::OK, Json(response)).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{create_test_app_state, create_test_site, create_test_user};
    use doxyde_core::models::McpToken;

    #[tokio::test]
    async fn test_mcp_server_simple() -> Result<()> {
        let state = create_test_app_state().await?;
        let pool = state.db.clone();
        let user = create_test_user(&pool, "testuser", "test@example.com", false).await?;
        let site = create_test_site(&pool, "example.com", "Example Site").await?;

        // Create a valid token
        let token = McpToken::new(user.id.unwrap(), site.id.unwrap(), "Test Token".to_string());
        let token_repo = McpTokenRepository::new(pool);
        token_repo.create(&token).await?;

        // Would need to update routes to test this
        // let app = crate::routes::create_router(state);

        Ok(())
    }
}
