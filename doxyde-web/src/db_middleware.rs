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
    extract::{FromRequestParts, Request, State},
    http::{request::Parts, StatusCode},
    middleware::Next,
    response::Response,
};
use sqlx::SqlitePool;

use crate::{db_router::DatabaseRouter, site_resolver::RequestSiteExt};

/// Database pool that gets injected per-request based on site context
#[derive(Clone)]
pub struct SiteDatabase(pub SqlitePool);

// Implement extractor for SiteDatabase
impl<S> FromRequestParts<S> for SiteDatabase
where
    S: Send + Sync,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<SiteDatabase>()
            .cloned()
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)
    }
}

/// Extension trait to get site-specific database from request
pub trait RequestDbExt {
    fn site_db(&self) -> Option<&SiteDatabase>;
}

impl<B> RequestDbExt for Request<B> {
    fn site_db(&self) -> Option<&SiteDatabase> {
        self.extensions().get::<SiteDatabase>()
    }
}

/// Middleware that injects the appropriate database pool based on site context
pub async fn database_injection_middleware(
    State(router): State<DatabaseRouter>,
    mut request: Request,
    next: Next,
) -> Response {
    // Get site context from request (set by site_resolver_middleware)
    let site_info = request.site_context().map(|ctx| ctx.domain.clone());

    if let Some(domain) = site_info {
        // Get site context again for the router
        if let Some(context) = request.site_context() {
            // Get appropriate database pool
            match router.get_pool(context).await {
                Ok(pool) => {
                    // Inject pool into request extensions
                    request.extensions_mut().insert(SiteDatabase(pool));
                    tracing::debug!("Injected database pool for site: {}", domain);
                }
                Err(e) => {
                    tracing::error!("Failed to get database pool for site '{}': {:?}", domain, e);
                    // Always fail fast to avoid data corruption
                    return axum::response::Response::builder()
                        .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
                        .body(axum::body::Body::from("Database initialization failed"))
                        .unwrap();
                }
            }
        }
    } else {
        tracing::error!("No site context found in request");
        // Fail fast - no fallback
        return axum::response::Response::builder()
            .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
            .body(axum::body::Body::from("Site context missing"))
            .unwrap();
    }

    // Continue with the request
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{config::Config, site_resolver::SiteContext};
    use axum::{
        body::Body,
        http::{Request as HttpRequest, StatusCode},
        middleware::from_fn_with_state,
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    fn create_test_config(sites_directory: Option<String>) -> Config {
        let has_sites_dir = sites_directory.is_some();
        Config {
            database_url: "sqlite::memory:".to_string(),
            host: "localhost".to_string(),
            port: 3000,
            templates_dir: "templates".to_string(),
            session_secret: "test".to_string(),
            development_mode: false,
            uploads_dir: "uploads".to_string(),
            max_upload_size: 1048576,
            secure_cookies: false,
            session_timeout_minutes: 1440,
            login_attempts_per_minute: 5,
            api_requests_per_minute: 60,
            csrf_enabled: true,
            csrf_token_expiry_hours: 24,
            csrf_token_length: 32,
            csrf_header_name: "X-CSRF-Token".to_string(),
            static_files_max_age: 86400,
            oauth_token_expiry: 3600,
            sites_directory: sites_directory.unwrap_or_else(|| "".to_string()),
            multi_site_mode: has_sites_dir,
        }
    }

    async fn test_handler(request: Request) -> Result<String, StatusCode> {
        let db = request.site_db().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

        // Just verify we got a pool
        Ok(format!("pool_size:{}", db.0.size()))
    }

    #[tokio::test]
    async fn test_database_injection_with_site_context() {
        let config = create_test_config(None);
        let router = DatabaseRouter::new(config).await.unwrap();

        let app = Router::new()
            .route("/", get(test_handler))
            .layer(from_fn_with_state(
                router.clone(),
                database_injection_middleware,
            ));

        // Create request with site context
        let mut request = HttpRequest::builder().uri("/").body(Body::empty()).unwrap();

        // Inject site context
        request
            .extensions_mut()
            .insert(SiteContext::legacy("example.com".to_string()));

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.starts_with("pool_size:"));
    }

    #[tokio::test]
    async fn test_database_injection_without_site_context() {
        let config = create_test_config(None);
        let router = DatabaseRouter::new(config).await.unwrap();

        let app = Router::new()
            .route("/", get(test_handler))
            .layer(from_fn_with_state(
                router.clone(),
                database_injection_middleware,
            ));

        // Request without site context
        let request = HttpRequest::builder().uri("/").body(Body::empty()).unwrap();

        let response = app.oneshot(request).await.unwrap();

        // In multi-database architecture, requests without site context return 500
        // because there's no legacy/default pool - every request needs a site
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_full_middleware_chain() {
        use crate::site_resolver::site_resolver_middleware;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let sites_dir = temp_dir.path().to_string_lossy().to_string();

        let config = create_test_config(Some(sites_dir));
        let router = DatabaseRouter::new(config.clone()).await.unwrap();

        let app = Router::new()
            .route("/", get(test_handler))
            // Apply middleware in the correct order
            .layer(from_fn_with_state(
                router.clone(),
                database_injection_middleware,
            ))
            .layer(from_fn_with_state(config.clone(), site_resolver_middleware));

        // Request with host header
        let request = HttpRequest::builder()
            .header("Host", "test.example.com")
            .uri("/")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        // Should work with site-specific pool
        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.starts_with("pool_size:"));
    }

    #[tokio::test]
    async fn test_concurrent_site_access() {
        use crate::site_resolver::site_resolver_middleware;
        use std::sync::Arc;
        use tempfile::TempDir;
        use tokio::sync::Mutex;

        let temp_dir = TempDir::new().unwrap();
        let sites_dir = temp_dir.path().to_string_lossy().to_string();

        let config = create_test_config(Some(sites_dir.clone()));
        let router = Arc::new(DatabaseRouter::new(config.clone()).await.unwrap());

        // Track which sites were accessed
        let accessed_sites = Arc::new(Mutex::new(Vec::new()));
        let accessed_sites_clone = accessed_sites.clone();

        // Handler that records which site was accessed
        let test_handler = move |request: Request| {
            let accessed = accessed_sites_clone.clone();
            async move {
                if let Some(context) = request.site_context() {
                    let mut sites = accessed.lock().await;
                    sites.push(context.domain.clone());
                }

                let db = request.site_db().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

                Ok::<_, StatusCode>(format!("pool_size:{}", db.0.size()))
            }
        };

        let app = Router::new()
            .route("/", get(test_handler))
            .layer(from_fn_with_state(
                router.as_ref().clone(),
                database_injection_middleware,
            ))
            .layer(from_fn_with_state(config.clone(), site_resolver_middleware));

        // Simulate concurrent requests to the same site
        let app = Arc::new(app);
        let mut handles = vec![];

        for _i in 0..5 {
            let app_clone = app.clone();
            let handle = tokio::spawn(async move {
                let request = HttpRequest::builder()
                    .header("Host", "site1.example.com")
                    .uri("/")
                    .body(Body::empty())
                    .unwrap();

                let response = tower::ServiceExt::oneshot(app_clone.as_ref().clone(), request)
                    .await
                    .unwrap();

                assert_eq!(response.status(), StatusCode::OK);
            });
            handles.push(handle);
        }

        // Wait for all requests to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // Verify all requests were for the same site
        let sites = accessed_sites.lock().await;
        assert_eq!(sites.len(), 5);
        assert!(sites.iter().all(|s| s == "site1.example.com"));
    }
}
