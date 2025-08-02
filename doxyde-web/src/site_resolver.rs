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
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use axum_extra::extract::Host;
use std::path::PathBuf;

use crate::{config::Config, domain_utils};

/// Site context that gets attached to requests
#[derive(Debug, Clone)]
pub struct SiteContext {
    /// The domain name for this site
    pub domain: String,
    /// The sanitized domain for filesystem paths
    pub sanitized_domain: String,
    /// The site directory path
    pub site_directory: PathBuf,
}

impl SiteContext {
    /// Create a site context
    pub fn new(domain: String, base_path: &PathBuf) -> Self {
        let site_dir = domain_utils::resolve_site_directory(base_path, &domain);
        let sanitized = site_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&domain)
            .to_string();

        Self {
            domain,
            sanitized_domain: sanitized,
            site_directory: site_dir,
        }
    }

    /// Get the database path for this site
    pub fn database_path(&self) -> String {
        // Use site-specific database
        let db_path = self.site_directory.join("site.db");
        format!("sqlite:{}", db_path.to_string_lossy())
    }

    /// Get the templates directory for this site
    pub fn templates_path(&self) -> PathBuf {
        // Use site-specific templates
        self.site_directory.join("templates")
    }

    /// Get the uploads directory for this site
    pub fn uploads_path(&self) -> PathBuf {
        // Use site-specific uploads
        self.site_directory.join("uploads")
    }
}

/// Extension trait to add site context methods to Request
pub trait RequestSiteExt {
    fn site_context(&self) -> Option<&SiteContext>;
}

impl RequestSiteExt for Request {
    fn site_context(&self) -> Option<&SiteContext> {
        self.extensions().get::<SiteContext>()
    }
}

/// Middleware that resolves the site from the request host
pub async fn site_resolver_middleware(
    State(config): State<Config>,
    Host(host): Host,
    mut request: Request,
    next: Next,
) -> Response {
    // Get sites directory
    let base_path = match config.get_sites_directory() {
        Ok(path) => path,
        Err(e) => {
            tracing::error!("Failed to get sites directory: {:?}", e);
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Site configuration error"))
                .unwrap();
        }
    };

    // Create site context
    let context = SiteContext::new(host, &base_path);

    // Attach context to request extensions
    request.extensions_mut().insert(context);

    // Continue with the request
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::{Request as HttpRequest, StatusCode},
        middleware::from_fn_with_state,
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    async fn test_handler(request: Request) -> Result<String, StatusCode> {
        let context = request
            .site_context()
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(format!(
            "domain:{},sanitized:{}",
            context.domain, context.sanitized_domain
        ))
    }

    #[tokio::test]
    async fn test_site_resolver() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let sites_dir = temp_dir.path().to_string_lossy().to_string();

        let config = Config {
            database_url: "sqlite:test.db".to_string(),
            host: "localhost".to_string(),
            port: 3000,
            templates_dir: "templates".to_string(),
            session_secret: "test".to_string(),
            development_mode: false,
            uploads_dir: "uploads".to_string(),
            max_upload_size: 1048576,
            secure_cookies: false,
            session_timeout_minutes: 1440,
            sites_directory: sites_dir,
        };

        let app = Router::new()
            .route("/", get(test_handler))
            .layer(from_fn_with_state(config.clone(), site_resolver_middleware));

        let response = app
            .oneshot(
                HttpRequest::builder()
                    .header("Host", "example.com")
                    .uri("/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.starts_with("domain:example.com,sanitized:example-com-"));
    }

    #[tokio::test]
    async fn test_site_resolver_with_port() {
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let sites_dir = temp_dir.path().to_string_lossy().to_string();

        let config = Config {
            database_url: "sqlite:test.db".to_string(),
            host: "localhost".to_string(),
            port: 3000,
            templates_dir: "templates".to_string(),
            session_secret: "test".to_string(),
            development_mode: false,
            uploads_dir: "uploads".to_string(),
            max_upload_size: 1048576,
            secure_cookies: false,
            session_timeout_minutes: 1440,
            sites_directory: sites_dir,
        };

        let app = Router::new()
            .route("/", get(test_handler))
            .layer(from_fn_with_state(config.clone(), site_resolver_middleware));

        let response = app
            .oneshot(
                HttpRequest::builder()
                    .header("Host", "example.com:8080")
                    .uri("/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        // The sanitized domain will be based on the base domain
        assert!(body_str.starts_with("domain:example.com:8080,sanitized:example-com-"));
    }

    #[test]
    fn test_site_context() {
        let base_path = PathBuf::from("/sites");
        let context = SiteContext::new("example.com".to_string(), &base_path);
        assert_eq!(context.domain, "example.com");
        assert!(context.sanitized_domain.starts_with("example-com-"));
        assert!(context.site_directory.starts_with("/sites"));
    }

    #[test]
    fn test_site_context_with_port() {
        let base_path = PathBuf::from("/sites");
        let context = SiteContext::new("example.com:8080".to_string(), &base_path);
        assert_eq!(context.domain, "example.com:8080");
        // Should use base domain for directory
        assert!(context.sanitized_domain.starts_with("example-com-"));
        assert!(context.site_directory.starts_with("/sites"));
        let site_dir = &context.site_directory;
        assert!(site_dir
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("example-com-"));
    }

    #[test]
    fn test_database_path() {
        let base_path = PathBuf::from("/sites");
        let context = SiteContext::new("example.com".to_string(), &base_path);
        let db_path = context.database_path();
        assert!(db_path.starts_with("sqlite:/sites/example-com-"));
        assert!(db_path.ends_with("/site.db"));
    }

    #[test]
    fn test_database_path_subdomain() {
        let base_path = PathBuf::from("/sites");
        let context = SiteContext::new("sub.example.com".to_string(), &base_path);
        let db_path = context.database_path();
        // Should use base domain directory
        assert!(db_path.starts_with("sqlite:/sites/example-com-"));
        assert!(db_path.ends_with("/site.db"));
    }

    #[test]
    fn test_templates_path() {
        let base_path = PathBuf::from("/sites");
        let context = SiteContext::new("example.com".to_string(), &base_path);
        let templates_path = context.templates_path();
        assert!(templates_path
            .to_str()
            .unwrap()
            .contains("/sites/example-com-"));
        assert!(templates_path.to_str().unwrap().ends_with("/templates"));
    }

    #[test]
    fn test_uploads_path() {
        let base_path = PathBuf::from("/sites");
        let context = SiteContext::new("example.com".to_string(), &base_path);
        let uploads_path = context.uploads_path();
        assert!(uploads_path
            .to_str()
            .unwrap()
            .contains("/sites/example-com-"));
        assert!(uploads_path.to_str().unwrap().ends_with("/uploads"));
    }

}
