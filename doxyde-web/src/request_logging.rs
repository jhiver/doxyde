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
    body::Body,
    http::{Request, Response},
    middleware::Next,
};
use std::time::Instant;
use tracing::info;

/// Middleware to log all incoming requests with headers for debugging
pub async fn request_logging_middleware(
    request: Request<Body>,
    next: Next,
) -> Result<Response<Body>, axum::http::StatusCode> {
    let start = Instant::now();
    
    // Extract request information
    let method = request.method().clone();
    let uri = request.uri().clone();
    let path = uri.path();
    let query = uri.query().unwrap_or("");
    
    // Log request details - especially for .well-known paths
    if path.starts_with("/.well-known") || path.starts_with(".well-known") {
        // Comprehensive logging for .well-known requests
        info!(
            "REQUEST_DEBUG: .well-known request - method={} path={} query={} uri={}",
            method, path, query, uri
        );
        
        // Log all headers
        for (name, value) in request.headers() {
            if let Ok(value_str) = value.to_str() {
                info!(
                    "REQUEST_DEBUG: .well-known header - {}={}",
                    name, value_str
                );
            }
        }
        
        // Log the full URI components
        info!(
            "REQUEST_DEBUG: .well-known URI components - scheme={:?} authority={:?} path_and_query={:?}",
            uri.scheme_str(),
            uri.authority().map(|a| a.as_str()),
            uri.path_and_query().map(|pq| pq.as_str())
        );
    } else {
        // Normal request logging
        info!(
            "REQUEST: {} {} {}",
            method,
            path,
            if query.is_empty() { "" } else { "?" }
        );
    }
    
    // Process the request
    let response = next.run(request).await;
    
    // Log response status and timing
    let duration = start.elapsed();
    let status = response.status();
    
    if path.starts_with("/.well-known") || path.starts_with(".well-known") {
        info!(
            "RESPONSE_DEBUG: .well-known response - path={} status={} duration={:?}",
            path, status, duration
        );
    } else {
        info!(
            "RESPONSE: {} {} - {} in {:?}",
            method, path, status, duration
        );
    }
    
    Ok(response)
}