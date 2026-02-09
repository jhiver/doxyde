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
    auth::CurrentUser,
    configuration::Configuration,
    content,
    db_middleware::{database_injection_middleware, SiteDatabase},
    debug_middleware::debug_form_middleware,
    error_middleware::error_enhancer_middleware,
    handlers,
    rate_limit::login_rate_limit_middleware,
    request_logging::request_logging_middleware,
    rmcp,
    security_headers::create_security_headers_middleware,
    session_activity::update_session_activity,
    site_resolver::{self, site_resolver_middleware},
    AppState,
};
use axum::extract::{DefaultBodyLimit, Query, State};
use axum::http::StatusCode;
use axum::response::Response;
use axum::{
    middleware,
    routing::{delete, get, post},
    Router,
};
use axum_extra::extract::Host;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;

pub fn create_router(state: AppState) -> Router {
    let max_upload_size = state.config.max_upload_size;

    // Load the configuration to get security headers config
    let config = Configuration::load().expect("Failed to load configuration");
    let headers_middleware = create_security_headers_middleware(config.security.headers);

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
        // Image preview for draft components
        .route("/.image-preview", get(image_preview))
        // MCP routes
        .route("/.mcp", post(rmcp::handle_http))
        // OAuth management (admin only)
        .route("/.mcp/token", post(rmcp::create_token))
        .route("/.mcp/tokens", get(rmcp::list_tokens))
        .route("/.mcp/token/{id}", delete(rmcp::revoke_token))
        // OAuth metadata discovery endpoints
        .route(
            "/.well-known/oauth-authorization-server",
            get(rmcp::oauth_authorization_server_metadata).options(rmcp::options_handler),
        )
        .route(
            "/.well-known/oauth-protected-resource",
            get(rmcp::oauth_protected_resource_metadata).options(rmcp::options_handler),
        )
        .route(
            "/.well-known/oauth-protected-resource/.mcp",
            get(rmcp::oauth_protected_resource_mcp_metadata).options(rmcp::options_handler),
        )
        // OAuth2 endpoints
        .route(
            "/.oauth/register",
            post(rmcp::register_client).options(rmcp::oauth_options),
        )
        .route(
            "/.oauth/authorize",
            get(rmcp::authorize)
                .post(rmcp::authorize_consent)
                .options(rmcp::oauth_options),
        )
        .route(
            "/.oauth/token",
            post(rmcp::token).options(rmcp::oauth_options),
        )
        // Per-site CSS and assets
        .route("/.site-css", get(handlers::site_assets::site_css_handler))
        .route(
            "/.site-assets/{*path}",
            get(handlers::site_assets::site_asset_handler),
        )
        // Dynamic content routes (last, to catch all)
        .fallback(get(content::content_handler).post(content::content_post_handler))
        // Add middleware
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
        .layer(middleware::from_fn(headers_middleware))
        // Add site resolution and database injection middleware
        .layer(middleware::from_fn_with_state(
            state.db_router.clone(),
            database_injection_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state.config.clone(),
            site_resolver_middleware,
        ))
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

// Image preview handler wrapper
async fn image_preview(
    Host(host): Host,
    State(state): State<AppState>,
    Query(params): Query<handlers::image_serve::ImagePreviewQuery>,
    user: CurrentUser,
    site_ctx: site_resolver::SiteContext,
    SiteDatabase(db): SiteDatabase,
) -> Result<Response, StatusCode> {
    // Use the full host as domain (including port)
    let domain = &host;

    tracing::debug!(
        "Image preview route - host: {}, domain: {}, component_id: {}",
        host,
        domain,
        params.component_id
    );
    let site = crate::site_config::get_site_config(&db, domain)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get site config for domain '{}': {}", domain, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tracing::debug!("Site found: {:?}", site.id);

    // Call the actual handler
    handlers::image_preview_handler(
        State(state),
        site,
        Query(params),
        user,
        site_ctx,
        SiteDatabase(db),
    )
    .await
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
    async fn test_image_preview_endpoint_exists() {
        // Create test app state
        let state = crate::test_helpers::create_test_app_state()
            .await
            .expect("Failed to create test state");

        // Create router and test server
        let app = create_router(state);
        let server = TestServer::new(app).expect("Failed to create test server");

        // Test that /.image-preview without params returns BAD_REQUEST (missing component_id)
        let response = server.get("/.image-preview").await;
        // Should return 400 Bad Request because component_id is missing
        response.assert_status(StatusCode::BAD_REQUEST);
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
        response.assert_header(header::CONTENT_SECURITY_POLICY, "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self';");
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
