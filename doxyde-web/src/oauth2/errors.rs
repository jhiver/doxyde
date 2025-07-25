use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use super::models::OAuthError;

/// OAuth2 error response wrapper
pub struct OAuthErrorResponse(pub OAuthError);

impl IntoResponse for OAuthErrorResponse {
    fn into_response(self) -> Response {
        let status = match self.0.error.as_str() {
            "invalid_request" => StatusCode::BAD_REQUEST,
            "invalid_client" => StatusCode::UNAUTHORIZED,
            "invalid_grant" => StatusCode::BAD_REQUEST,
            "unauthorized_client" => StatusCode::FORBIDDEN,
            "unsupported_grant_type" => StatusCode::BAD_REQUEST,
            "invalid_scope" => StatusCode::BAD_REQUEST,
            _ => StatusCode::BAD_REQUEST,
        };

        let mut response = (status, Json(self.0)).into_response();

        // Add WWW-Authenticate header for 401 responses
        if status == StatusCode::UNAUTHORIZED {
            response.headers_mut().insert(
                "WWW-Authenticate",
                "Bearer error=\"invalid_token\"".parse().unwrap(),
            );
        }

        response
    }
}

/// Authorization error response (for redirect)
#[derive(Debug)]
pub struct AuthorizationError {
    pub error: String,
    pub error_description: Option<String>,
    pub state: Option<String>,
}

impl AuthorizationError {
    pub fn invalid_request(description: &str, state: Option<String>) -> Self {
        Self {
            error: "invalid_request".to_string(),
            error_description: Some(description.to_string()),
            state,
        }
    }

    pub fn unauthorized_client(description: &str, state: Option<String>) -> Self {
        Self {
            error: "unauthorized_client".to_string(),
            error_description: Some(description.to_string()),
            state,
        }
    }

    pub fn access_denied(description: &str, state: Option<String>) -> Self {
        Self {
            error: "access_denied".to_string(),
            error_description: Some(description.to_string()),
            state,
        }
    }

    pub fn unsupported_response_type(description: &str, state: Option<String>) -> Self {
        Self {
            error: "unsupported_response_type".to_string(),
            error_description: Some(description.to_string()),
            state,
        }
    }

    pub fn invalid_scope(description: &str, state: Option<String>) -> Self {
        Self {
            error: "invalid_scope".to_string(),
            error_description: Some(description.to_string()),
            state,
        }
    }

    pub fn server_error(description: &str, state: Option<String>) -> Self {
        Self {
            error: "server_error".to_string(),
            error_description: Some(description.to_string()),
            state,
        }
    }

    /// Build redirect URL with error parameters
    pub fn to_redirect_url(&self, redirect_uri: &str) -> String {
        let mut url = url::Url::parse(redirect_uri)
            .unwrap_or_else(|_| url::Url::parse("http://localhost:3000/error").unwrap());

        url.query_pairs_mut().append_pair("error", &self.error);

        if let Some(desc) = &self.error_description {
            url.query_pairs_mut().append_pair("error_description", desc);
        }

        if let Some(state) = &self.state {
            url.query_pairs_mut().append_pair("state", state);
        }

        url.to_string()
    }
}

/// Bearer token error response for resource server
pub struct BearerError {
    pub error: &'static str,
    pub error_description: &'static str,
}

impl BearerError {
    pub fn invalid_token() -> Self {
        Self {
            error: "invalid_token",
            error_description:
                "The access token provided is expired, revoked, malformed, or invalid",
        }
    }

    pub fn insufficient_scope() -> Self {
        Self {
            error: "insufficient_scope",
            error_description:
                "The request requires higher privileges than provided by the access token",
        }
    }
}

impl IntoResponse for BearerError {
    fn into_response(self) -> Response {
        let mut response = (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": self.error,
                "error_description": self.error_description
            })),
        )
            .into_response();

        response.headers_mut().insert(
            "WWW-Authenticate",
            format!(
                r#"Bearer error="{}", error_description="{}""#,
                self.error, self.error_description
            )
            .parse()
            .unwrap(),
        );

        response
    }
}

#[cfg(test)]
#[path = "errors_test.rs"]
mod tests;
