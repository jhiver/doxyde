use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter,
};
use std::{net::SocketAddr, num::NonZeroU32, sync::Arc};

pub type SharedRateLimiter = Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>;

/// Create a rate limiter for login attempts
pub fn create_login_rate_limiter(max_attempts: u32) -> SharedRateLimiter {
    let quota = match NonZeroU32::new(max_attempts) {
        Some(n) => Quota::per_minute(n),
        None => {
            // If zero is passed, default to 1 to avoid panic
            Quota::per_minute(NonZeroU32::new(1).unwrap())
        }
    };
    Arc::new(RateLimiter::direct(quota))
}

/// Create a rate limiter for API endpoints
pub fn create_api_rate_limiter(max_requests: u32) -> SharedRateLimiter {
    let quota = match NonZeroU32::new(max_requests) {
        Some(n) => Quota::per_minute(n),
        None => {
            // If zero is passed, default to 1 to avoid panic
            Quota::per_minute(NonZeroU32::new(1).unwrap())
        }
    };
    Arc::new(RateLimiter::direct(quota))
}

/// Manual rate limiting middleware for specific endpoints
pub async fn rate_limit_middleware(
    limiter: Arc<RateLimiter<NotKeyed, InMemoryState, DefaultClock>>,
    ConnectInfo(_addr): ConnectInfo<SocketAddr>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Check rate limit
    match limiter.check() {
        Ok(_) => Ok(next.run(request).await),
        Err(_) => Err(StatusCode::TOO_MANY_REQUESTS),
    }
}

/// Rate limiting specifically for login endpoint
pub async fn login_rate_limit_middleware(
    State(limiter): State<SharedRateLimiter>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Only apply to POST requests to /.login
    if request.method() == "POST" && request.uri().path() == "/.login" {
        match limiter.check() {
            Ok(_) => Ok(next.run(request).await),
            Err(_) => {
                tracing::warn!("Rate limit exceeded for login");
                Err(StatusCode::TOO_MANY_REQUESTS)
            }
        }
    } else {
        Ok(next.run(request).await)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_login_rate_limiter() {
        let limiter = create_login_rate_limiter(5);

        // Should allow 5 requests
        for _ in 0..5 {
            assert!(limiter.check().is_ok());
        }

        // 6th request should fail
        assert!(limiter.check().is_err());
    }

    #[test]
    fn test_create_api_rate_limiter() {
        let limiter = create_api_rate_limiter(60);

        // Should allow many requests
        for _ in 0..60 {
            assert!(limiter.check().is_ok());
        }

        // 61st request should fail
        assert!(limiter.check().is_err());
    }

    #[test]
    fn test_create_login_rate_limiter_with_zero() {
        // Should default to 1 when zero is passed
        let limiter = create_login_rate_limiter(0);

        // Should allow 1 request
        assert!(limiter.check().is_ok());

        // 2nd request should fail
        assert!(limiter.check().is_err());
    }

    #[test]
    fn test_create_api_rate_limiter_with_zero() {
        // Should default to 1 when zero is passed
        let limiter = create_api_rate_limiter(0);

        // Should allow 1 request
        assert!(limiter.check().is_ok());

        // 2nd request should fail
        assert!(limiter.check().is_err());
    }

    #[test]
    fn test_create_login_rate_limiter_custom_value() {
        let limiter = create_login_rate_limiter(3);

        // Should allow 3 requests
        for _ in 0..3 {
            assert!(limiter.check().is_ok());
        }

        // 4th request should fail
        assert!(limiter.check().is_err());
    }

    #[test]
    fn test_create_api_rate_limiter_custom_value() {
        let limiter = create_api_rate_limiter(10);

        // Should allow 10 requests
        for _ in 0..10 {
            assert!(limiter.check().is_ok());
        }

        // 11th request should fail
        assert!(limiter.check().is_err());
    }

    #[test]
    fn test_rate_limiter_uses_configuration_values() {
        // Test with custom configuration values
        let custom_login_limit = 3;
        let custom_api_limit = 10;

        let login_limiter = create_login_rate_limiter(custom_login_limit);
        let api_limiter = create_api_rate_limiter(custom_api_limit);

        // Login limiter should allow exactly 3 requests
        for _ in 0..custom_login_limit {
            assert!(login_limiter.check().is_ok());
        }
        assert!(login_limiter.check().is_err());

        // API limiter should allow exactly 10 requests
        for _ in 0..custom_api_limit {
            assert!(api_limiter.check().is_ok());
        }
        assert!(api_limiter.check().is_err());
    }
}
