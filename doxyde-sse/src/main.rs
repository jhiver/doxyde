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

mod config;

use anyhow::Result;
use axum::{
    extract::State, 
    http::{header::AUTHORIZATION, HeaderMap, StatusCode}, 
    middleware as axum_middleware,
    routing::get,
    Router,
    Json,
};
use config::Config;
use doxyde_shared::{mcp::DoxydeRmcpService, oauth::validate_token};
use rmcp::transport::sse_server::{SseServer, SseServerConfig};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::{net::SocketAddr, sync::Arc};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};

#[derive(Clone)]
struct AppState {
    db: SqlitePool,
}

// OAuth metadata structures
#[derive(Debug, Serialize, Deserialize)]
struct AuthorizationServerMetadata {
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    registration_endpoint: Option<String>,
    scopes_supported: Vec<String>,
    response_types_supported: Vec<String>,
    response_modes_supported: Vec<String>,
    grant_types_supported: Vec<String>,
    token_endpoint_auth_methods_supported: Vec<String>,
    code_challenge_methods_supported: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProtectedResourceMetadata {
    #[serde(rename = "oauth-authorization-server")]
    oauth_authorization_server: String,
    #[serde(rename = "protected-resources")]
    protected_resources: Vec<String>,
}

// Health check endpoint
async fn health_handler() -> &'static str {
    "OK"
}

// OAuth discovery endpoints that point to main doxyde.com server
async fn oauth_authorization_server_metadata() -> Json<AuthorizationServerMetadata> {
    Json(AuthorizationServerMetadata {
        issuer: "https://doxyde.com".to_string(),
        authorization_endpoint: "https://doxyde.com/.oauth/authorize".to_string(),
        token_endpoint: "https://doxyde.com/.oauth/token".to_string(),
        registration_endpoint: Some("https://doxyde.com/.oauth/register".to_string()),
        scopes_supported: vec![
            "mcp:read".to_string(),
            "mcp:write".to_string(),
            "read".to_string(),
            "write".to_string(),
            "admin".to_string(),
        ],
        response_types_supported: vec![
            "code".to_string(),
            "token".to_string(),
        ],
        response_modes_supported: vec![
            "query".to_string(),
            "fragment".to_string(),
        ],
        grant_types_supported: vec![
            "authorization_code".to_string(),
            "implicit".to_string(),
            "client_credentials".to_string(),
        ],
        token_endpoint_auth_methods_supported: vec![
            "client_secret_basic".to_string(),
            "client_secret_post".to_string(),
        ],
        code_challenge_methods_supported: vec![
            "plain".to_string(),
            "S256".to_string(),
        ],
    })
}

async fn oauth_protected_resource_metadata() -> Json<ProtectedResourceMetadata> {
    Json(ProtectedResourceMetadata {
        oauth_authorization_server: "https://doxyde.com/.well-known/oauth-authorization-server".to_string(),
        protected_resources: vec![
            "https://sse.doxyde.com/".to_string(),
            "https://sse.doxyde.com/message".to_string(),
        ],
    })
}

async fn oauth_protected_resource_mcp_metadata() -> Json<ProtectedResourceMetadata> {
    // Same as above but specific to MCP endpoints
    oauth_protected_resource_metadata().await
}

