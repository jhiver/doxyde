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

use crate::{
    content, cors::cors_middleware, debug_middleware::debug_form_middleware,
    error_middleware::error_enhancer_middleware, handlers, rate_limit::login_rate_limit_middleware,
    request_logging::request_logging_middleware, security_headers::security_headers_middleware,
    session_activity::update_session_activity, AppState,
};
use axum::extract::{DefaultBodyLimit, State};
use axum::{http::StatusCode, middleware, routing, routing::get, Router};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

pub fn create_router(state: AppState) -> Router {
    let max_upload_size = state.config.max_upload_size;

    Router::new()
        // Health check
        .route("/.health", get(health))
        // Static files
        .nest_service("/.static", ServeDir::new("static"))
        // System routes (dot-prefixed)
        .route(
            "/.login",
            get(handlers::login_form)
                .post(handlers::login)
                .layer(middleware::from_fn_with_state(
                    state.login_rate_limiter.clone(),
                    login_rate_limit_middleware,
                )),
        )
        .route("/.logout", get(handlers::logout).post(handlers::logout))
        // MCP Token management
        .route(
            "/.tokens",
            get(handlers::list_tokens_handler).post(handlers::create_token_handler),
        )
        .route("/.tokens/:token_id", get(handlers::show_token_handler))
        .route(
            "/.tokens/:token_id/revoke",
            get(handlers::revoke_token_handler).post(handlers::revoke_token_handler),
        )
        // OAuth2 endpoints
        .route(
            "/.well-known",
            get(crate::oauth2::discovery::well_known_directory_handler)
                .options(|| async { StatusCode::NO_CONTENT }),
        )
        .route(
            "/.well-known/",
            get(crate::oauth2::discovery::well_known_directory_handler)
                .options(|| async { StatusCode::NO_CONTENT }),
        )
        .route(
            "/.well-known/oauth-authorization-server",
            get(crate::oauth2::discovery::oauth_authorization_server_handler)
                .options(|| async { StatusCode::NO_CONTENT }),
        )
        .route(
            "/.well-known/openid-configuration",
            get(crate::oauth2::discovery::openid_configuration_handler)
                .options(|| async { StatusCode::NO_CONTENT }),
        )
        .route(
            "/.well-known/oauth-protected-resource",
            get(crate::oauth2::discovery::oauth_protected_resource_handler)
                .options(|| async { StatusCode::NO_CONTENT }),
        )
        // Workaround: Claude Desktop incorrectly appends /.mcp to discovery URLs
        .route(
            "/.well-known/oauth-authorization-server/.mcp",
            get(crate::oauth2::discovery::oauth_authorization_server_handler)
                .options(|| async { StatusCode::NO_CONTENT }),
        )
        .route(
            "/.well-known/oauth-protected-resource/.mcp",
            get(crate::oauth2::discovery::oauth_protected_resource_handler)
                .options(|| async { StatusCode::NO_CONTENT }),
        )
        .route(
            "/.oauth/register",
            routing::post(crate::oauth2::client_registration::client_registration_handler)
                .options(|| async { StatusCode::NO_CONTENT }),
        )
        .route(
            "/.oauth/authorize",
            get(crate::oauth2::authorization::authorization_handler)
                .post(crate::oauth2::authorization::consent_handler)
                .options(|| async { StatusCode::NO_CONTENT }),
        )
        .route(
            "/.oauth/token",
            routing::post(crate::oauth2::token::token_handler)
                .options(|| async { StatusCode::NO_CONTENT }),
        )
        .route(
            "/.oauth/revoke",
            routing::post(crate::oauth2::token::revoke_handler)
                .options(|| async { StatusCode::NO_CONTENT }),
        )
        // OAuth2-protected MCP endpoint
        .route(
            "/.mcp",
            routing::post(handlers::mcp_oauth_handler)
                .head(handlers::mcp_oauth_head_handler)
                .options(|| async { StatusCode::NO_CONTENT })
                .layer(middleware::from_fn_with_state(
                    state.api_rate_limiter.clone(),
                    |State(limiter): State<crate::rate_limit::SharedRateLimiter>,
                     request: axum::http::Request<axum::body::Body>,
                     next: axum::middleware::Next| async move {
                        match limiter.check() {
                            Ok(_) => Ok(next.run(request).await),
                            Err(_) => Err(axum::http::StatusCode::TOO_MANY_REQUESTS),
                        }
                    },
                )),
        )
        // OAuth2 MCP SSE endpoint
        .route(
            "/.mcp/sse",
            routing::get(handlers::mcp_oauth_sse_handler)
                .options(|| async { StatusCode::NO_CONTENT })
                .layer(middleware::from_fn_with_state(
                    state.api_rate_limiter.clone(),
                    |State(limiter): State<crate::rate_limit::SharedRateLimiter>,
                     request: axum::http::Request<axum::body::Body>,
                     next: axum::middleware::Next| async move {
                        match limiter.check() {
                            Ok(_) => Ok(next.run(request).await),
                            Err(_) => Err(axum::http::StatusCode::TOO_MANY_REQUESTS),
                        }
                    },
                )),
        )
        // Legacy MCP Server endpoint (backward compatibility)
        .route(
            "/.mcp/:token_id",
            routing::post(handlers::mcp_legacy_handler).layer(middleware::from_fn_with_state(
                state.api_rate_limiter.clone(),
                |State(limiter): State<crate::rate_limit::SharedRateLimiter>,
                 request: axum::http::Request<axum::body::Body>,
                 next: axum::middleware::Next| async move {
                    match limiter.check() {
                        Ok(_) => Ok(next.run(request).await),
                        Err(_) => Err(axum::http::StatusCode::TOO_MANY_REQUESTS),
                    }
                },
            )),
        )
        // Dynamic content routes (last, to catch all)
        .fallback(get(content::content_handler).post(content::content_post_handler))
        // Add middleware
        .layer(middleware::from_fn(cors_middleware))
        .layer(middleware::from_fn(request_logging_middleware))
        .layer(middleware::from_fn(debug_form_middleware))
        .layer(middleware::from_fn_with_state(
            Arc::new(state.clone()),
            error_enhancer_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            Arc::new(state.clone()),
            update_session_activity,
        ))
        .layer(middleware::from_fn(security_headers_middleware))
        .layer(
            ServiceBuilder::new()
                .layer(DefaultBodyLimit::max(max_upload_size))
                .layer(TraceLayer::new_for_http()),
        )
        .with_state(state)
}

