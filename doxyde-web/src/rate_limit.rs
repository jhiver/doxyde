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
pub fn create_login_rate_limiter() -> SharedRateLimiter {
    // NonZeroU32::new(5) is safe because 5 is not zero
    let quota = match NonZeroU32::new(5) {
        Some(n) => Quota::per_minute(n), // 5 attempts per minute
        None => unreachable!("5 is not zero"),
    };
    Arc::new(RateLimiter::direct(quota))
}

/// Create a rate limiter for API endpoints
pub fn create_api_rate_limiter() -> SharedRateLimiter {
    // NonZeroU32::new(60) is safe because 60 is not zero
    let quota = match NonZeroU32::new(60) {
        Some(n) => Quota::per_minute(n), // 60 requests per minute
        None => unreachable!("60 is not zero"),
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
        let limiter = create_login_rate_limiter();

        // Should allow 5 requests
        for _ in 0..5 {
            assert!(limiter.check().is_ok());
        }

        // 6th request should fail
        assert!(limiter.check().is_err());
    }

    #[test]
    fn test_create_api_rate_limiter() {
        let limiter = create_api_rate_limiter();

        // Should allow many requests
        for _ in 0..60 {
            assert!(limiter.check().is_ok());
        }

        // 61st request should fail
        assert!(limiter.check().is_err());
    }
}