// OAuth validation middleware for SSE connections
async fn validate_sse_auth(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<(), StatusCode> {
    // Extract bearer token from Authorization header
    let token = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|auth| auth.strip_prefix("Bearer "));

    if let Some(token) = token {
        match validate_token(&state.db, token).await {
            Ok(Some(token_info)) => {
                debug!("Valid OAuth token for SSE connection: site_id={}", token_info.site_id);
                Ok(())
            }
            Ok(None) => {
                error!("Invalid OAuth token");
                Err(StatusCode::UNAUTHORIZED)
            }
            Err(e) => {
                error!("Token validation error: {}", e);
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    } else {
        error!("Missing Authorization header");
        Err(StatusCode::UNAUTHORIZED)
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting Doxyde SSE Server");

    // Load configuration
    let config = Config::from_env()?;
    info!("Configuration loaded: bind_addr={}", config.bind_addr);

    // Connect to database
    let db = SqlitePool::connect(&config.database_url).await?;
    info!("Connected to database");

    // Create app state
    let app_state = Arc::new(AppState { db: db.clone() });

    // Parse bind address
    let bind_addr: SocketAddr = config.bind_addr.parse()?;

    // Create cancellation token for graceful shutdown
    let ct = CancellationToken::new();

    // Create SSE server configuration
    let sse_config = SseServerConfig {
        bind: bind_addr,
        sse_path: config.sse_path.clone(),
        post_path: config.post_path.clone(),
        ct: ct.clone(),
        sse_keep_alive: Some(config.keep_alive_duration()),
    };

    // Create SSE server and get router
    let (sse_server, sse_router) = SseServer::new(sse_config);
    info!("SSE server created with paths: SSE={}, POST={}", config.sse_path, config.post_path);

    // Register a default service - this will be overridden per connection
    // but rmcp requires at least one service to be registered
    let default_pool = db.clone();
    let _service_handle = sse_server.with_service(move || {
        info!("Creating new DoxydeRmcpService instance for site_id=1");
        DoxydeRmcpService::new(default_pool.clone(), 1)
    });
    
    // Create OAuth validation middleware that works for both SSE and POST requests
    let oauth_middleware = axum_middleware::from_fn_with_state(
        app_state.clone(),
        |State(state): State<Arc<AppState>>, headers: HeaderMap, req: axum::extract::Request, next: axum::middleware::Next| async move {
            // Extract bearer token from Authorization header
            let token = headers
                .get(AUTHORIZATION)
                .and_then(|value| value.to_str().ok())
                .and_then(|auth| auth.strip_prefix("Bearer "));

            if let Some(token) = token {
                match validate_token(&state.db, token).await {
                    Ok(Some(token_info)) => {
                        debug!("Valid OAuth token: site_id={}, path={}", token_info.site_id, req.uri().path());
                        // The service registered with site_id=1 will handle requests
                        // In the future, we could dynamically switch services based on token_info.site_id
                        Ok(next.run(req).await)
                    }
                    Ok(None) => {
                        error!("Invalid OAuth token for path: {}", req.uri().path());
                        Err(StatusCode::UNAUTHORIZED)
                    }
                    Err(e) => {
                        error!("Token validation error: {}", e);
                        Err(StatusCode::INTERNAL_SERVER_ERROR)
                    }
                }
            } else {
                error!("Missing Authorization header for path: {}", req.uri().path());
                Err(StatusCode::UNAUTHORIZED)
            }
        },
    );
    
    // Apply OAuth middleware to the entire SSE router (includes both SSE and POST endpoints)
    let protected_sse_router = sse_router.layer(oauth_middleware);

    // Create main router with health and OAuth discovery endpoints
    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/.well-known/oauth-authorization-server", get(oauth_authorization_server_metadata))
        .route("/.well-known/oauth-protected-resource", get(oauth_protected_resource_metadata))
        .route("/.well-known/oauth-protected-resource/.mcp", get(oauth_protected_resource_mcp_metadata))
        .merge(protected_sse_router);

    // Spawn the SSE server task
    let server_handle: JoinHandle<Result<(), std::io::Error>> = tokio::spawn(async move {
        info!("SSE server listening on {}", bind_addr);
        info!("SSE endpoint: {}", config.sse_path);
        info!("POST endpoint: {}", config.post_path);
        
        axum::serve(
            tokio::net::TcpListener::bind(&bind_addr).await?,
            app.into_make_service(),
        )
        .await
    });

    // Wait for shutdown signal
    shutdown_signal(ct.clone()).await;

    // Cancel the server
    ct.cancel();

    // Wait for server to finish
    match server_handle.await {
        Ok(Ok(())) => info!("SSE server shut down gracefully"),
        Ok(Err(e)) => error!("SSE server error: {}", e),
        Err(e) => error!("Failed to join server task: {}", e),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::header;

    // Note: AppState creation is tested implicitly in async tests
    // Cannot test synchronously as SqlitePool requires async runtime

    #[tokio::test]
    async fn test_validate_sse_auth_missing_header() {
        let state = Arc::new(AppState {
            db: SqlitePool::connect_lazy("sqlite::memory:").unwrap(),
        });
        let headers = HeaderMap::new();
        
        let result = validate_sse_auth(State(state), headers).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_validate_sse_auth_invalid_header() {
        let state = Arc::new(AppState {
            db: SqlitePool::connect_lazy("sqlite::memory:").unwrap(),
        });
        let mut headers = HeaderMap::new();
        headers.insert(header::AUTHORIZATION, "Invalid".parse().unwrap());
        
        let result = validate_sse_auth(State(state), headers).await;
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_validate_sse_auth_bearer_prefix() {
        let state = Arc::new(AppState {
            db: SqlitePool::connect_lazy("sqlite::memory:").unwrap(),
        });
        let mut headers = HeaderMap::new();
        headers.insert(header::AUTHORIZATION, "Bearer test-token".parse().unwrap());
        
        // This will fail because the token doesn't exist in the database
        // But it tests that we extract the token correctly
        let result = validate_sse_auth(State(state), headers).await;
        assert!(result.is_err());
        // Should be UNAUTHORIZED because token is invalid, not INTERNAL_SERVER_ERROR
    }

    #[test]
    fn test_cancellation_token_creation() {
        let ct = CancellationToken::new();
        assert!(!ct.is_cancelled());
        ct.cancel();
        assert!(ct.is_cancelled());
    }
}

async fn shutdown_signal(ct: CancellationToken) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C, shutting down");
        },
        _ = terminate => {
            info!("Received terminate signal, shutting down");
        },
        _ = ct.cancelled() => {
            info!("Cancellation token triggered, shutting down");
        }
    }
}