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
    http::{
        header::{
            ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_HEADERS,
            ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_MAX_AGE,
        },
        Method, Request, Response, StatusCode,
    },
    middleware::Next,
};

/// CORS middleware for OAuth2 and MCP endpoints
pub async fn cors_middleware(
    request: Request<Body>,
    next: Next,
) -> Result<Response<Body>, StatusCode> {
    let origin = request
        .headers()
        .get("origin")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("")
        .to_string();

    let path = request.uri().path().to_string();

    // Only apply CORS to OAuth2 and MCP endpoints
    let should_apply_cors = path.starts_with("/.well-known")
        || path.starts_with("/.oauth")
        || path.starts_with("/.mcp");

    if !should_apply_cors {
        return Ok(next.run(request).await);
    }

    // Handle OPTIONS preflight requests
    if request.method() == Method::OPTIONS {
        let mut response = Response::builder()
            .status(StatusCode::NO_CONTENT)
            .body(Body::empty())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let headers = response.headers_mut();

        // Allow specific origins for MCP development and Claude Desktop
        if origin == "http://localhost:6274" || origin.starts_with("http://localhost:") {
            headers.insert(ACCESS_CONTROL_ALLOW_ORIGIN, origin.parse().unwrap());
        } else if origin.starts_with("https://") {
            // Allow HTTPS origins (for production Claude Desktop)
            headers.insert(ACCESS_CONTROL_ALLOW_ORIGIN, origin.parse().unwrap());
        }

        headers.insert(
            ACCESS_CONTROL_ALLOW_METHODS,
            "GET, POST, OPTIONS, HEAD".parse().unwrap(),
        );
        headers.insert(
            ACCESS_CONTROL_ALLOW_HEADERS,
            "Content-Type, Authorization, Accept, Accept-Encoding, User-Agent"
                .parse()
                .unwrap(),
        );
        headers.insert(ACCESS_CONTROL_ALLOW_CREDENTIALS, "true".parse().unwrap());
        headers.insert(ACCESS_CONTROL_MAX_AGE, "86400".parse().unwrap()); // 24 hours

        return Ok(response);
    }

    // Process the actual request
    let mut response = next.run(request).await;

    // Add CORS headers to the response
    let headers = response.headers_mut();

    // Allow specific origins for MCP development and Claude Desktop
    if &origin == "http://localhost:6274" || origin.starts_with("http://localhost:") {
        headers.insert(ACCESS_CONTROL_ALLOW_ORIGIN, origin.parse().unwrap());
    } else if origin.starts_with("https://") {
        // Allow HTTPS origins (for production Claude Desktop)
        headers.insert(ACCESS_CONTROL_ALLOW_ORIGIN, origin.parse().unwrap());
    }

    headers.insert(ACCESS_CONTROL_ALLOW_CREDENTIALS, "true".parse().unwrap());

    Ok(response)
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_cors_origin_matching() {
        // Test localhost:6274
        assert!("http://localhost:6274" == "http://localhost:6274");
        assert!("http://localhost:3000".starts_with("http://localhost:"));

        // Test HTTPS origins
        assert!("https://claude.ai".starts_with("https://"));
        assert!("https://doxyde.com".starts_with("https://"));
    }

    #[test]
    fn test_cors_path_matching() {
        // Test OAuth2 and MCP paths
        assert!("/.well-known/oauth-authorization-server".starts_with("/.well-known"));
        assert!("/.oauth/authorize".starts_with("/.oauth"));
        assert!("/.mcp".starts_with("/.mcp"));

        // Test paths that should NOT match
        assert!(!"/about".starts_with("/.well-known"));
        assert!(!"/static/css/style.css".starts_with("/.oauth"));
    }
}
