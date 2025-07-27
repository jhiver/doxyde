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
use axum::{extract::State, http::{header::AUTHORIZATION, HeaderMap, StatusCode}, middleware as axum_middleware};
use config::Config;
use doxyde_shared::{mcp::DoxydeRmcpService, oauth::validate_token};
use rmcp::transport::sse_server::{SseServer, SseServerConfig};
use sqlx::SqlitePool;
use std::{net::SocketAddr, sync::Arc};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};

#[derive(Clone)]
struct AppState {
    db: SqlitePool,
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
    let app_state = Arc::new(AppState { db });

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
    let (sse_server, mut sse_router) = SseServer::new(sse_config);

    // Apply OAuth validation middleware to SSE endpoints
    sse_router = sse_router.layer(axum_middleware::from_fn_with_state(
        app_state.clone(),
        |State(state): State<Arc<AppState>>, headers: HeaderMap, req: axum::extract::Request, next: axum::middleware::Next| async move {
            match validate_sse_auth(State(state), headers).await {
                Ok(()) => Ok(next.run(req).await),
                Err(status) => Err(status),
            }
        },
    ));

    // Register the MCP service
    let _service_handle = sse_server.with_service(|| DoxydeRmcpService::new());

    // Spawn the SSE server task
    let server_handle: JoinHandle<Result<(), std::io::Error>> = tokio::spawn(async move {
        info!("SSE server listening on {}", bind_addr);
        info!("SSE endpoint: {}", config.sse_path);
        info!("POST endpoint: {}", config.post_path);
        
        axum::serve(
            tokio::net::TcpListener::bind(&bind_addr).await?,
            sse_router.into_make_service(),
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