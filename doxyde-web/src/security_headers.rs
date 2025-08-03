use crate::configuration::HeadersConfig;
use axum::{
    body::Body,
    http::{header, Request, Response, StatusCode},
    middleware::Next,
};

pub fn create_security_headers_middleware(
    config: HeadersConfig,
) -> impl Fn(
    Request<Body>,
    Next,
) -> std::pin::Pin<
    Box<dyn std::future::Future<Output = Result<Response<Body>, StatusCode>> + Send>,
> + Clone {
    move |request: Request<Body>, next: Next| {
        let config = config.clone();
        Box::pin(async move {
            let mut response = next.run(request).await;
            let headers = response.headers_mut();

            // Add CSP header if enabled
            if config.enable_csp {
                if let Some(csp_content) = &config.csp_content {
                    headers.insert(
                        header::CONTENT_SECURITY_POLICY,
                        csp_content
                            .parse()
                            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
                    );
                }
            }

            // Add HSTS header if enabled
            if config.enable_hsts {
                if let Some(hsts_content) = &config.hsts_content {
                    headers.insert(
                        header::STRICT_TRANSPORT_SECURITY,
                        hsts_content
                            .parse()
                            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
                    );
                }
            }

            // Add X-Frame-Options header if enabled
            if config.enable_frame_options {
                if let Some(frame_options_content) = &config.frame_options_content {
                    headers.insert(
                        header::X_FRAME_OPTIONS,
                        frame_options_content
                            .parse()
                            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
                    );
                }
            }

            // Add X-Content-Type-Options header if enabled
            if config.enable_content_type_options {
                headers.insert(
                    header::X_CONTENT_TYPE_OPTIONS,
                    "nosniff"
                        .parse()
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
                );
            }

            // Add Referrer-Policy header if configured
            if let Some(referrer_policy) = &config.referrer_policy {
                headers.insert(
                    header::REFERRER_POLICY,
                    referrer_policy
                        .parse()
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
                );
            }

            // Add Permissions-Policy header if configured
            if let Some(permissions_policy) = &config.permissions_policy {
                headers.insert(
                    "Permissions-Policy",
                    permissions_policy
                        .parse()
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
                );
            }

            Ok(response)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::configuration::defaults;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        middleware::{self},
        response::IntoResponse,
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    async fn dummy_handler() -> impl IntoResponse {
        "Hello, World!"
    }

    fn create_default_config() -> HeadersConfig {
        HeadersConfig {
            enable_hsts: true,
            enable_csp: true,
            enable_frame_options: true,
            enable_content_type_options: true,
            csp_content: defaults::default_csp_content(),
            hsts_content: defaults::default_hsts_content(),
            frame_options_content: defaults::default_frame_options_content(),
            referrer_policy: defaults::default_referrer_policy(),
            permissions_policy: defaults::default_permissions_policy(),
        }
    }

    #[tokio::test]
    async fn test_security_headers_added() {
        let config = create_default_config();
        let middleware = create_security_headers_middleware(config);
        let app = Router::new()
            .route("/", get(dummy_handler))
            .layer(middleware::from_fn(middleware));

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let headers = response.headers();

        assert!(headers.contains_key(header::CONTENT_SECURITY_POLICY));
        assert!(headers.contains_key(header::STRICT_TRANSPORT_SECURITY));
        assert!(headers.contains_key(header::X_FRAME_OPTIONS));
        assert!(headers.contains_key(header::X_CONTENT_TYPE_OPTIONS));
        assert!(headers.contains_key(header::REFERRER_POLICY));
        assert!(headers.contains_key("Permissions-Policy"));
    }

    #[tokio::test]
    async fn test_csp_header_content() {
        let config = create_default_config();
        let middleware = create_security_headers_middleware(config);
        let app = Router::new()
            .route("/", get(dummy_handler))
            .layer(middleware::from_fn(middleware));

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let csp = response
            .headers()
            .get(header::CONTENT_SECURITY_POLICY)
            .unwrap();
        let csp_str = csp.to_str().unwrap();

        assert!(csp_str.contains("default-src 'self'"));
        assert!(csp_str.contains("frame-ancestors 'none'"));
        assert!(csp_str.contains("base-uri 'self'"));
    }

    #[tokio::test]
    async fn test_hsts_header_content() {
        let config = create_default_config();
        let middleware = create_security_headers_middleware(config);
        let app = Router::new()
            .route("/", get(dummy_handler))
            .layer(middleware::from_fn(middleware));

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let hsts = response
            .headers()
            .get(header::STRICT_TRANSPORT_SECURITY)
            .unwrap();
        assert_eq!(
            hsts.to_str().unwrap(),
            "max-age=31536000; includeSubDomains"
        );
    }

    #[tokio::test]
    async fn test_x_frame_options_header() {
        let config = create_default_config();
        let middleware = create_security_headers_middleware(config);
        let app = Router::new()
            .route("/", get(dummy_handler))
            .layer(middleware::from_fn(middleware));

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let x_frame = response.headers().get(header::X_FRAME_OPTIONS).unwrap();
        assert_eq!(x_frame.to_str().unwrap(), "DENY");
    }

    #[tokio::test]
    async fn test_headers_can_be_disabled() {
        let config = HeadersConfig {
            enable_hsts: false,
            enable_csp: false,
            enable_frame_options: false,
            enable_content_type_options: false,
            csp_content: None,
            hsts_content: None,
            frame_options_content: None,
            referrer_policy: None,
            permissions_policy: None,
        };
        let middleware = create_security_headers_middleware(config);
        let app = Router::new()
            .route("/", get(dummy_handler))
            .layer(middleware::from_fn(middleware));

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let headers = response.headers();

        assert!(!headers.contains_key(header::CONTENT_SECURITY_POLICY));
        assert!(!headers.contains_key(header::STRICT_TRANSPORT_SECURITY));
        assert!(!headers.contains_key(header::X_FRAME_OPTIONS));
        assert!(!headers.contains_key(header::X_CONTENT_TYPE_OPTIONS));
        assert!(!headers.contains_key(header::REFERRER_POLICY));
        assert!(!headers.contains_key("Permissions-Policy"));
    }

    #[tokio::test]
    async fn test_custom_header_content() {
        let config = HeadersConfig {
            enable_hsts: true,
            enable_csp: true,
            enable_frame_options: true,
            enable_content_type_options: true,
            csp_content: Some("default-src 'none'; script-src 'self'".to_string()),
            hsts_content: Some("max-age=3600".to_string()),
            frame_options_content: Some("SAMEORIGIN".to_string()),
            referrer_policy: Some("no-referrer".to_string()),
            permissions_policy: Some("camera=(), microphone=()".to_string()),
        };
        let middleware = create_security_headers_middleware(config);
        let app = Router::new()
            .route("/", get(dummy_handler))
            .layer(middleware::from_fn(middleware));

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let headers = response.headers();

        let csp = headers.get(header::CONTENT_SECURITY_POLICY).unwrap();
        assert_eq!(
            csp.to_str().unwrap(),
            "default-src 'none'; script-src 'self'"
        );

        let hsts = headers.get(header::STRICT_TRANSPORT_SECURITY).unwrap();
        assert_eq!(hsts.to_str().unwrap(), "max-age=3600");

        let frame_options = headers.get(header::X_FRAME_OPTIONS).unwrap();
        assert_eq!(frame_options.to_str().unwrap(), "SAMEORIGIN");

        let referrer_policy = headers.get(header::REFERRER_POLICY).unwrap();
        assert_eq!(referrer_policy.to_str().unwrap(), "no-referrer");

        let permissions_policy = headers.get("Permissions-Policy").unwrap();
        assert_eq!(
            permissions_policy.to_str().unwrap(),
            "camera=(), microphone=()"
        );
    }
}