// Health check handler
async fn health() -> &'static str {
    "OK"
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum_test::TestServer;

    #[tokio::test]
    async fn test_health_endpoint_uses_dot_prefix() {
        // Create test app state
        let state = crate::test_helpers::create_test_app_state()
            .await
            .expect("Failed to create test state");

        // Create router and test server
        let app = create_router(state);
        let server = TestServer::new(app).expect("Failed to create test server");

        // Test that /.health works
        let response = server.get("/.health").await;
        response.assert_status(StatusCode::OK);
        response.assert_text("OK");

        // Test that /health does NOT work (should be 404)
        let response = server.get("/health").await;
        response.assert_status(StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_health_endpoint_has_security_headers() {
        use axum::http::header;

        // Create test app state
        let state = crate::test_helpers::create_test_app_state()
            .await
            .expect("Failed to create test state");

        // Create router and test server
        let app = create_router(state);
        let server = TestServer::new(app).expect("Failed to create test server");

        // Test that security headers are present
        let response = server.get("/.health").await;
        response.assert_status(StatusCode::OK);

        // Check all security headers
        response.assert_header(header::CONTENT_SECURITY_POLICY, "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; img-src 'self' data: https:; font-src 'self' https://fonts.gstatic.com; connect-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self';");
        response.assert_header(
            header::STRICT_TRANSPORT_SECURITY,
            "max-age=31536000; includeSubDomains",
        );
        response.assert_header(header::X_FRAME_OPTIONS, "DENY");
        response.assert_header(header::X_CONTENT_TYPE_OPTIONS, "nosniff");
        response.assert_header(header::REFERRER_POLICY, "strict-origin-when-cross-origin");
        response.assert_header(
            "Permissions-Policy",
            "geolocation=(), camera=(), microphone=()",
        );
    }

    #[tokio::test]
    async fn test_static_endpoint_uses_dot_prefix() {
        // Create test app state
        let state = crate::test_helpers::create_test_app_state()
            .await
            .expect("Failed to create test state");

        // Create router and test server
        let app = create_router(state);
        let server = TestServer::new(app).expect("Failed to create test server");

        // Test that /.static/js/clipboard.js would work (if file exists)
        // We expect 404 since the file doesn't exist in test environment
        let response = server.get("/.static/js/clipboard.js").await;
        // Static file server returns 404 for missing files
        response.assert_status(StatusCode::NOT_FOUND);

        // Test that /static does NOT work (should be handled by fallback)
        let response = server.get("/static/js/clipboard.js").await;
        // This should hit the fallback handler, not static file server
        // The fallback handler would typically return a different error
        response.assert_status(StatusCode::NOT_FOUND);
    }
}
