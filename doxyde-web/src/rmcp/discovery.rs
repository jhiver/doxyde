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
    extract::{State, Host},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use crate::AppState;
use anyhow::Context;
use sqlx;

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthorizationServerMetadata {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub registration_endpoint: Option<String>,
    pub scopes_supported: Vec<String>,
    pub response_types_supported: Vec<String>,
    pub response_modes_supported: Vec<String>,
    pub grant_types_supported: Vec<String>,
    pub token_endpoint_auth_methods_supported: Vec<String>,
    pub code_challenge_methods_supported: Vec<String>,
    pub service_documentation: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProtectedResourceMetadata {
    pub resource: String,
    pub bearer_methods_supported: Vec<String>,
    pub resource_documentation: Option<String>,
    pub resource_registration_endpoint: Option<String>,
    pub scopes_supported: Vec<String>,
}

fn determine_protocol(headers: &HeaderMap, host: &str) -> &'static str {
    // Check X-Forwarded-Proto header first (set by reverse proxies)
    if let Some(proto) = headers.get("x-forwarded-proto") {
        if let Ok(proto_str) = proto.to_str() {
            return match proto_str {
                "https" => "https",
                "http" => "http",
                _ => "https"
            };
        }
    }
    
    // Check if host contains a port that indicates local development
    if host.contains(":3000") || host.contains(":8000") || host.contains(":8001") || host == "localhost" {
        return "http";
    }
    
    // For doxyde.com without explicit port, check if we're behind a proxy
    // If no X-Forwarded-Proto header, assume http to avoid redirect issues
    if host == "doxyde.com" || host.starts_with("doxyde.com:") {
        return "http";
    }
    
    // Default to https for other domains
    "https"
}

pub async fn oauth_authorization_server_metadata(
    State(state): State<AppState>,
    Host(host): Host,
    headers: HeaderMap,
) -> impl IntoResponse {
    // Get site domain from database (use site_id 1 for now)
    let site_domain = match sqlx::query!("SELECT domain FROM sites WHERE id = 1")
        .fetch_one(&state.db)
        .await
        .context("Failed to fetch site domain")
    {
        Ok(row) => row.domain,
        Err(_) => {
            // Fallback to the Host header if no site exists
            host.clone()
        }
    };
    
    let protocol = determine_protocol(&headers, &site_domain);
    let base_url = format!("{}://{}", protocol, site_domain);
    
    let metadata = AuthorizationServerMetadata {
        issuer: base_url.clone(),
        authorization_endpoint: format!("{}/.oauth/authorize", base_url),
        token_endpoint: format!("{}/.oauth/token", base_url),
        registration_endpoint: Some(format!("{}/.oauth/register", base_url)),
        scopes_supported: vec![
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
        service_documentation: Some(format!("{}/docs/oauth", base_url)),
    };

    let mut headers = HeaderMap::new();
    add_cors_headers(&mut headers);
    headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
    
    (StatusCode::OK, headers, Json(metadata))
}

pub async fn oauth_protected_resource_metadata(
    State(state): State<AppState>,
    Host(host): Host,
    headers: HeaderMap,
) -> impl IntoResponse {
    // Get site domain from database (use site_id 1 for now)
    let site_domain = match sqlx::query!("SELECT domain FROM sites WHERE id = 1")
        .fetch_one(&state.db)
        .await
        .context("Failed to fetch site domain")
    {
        Ok(row) => row.domain,
        Err(_) => {
            // Fallback to the Host header if no site exists
            host.clone()
        }
    };
    
    let protocol = determine_protocol(&headers, &site_domain);
    let base_url = format!("{}://{}", protocol, site_domain);
    
    let metadata = ProtectedResourceMetadata {
        resource: format!("{}/.mcp", base_url),
        bearer_methods_supported: vec![
            "header".to_string(),
        ],
        resource_documentation: Some(format!("{}/docs/mcp", base_url)),
        resource_registration_endpoint: Some(format!("{}/.mcp/register", base_url)),
        scopes_supported: vec![
            "read".to_string(),
            "write".to_string(),
            "admin".to_string(),
        ],
    };

    let mut headers = HeaderMap::new();
    add_cors_headers(&mut headers);
    headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
    
    (StatusCode::OK, headers, Json(metadata))
}

pub async fn oauth_protected_resource_mcp_metadata(
    State(state): State<AppState>,
    Host(host): Host,
    headers: HeaderMap,
) -> impl IntoResponse {
    oauth_protected_resource_metadata(State(state), Host(host), headers).await
}

pub async fn options_handler() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    add_cors_headers(&mut headers);
    (StatusCode::NO_CONTENT, headers)
}

fn add_cors_headers(headers: &mut HeaderMap) {
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        "*".parse().unwrap(),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        "GET, OPTIONS".parse().unwrap(),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        "Authorization, Content-Type".parse().unwrap(),
    );
    headers.insert(
        header::ACCESS_CONTROL_MAX_AGE,
        "3600".parse().unwrap(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers;
    use axum::http::StatusCode;

    #[tokio::test]
    async fn test_oauth_authorization_server_metadata() {
        let state = test_helpers::create_test_app_state()
            .await
            .expect("Failed to create test state");

        let headers = HeaderMap::new();
        let response = oauth_authorization_server_metadata(
            State(state),
            Host("localhost:3000".to_string()),
            headers
        ).await.into_response();
        
        assert_eq!(response.status(), StatusCode::OK);
        
        let headers = response.headers();
        assert_eq!(headers.get(header::ACCESS_CONTROL_ALLOW_ORIGIN).unwrap(), "*");
        assert_eq!(headers.get(header::CONTENT_TYPE).unwrap(), "application/json");
    }

    #[tokio::test]
    async fn test_oauth_protected_resource_metadata() {
        let state = test_helpers::create_test_app_state()
            .await
            .expect("Failed to create test state");

        let headers = HeaderMap::new();
        let response = oauth_protected_resource_metadata(
            State(state),
            Host("localhost:3000".to_string()),
            headers
        ).await.into_response();
        
        assert_eq!(response.status(), StatusCode::OK);
        
        let headers = response.headers();
        assert_eq!(headers.get(header::ACCESS_CONTROL_ALLOW_ORIGIN).unwrap(), "*");
        assert_eq!(headers.get(header::CONTENT_TYPE).unwrap(), "application/json");
    }

    #[tokio::test]
    async fn test_options_handler() {
        let response = options_handler().await.into_response();
        
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        
        let headers = response.headers();
        assert_eq!(headers.get(header::ACCESS_CONTROL_ALLOW_ORIGIN).unwrap(), "*");
        assert_eq!(headers.get(header::ACCESS_CONTROL_ALLOW_METHODS).unwrap(), "GET, OPTIONS");
        assert_eq!(headers.get(header::ACCESS_CONTROL_ALLOW_HEADERS).unwrap(), "Authorization, Content-Type");
    }
}