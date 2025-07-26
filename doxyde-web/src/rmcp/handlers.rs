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
};
use rmcp::transport::sse_server::{SseServer, SseServerConfig};
use std::net::SocketAddr;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use tracing::{error, info};

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