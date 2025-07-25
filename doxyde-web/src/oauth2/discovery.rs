use axum::{
    extract::{Host, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;

use crate::state::AppState;

/// OAuth2 Authorization Server Metadata
/// https://datatracker.ietf.org/doc/html/rfc8414
pub async fn oauth_authorization_server_handler(
    Host(host): Host,
    State(state): State<AppState>,
) -> impl IntoResponse {
    tracing::info!("DEBUGGING: oauth_authorization_server_handler called");

    let base_url = get_base_url_from_host(&host, &state);

    let metadata = json!({
        "issuer": base_url,
        "authorization_endpoint": format!("{}/.oauth/authorize", base_url),
        "token_endpoint": format!("{}/.oauth/token", base_url),
        "registration_endpoint": format!("{}/.oauth/register", base_url),
        "revocation_endpoint": format!("{}/.oauth/revoke", base_url),
        "scopes_supported": ["mcp:read", "mcp:write"],
        "response_types_supported": ["code"],
        "response_modes_supported": ["query"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "code_challenge_methods_supported": ["S256"],
        "token_endpoint_auth_methods_supported": ["client_secret_basic", "client_secret_post"],
        "service_documentation": "https://github.com/jhiver/doxyde",
        "ui_locales_supported": ["en"],
    });

    (StatusCode::OK, Json(metadata))
}

/// OpenID Connect Discovery (alias for OAuth2 metadata)
pub async fn openid_configuration_handler(host: Host, state: State<AppState>) -> impl IntoResponse {
    tracing::info!("DEBUGGING: openid_configuration_handler called");
    oauth_authorization_server_handler(host, state).await
}

/// OAuth Protected Resource Metadata
/// Indicates this resource server accepts OAuth2 tokens
pub async fn oauth_protected_resource_handler(
    Host(host): Host,
    State(state): State<AppState>,
) -> impl IntoResponse {
    tracing::info!("DEBUGGING: oauth_protected_resource_handler called");

    let base_url = get_base_url_from_host(&host, &state);

    // According to RFC 9728, we need to indicate the authorization servers
    // The resource field should be the base URL, not the MCP endpoint
    // But Claude Desktop requires the mcp_endpoint field
    let metadata = json!({
        "resource": base_url.clone(),
        "authorization_servers": [base_url.clone()],
        "bearer_methods_supported": ["header"],
        "scopes_supported": ["mcp:read", "mcp:write"],
        "resource_documentation": "https://github.com/jhiver/doxyde",
        "mcp_endpoint": format!("{}/.mcp", base_url),
    });

    (StatusCode::OK, Json(metadata))
}

/// Handler for .well-known directory listing
pub async fn well_known_directory_handler(
    Host(host): Host,
    State(state): State<AppState>,
) -> impl IntoResponse {
    tracing::info!("DEBUGGING: well_known_directory_handler called");

    let base_url = get_base_url_from_host(&host, &state);

    let directory = json!({
        "links": [
            {
                "rel": "oauth-authorization-server",
                "href": format!("{}/.well-known/oauth-authorization-server", base_url)
            },
            {
                "rel": "openid-configuration",
                "href": format!("{}/.well-known/openid-configuration", base_url)
            },
            {
                "rel": "oauth-protected-resource",
                "href": format!("{}/.well-known/oauth-protected-resource", base_url)
            }
        ]
    });

    (StatusCode::OK, Json(directory))
}

/// Get base URL from host header
fn get_base_url_from_host(host: &str, _state: &AppState) -> String {
    // Check if we're behind a proxy (looking at common patterns)
    // If host contains port, use as-is, otherwise assume HTTPS
    if host.contains(':') && !host.starts_with("localhost:") {
        // Has explicit port, likely HTTP in development
        format!("http://{}", host)
    } else {
        // Production domain or localhost, use HTTPS
        format!("https://{}", host)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::create_test_app_state;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_oauth_discovery() -> anyhow::Result<()> {
        let state = create_test_app_state().await?;
        let app = crate::routes::create_router(state);

        let request = Request::builder()
            .uri("/.well-known/oauth-authorization-server")
            .header("host", "example.com")
            .body(Body::empty())?;

        let response = app.oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), 10_000_000).await?;
        let json: serde_json::Value = serde_json::from_slice(&body)?;

        assert!(json.get("authorization_endpoint").is_some());
        assert!(json.get("token_endpoint").is_some());
        assert!(json.get("registration_endpoint").is_some());
        assert_eq!(
            json.get("code_challenge_methods_supported").unwrap(),
            &json!(["S256"])
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_well_known_directory() -> anyhow::Result<()> {
        let state = create_test_app_state().await?;
        let app = crate::routes::create_router(state);

        let request = Request::builder()
            .uri("/.well-known")
            .header("host", "example.com")
            .body(Body::empty())?;

        let response = app.clone().oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), 10_000_000).await?;
        let json: serde_json::Value = serde_json::from_slice(&body)?;

        assert!(json.get("links").is_some());
        let links = json.get("links").unwrap().as_array().unwrap();
        assert_eq!(links.len(), 3);

        Ok(())
    }
}
