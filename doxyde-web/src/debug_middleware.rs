// Copyright (C) 2025 Doxyde
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
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
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};

/// Debug middleware that logs raw form data for POST requests
pub async fn debug_form_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let (parts, body) = request.into_parts();
    
    // Only debug POST requests to .edit (which handles save_draft)
    if parts.method == axum::http::Method::POST && parts.uri.path().ends_with(".edit") {
        // Read the entire body
        let bytes = match axum::body::to_bytes(body, usize::MAX).await {
            Ok(bytes) => bytes,
            Err(_) => return Err(StatusCode::BAD_REQUEST),
        };
        
        // Log the raw form data
        let body_str = String::from_utf8_lossy(&bytes);
        tracing::info!("=== RAW FORM DATA DEBUG ===");
        tracing::info!("URI: {}", parts.uri);
        tracing::info!("Content-Type: {:?}", parts.headers.get("content-type"));
        tracing::info!("Body length: {} bytes", bytes.len());
        tracing::info!("Raw body: {}", body_str);
        
        // Parse form data manually to see what we get
        let params: Vec<(String, String)> = String::from_utf8_lossy(&bytes)
            .split('&')
            .filter_map(|pair| {
                let mut parts = pair.split('=');
                match (parts.next(), parts.next()) {
                    (Some(k), Some(v)) => Some((k.to_string(), v.to_string())),
                    _ => None
                }
            })
            .collect();
        
        tracing::info!("=== PARSED URL-ENCODED PARAMS ===");
        // Count occurrences of component_ids
        let component_ids_count = params.iter()
            .filter(|(k, _)| k == "component_ids")
            .count();
            
        tracing::info!("Field counts - component_ids: {}", component_ids_count);
        
        // Reconstruct the request with the body
        let request = Request::from_parts(parts, Body::from(bytes));
        Ok(next.run(request).await)
    } else {
        // For other requests, just pass through
        let request = Request::from_parts(parts, body);
        Ok(next.run(request).await)
    }
}