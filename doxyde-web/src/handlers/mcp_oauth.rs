use anyhow::Result;
use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    Json,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use futures::stream;
use serde_json::Value;
use std::convert::Infallible;
use std::time::Duration;

use crate::{
    error::AppError,
    mcp_simple::SimpleMcpServer,
    oauth2::{models::hash_token, BearerError},
    state::AppState,
};

/// OAuth2-protected MCP endpoint handler
pub async fn mcp_oauth_handler(
    State(state): State<AppState>,
    auth_header: Option<TypedHeader<Authorization<Bearer>>>,
    _headers: HeaderMap,
    Json(request): Json<Value>,
) -> Result<Response, AppError> {
    // Debug log the incoming request
    tracing::debug!(
        "OAuth MCP request received: {}",
        serde_json::to_string_pretty(&request).unwrap_or_default()
    );

    // Extract and validate Bearer token
    let bearer_token = match auth_header {
        Some(TypedHeader(auth)) => auth.token().to_string(),
        None => {
            return Ok(BearerError::invalid_token().into_response());
        }
    };

    // Hash the token to look it up
    let token_hash = hash_token(&bearer_token);

    // Look up access token
    let access_token_repo = doxyde_db::repositories::AccessTokenRepository::new(state.db.clone());
    let access_token = match access_token_repo.find_by_hash(&token_hash).await? {
        Some(token) => token,
        None => {
            return Ok(BearerError::invalid_token().into_response());
        }
    };

    // Check if access token is valid
    if !access_token.is_valid() {
        return Ok(BearerError::invalid_token().into_response());
    }

    // Get the MCP token associated with this access token
    let mcp_token_repo = doxyde_db::repositories::McpTokenRepository::new(state.db.clone());
    let mcp_token = mcp_token_repo
        .find_by_id(&access_token.mcp_token_id)
        .await?
        .ok_or(AppError::internal_server_error("MCP token not found"))?;

    // Check if MCP token is valid
    if !mcp_token.is_valid() {
        return Ok(BearerError::invalid_token().into_response());
    }

    // Update last used on MCP token
    let _ = mcp_token_repo.update_last_used(&mcp_token.id).await;

    // Get site_id from MCP token
    let site_id = mcp_token.site_id;

    // Return regular JSON response
    let server = SimpleMcpServer::new(state.db.clone(), site_id);

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

/// OAuth2 MCP SSE endpoint handler
pub async fn mcp_oauth_sse_handler(
    State(state): State<AppState>,
    auth_header: Option<TypedHeader<Authorization<Bearer>>>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    // Debug log
    tracing::debug!("OAuth MCP SSE connection request");

    // Extract and validate Bearer token
    let bearer_token = match auth_header {
        Some(TypedHeader(auth)) => auth.token().to_string(),
        None => {
            return Ok(BearerError::invalid_token().into_response());
        }
    };

    // Hash the token to look it up
    let token_hash = hash_token(&bearer_token);

    // Look up access token
    let access_token_repo = doxyde_db::repositories::AccessTokenRepository::new(state.db.clone());
    let access_token = match access_token_repo.find_by_hash(&token_hash).await? {
        Some(token) => token,
        None => {
            return Ok(BearerError::invalid_token().into_response());
        }
    };

    // Check if access token is valid
    if !access_token.is_valid() {
        return Ok(BearerError::invalid_token().into_response());
    }

    // Get the MCP token associated with this access token
    let mcp_token_repo = doxyde_db::repositories::McpTokenRepository::new(state.db.clone());
    let mcp_token = mcp_token_repo
        .find_by_id(&access_token.mcp_token_id)
        .await?
        .ok_or(AppError::internal_server_error("MCP token not found"))?;

    // Check if MCP token is valid
    if !mcp_token.is_valid() {
        return Ok(BearerError::invalid_token().into_response());
    }

    // Update last used on MCP token
    let _ = mcp_token_repo.update_last_used(&mcp_token.id).await;

    // Get the host header to construct the endpoint URL
    let host = headers
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("localhost");

    // Determine protocol based on host
    let protocol =
        if host.starts_with("localhost") || host.contains(":3000") || host.contains(":8000") {
            "http"
        } else {
            "https"
        };

    // Create SSE stream
    // The endpoint event should send the URI as a plain string, not JSON
    let endpoint_uri = format!("{}://{}/.mcp", protocol, host);

    let event = Event::default()
        .event("endpoint")
        .data(endpoint_uri);

    let stream = stream::once(async move { Ok::<_, Infallible>(event) });

    let sse = Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(30)));

    Ok(sse.into_response())
}

/// Legacy MCP endpoint that expects MCP token in path (backward compatibility)
pub async fn mcp_legacy_handler(
    State(state): State<AppState>,
    Path(token_id): Path<String>,
    headers: HeaderMap,
    Json(request): Json<Value>,
) -> Result<Response, AppError> {
    // Check for Authorization header - if present, redirect to OAuth endpoint
    if headers.get(header::AUTHORIZATION).is_some() {
        return Ok((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "jsonrpc": "2.0",
                "error": {
                    "code": -32600,
                    "message": "This endpoint does not support OAuth2. Use /.mcp for OAuth2-protected access."
                }
            })),
        )
            .into_response());
    }

    // Debug log the incoming request
    tracing::debug!(
        "Legacy MCP request received: {}",
        serde_json::to_string_pretty(&request).unwrap_or_default()
    );

    // Validate token
    let token_repo = doxyde_db::repositories::McpTokenRepository::new(state.db.clone());
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
