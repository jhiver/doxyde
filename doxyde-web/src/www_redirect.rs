use axum::{
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::Host;

/// Middleware that redirects www.domain.com to domain.com
pub async fn www_redirect_middleware(
    Host(host): Host,
    request: Request,
    next: Next,
) -> Response {
    // Check if the host starts with "www."
    if let Some(domain) = host.strip_prefix("www.") {
        // Build the redirect URL with the same path and query
        let uri = request.uri();
        let path_and_query = uri
            .path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or("/");
        
        // Use HTTPS for production domains, HTTP for localhost
        let scheme = if domain.starts_with("localhost") || domain.starts_with("127.0.0.1") {
            "http"
        } else {
            "https"
        };
        
        let redirect_url = format!("{scheme}://{domain}{path_and_query}");
        
        tracing::info!("Redirecting www.{} to {}", domain, domain);
        return Redirect::permanent(&redirect_url).into_response();
    }
    
    // Not a www domain, continue normally
    next.run(request).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request as HttpRequest, StatusCode},
        middleware::from_fn,
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    async fn test_handler() -> &'static str {
        "Hello from handler"
    }

    #[tokio::test]
    async fn test_www_redirect() {
        let app = Router::new()
            .route("/", get(test_handler))
            .layer(from_fn(www_redirect_middleware));

        // Test www redirect
        let response = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .header("Host", "www.example.com")
                    .uri("/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::PERMANENT_REDIRECT);
        let location = response.headers().get("location").unwrap();
        assert_eq!(location, "https://example.com/");
    }

    #[tokio::test]
    async fn test_www_redirect_with_path() {
        let app = Router::new()
            .route("/test", get(test_handler))
            .layer(from_fn(www_redirect_middleware));

        let response = app
            .oneshot(
                HttpRequest::builder()
                    .header("Host", "www.example.com")
                    .uri("/test?foo=bar")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::PERMANENT_REDIRECT);
        let location = response.headers().get("location").unwrap();
        assert_eq!(location, "https://example.com/test?foo=bar");
    }

    #[tokio::test]
    async fn test_non_www_passthrough() {
        let app = Router::new()
            .route("/", get(test_handler))
            .layer(from_fn(www_redirect_middleware));

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
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(&body[..], b"Hello from handler");
    }

    #[tokio::test]
    async fn test_localhost_redirect_uses_http() {
        let app = Router::new()
            .route("/", get(test_handler))
            .layer(from_fn(www_redirect_middleware));

        let response = app
            .oneshot(
                HttpRequest::builder()
                    .header("Host", "www.localhost:3000")
                    .uri("/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::PERMANENT_REDIRECT);
        let location = response.headers().get("location").unwrap();
        assert_eq!(location, "http://localhost:3000/");
    }

    #[tokio::test]
    async fn test_subdomain_not_affected() {
        let app = Router::new()
            .route("/", get(test_handler))
            .layer(from_fn(www_redirect_middleware));

        // Test that non-www subdomains are not affected
        let response = app
            .oneshot(
                HttpRequest::builder()
                    .header("Host", "api.example.com")
                    .uri("/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}