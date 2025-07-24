use axum::{
    body::Body,
    http::{header, Request, Response, StatusCode},
    middleware::Next,
};

pub async fn security_headers_middleware(
    request: Request<Body>,
    next: Next,
) -> Result<Response<Body>, StatusCode> {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self';"
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );

    headers.insert(
        header::STRICT_TRANSPORT_SECURITY,
        "max-age=31536000; includeSubDomains"
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );

    headers.insert(
        header::X_FRAME_OPTIONS,
        "DENY"
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );

    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        "nosniff"
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );

    headers.insert(
        header::REFERRER_POLICY,
        "strict-origin-when-cross-origin"
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );

    headers.insert(
        "Permissions-Policy",
        "geolocation=(), camera=(), microphone=()"
            .parse()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
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

    #[tokio::test]
    async fn test_security_headers_added() {
        let app = Router::new()
            .route("/", get(dummy_handler))
            .layer(middleware::from_fn(security_headers_middleware));

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
        let app = Router::new()
            .route("/", get(dummy_handler))
            .layer(middleware::from_fn(security_headers_middleware));

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
        let app = Router::new()
            .route("/", get(dummy_handler))
            .layer(middleware::from_fn(security_headers_middleware));

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
        let app = Router::new()
            .route("/", get(dummy_handler))
            .layer(middleware::from_fn(security_headers_middleware));

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let x_frame = response.headers().get(header::X_FRAME_OPTIONS).unwrap();
        assert_eq!(x_frame.to_str().unwrap(), "DENY");
    }
}
