use anyhow::Result;
use axum::{
    extract::{Query, State},
    http::{header, HeaderMap},
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
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use uuid::Uuid;
use futures::stream::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::{
    error::AppError,
    mcp_simple::SimpleMcpServer,
    oauth2::{models::hash_token, BearerError},
    state::AppState,
};

// Store active SSE sessions
lazy_static::lazy_static! {
    static ref SSE_SESSIONS: Arc<RwLock<HashMap<String, SseSession>>> = Arc::new(RwLock::new(HashMap::new()));
}

#[derive(Clone)]
struct SseSession {
    site_id: i64,
    #[allow(dead_code)]
    bearer_token: String,
}

#[derive(Deserialize)]
pub struct SseQuery {
    session_id: Option<String>,
}

// Custom stream that sends the endpoint event and then keeps the connection alive
struct SseEndpointStream {
    sent_endpoint: bool,
    endpoint_uri: String,
    session_id: String,
}

impl Stream for SseEndpointStream {
    type Item = Result<Event, std::convert::Infallible>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if !self.sent_endpoint {
            self.sent_endpoint = true;
            let event = Event::default()
                .event("endpoint")
                .data(self.endpoint_uri.clone());
            tracing::info!("Sending SSE endpoint event: {}", self.endpoint_uri);
            Poll::Ready(Some(Ok(event)))
        } else {
            // Keep the stream alive by returning Pending
            // The KeepAlive will handle sending heartbeats
            Poll::Pending
        }
    }
}

/// SSE endpoint handler - establishes SSE connection and sends endpoint event
pub async fn mcp_sse_handler(
    State(state): State<AppState>,
    auth_header: Option<TypedHeader<Authorization<Bearer>>>,
    headers: HeaderMap,
) -> Result<Response, AppError> {
    tracing::info!("SSE connection request");

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

    // Generate a session ID for this SSE connection
    let session_id = Uuid::new_v4().to_string();

    // Store the session
    {
        let mut sessions = SSE_SESSIONS.write().await;
        sessions.insert(
            session_id.clone(),
            SseSession {
                site_id,
                bearer_token: bearer_token.clone(),
            },
        );
    }

    // Create the endpoint URL for SSE clients to POST to
    // MCP spec requires a relative URI, not absolute
    let endpoint_uri = format!("/.sse/messages?session_id={}", session_id);

    tracing::info!("Creating SSE stream with endpoint: {}", endpoint_uri);

    // Create our custom stream
    let stream = SseEndpointStream {
        sent_endpoint: false,
        endpoint_uri: endpoint_uri.clone(),
        session_id: session_id.clone(),
    };

    // Create SSE response with keep-alive
    let sse = Sse::new(stream)
        .keep_alive(KeepAlive::new().interval(Duration::from_secs(30)));

    // Set up cleanup when connection closes
    let session_id_for_cleanup = session_id.clone();
    tokio::spawn(async move {
        // Wait a bit to ensure the connection is established
        tokio::time::sleep(Duration::from_secs(5)).await;
        
        // This is a simplified cleanup - in production, you'd want to
        // detect when the HTTP connection actually closes
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            // In a real implementation, check if the connection is still active
            // For now, sessions will be cleaned up when they're used or on server restart
        }
    });

    tracing::info!("SSE handler returning response for session {}", session_id);
    Ok(sse.into_response())
}

/// SSE messages endpoint - handles MCP requests from SSE clients
pub async fn mcp_sse_messages_handler(
    State(state): State<AppState>,
    Query(query): Query<SseQuery>,
    Json(request): Json<Value>,
) -> Result<Response, AppError> {
    let session_id = query.session_id
        .ok_or_else(|| AppError::bad_request("Missing session_id parameter"))?;

    tracing::info!("SSE message request for session: {}", session_id);

    // Look up the session
    let session = {
        let sessions = SSE_SESSIONS.read().await;
        sessions.get(&session_id).cloned()
    };

    let session = session
        .ok_or_else(|| AppError::bad_request("Invalid or expired session"))?;

    // Log the incoming request
    tracing::info!(
        "SSE MCP request received: {}",
        serde_json::to_string_pretty(&request).unwrap_or_default()
    );

    // Handle the MCP request
    let server = SimpleMcpServer::new(state.db.clone(), session.site_id);

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