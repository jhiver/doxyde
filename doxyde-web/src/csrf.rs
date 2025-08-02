use anyhow::Result;
use axum::{
    body::Body,
    extract::{FromRequestParts, State},
    http::{request::Parts, Method, Request, StatusCode},
    middleware::Next,
    response::Response,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{auth::SessionUser, AppState};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsrfToken {
    pub token: String,
}

impl CsrfToken {
    pub fn new(token_length: usize) -> Self {
        let mut bytes = vec![0u8; token_length];
        rand::thread_rng().fill_bytes(&mut bytes);
        Self {
            token: URL_SAFE_NO_PAD.encode(&bytes),
        }
    }

    pub fn verify(&self, provided_token: &str) -> bool {
        self.token == provided_token
    }
}

impl<S> FromRequestParts<S> for CsrfToken
where
    S: Send + Sync,
    Arc<AppState>: FromRequestParts<S>,
    SessionUser: FromRequestParts<S>,
{
    type Rejection = StatusCode;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let session_user = SessionUser::from_request_parts(parts, state)
            .await
            .map_err(|_| StatusCode::UNAUTHORIZED)?;

        let app_state = Arc::<AppState>::from_request_parts(parts, state)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        get_or_create_csrf_token(&app_state, &session_user.session_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    }
}

pub async fn get_or_create_csrf_token(state: &AppState, session_id: &str) -> Result<CsrfToken> {
    let pool = &state.db;

    // Try to get existing token
    let existing = sqlx::query!(
        r#"
        SELECT csrf_token
        FROM sessions
        WHERE id = ?
        "#,
        session_id
    )
    .fetch_optional(pool)
    .await?;

    if let Some(row) = existing {
        if let Some(token) = row.csrf_token {
            return Ok(CsrfToken { token });
        }
    }

    // Create new token using configured length
    let csrf_token = CsrfToken::new(state.config.csrf_token_length);

    sqlx::query!(
        r#"
        UPDATE sessions
        SET csrf_token = ?
        WHERE id = ?
        "#,
        csrf_token.token,
        session_id
    )
    .execute(pool)
    .await?;

    Ok(csrf_token)
}

pub async fn csrf_protection_middleware(
    State(state): State<Arc<AppState>>,
    session_user: Option<SessionUser>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Skip CSRF check for:
    // 1. GET, HEAD, OPTIONS requests (safe methods)
    // 2. Unauthenticated requests (no session to protect)
    let method = request.method();
    if matches!(method, &Method::GET | &Method::HEAD | &Method::OPTIONS) {
        return Ok(next.run(request).await);
    }

    let session_user = match session_user {
        Some(user) => user,
        None => return Ok(next.run(request).await), // No session to protect
    };

    // Get the expected CSRF token from session
    let expected_token = get_or_create_csrf_token(&state, &session_user.session_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Extract provided token from header or form using configured header name
    let provided_token = extract_csrf_token(&request, &state.config.csrf_header_name);

    match provided_token {
        Some(token) if expected_token.verify(&token) => Ok(next.run(request).await),
        _ => Err(StatusCode::FORBIDDEN),
    }
}

fn extract_csrf_token(request: &Request<Body>, header_name: &str) -> Option<String> {
    // First check header
    if let Some(header_value) = request.headers().get(header_name) {
        if let Ok(token) = header_value.to_str() {
            return Some(token.to_string());
        }
    }

    // For form submissions, we'd need to parse the body, but that's complex
    // in middleware. For now, we'll rely on header-based CSRF tokens.
    // Form parsing will be handled in individual handlers.
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csrf_token_generation() {
        let token1 = CsrfToken::new(32);
        let token2 = CsrfToken::new(32);

        // Tokens should be unique
        assert_ne!(token1.token, token2.token);

        // Tokens should have reasonable length (base64 encoded)
        assert!(token1.token.len() > 20);
        assert!(token2.token.len() > 20);
    }

    #[test]
    fn test_csrf_token_verification() {
        let token = CsrfToken::new(32);

        // Should verify correctly
        assert!(token.verify(&token.token));

        // Should not verify incorrect token
        assert!(!token.verify("wrong-token"));
        assert!(!token.verify(""));

        // Should be case sensitive
        let uppercase = token.token.to_uppercase();
        if uppercase != token.token {
            assert!(!token.verify(&uppercase));
        }
    }

    #[test]
    fn test_csrf_token_serialization() {
        let token = CsrfToken::new(32);

        // Should serialize to JSON
        let json = serde_json::to_string(&token).unwrap();

        // Should deserialize back
        let deserialized: CsrfToken = serde_json::from_str(&json).unwrap();

        assert_eq!(token.token, deserialized.token);
    }

    #[test]
    fn test_csrf_token_different_lengths() {
        let token_16 = CsrfToken::new(16);
        let token_32 = CsrfToken::new(32);
        let token_64 = CsrfToken::new(64);

        // Different lengths should produce different token sizes (base64 encoded)
        // 16 bytes -> ~22 chars, 32 bytes -> ~43 chars, 64 bytes -> ~86 chars
        assert!(token_16.token.len() < token_32.token.len());
        assert!(token_32.token.len() < token_64.token.len());

        // All should be valid tokens
        assert!(token_16.verify(&token_16.token));
        assert!(token_32.verify(&token_32.token));
        assert!(token_64.verify(&token_64.token));
    }

    #[test]
    fn test_csrf_token_configurable_length() {
        // Test that different configured lengths work correctly
        let length_8 = 8;
        let length_128 = 128;

        let token_8 = CsrfToken::new(length_8);
        let token_128 = CsrfToken::new(length_128);

        // Base64 encoding means actual string length will be different
        // 8 bytes -> ~11 chars, 128 bytes -> ~171 chars
        assert!(token_8.token.len() < token_128.token.len());

        // Both should be valid regardless of length
        assert!(token_8.verify(&token_8.token));
        assert!(token_128.verify(&token_128.token));

        // Cross-verification should fail
        assert!(!token_8.verify(&token_128.token));
        assert!(!token_128.verify(&token_8.token));
    }
}
