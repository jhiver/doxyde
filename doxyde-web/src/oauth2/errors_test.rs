#[cfg(test)]
mod tests {
    use super::super::*;
    use axum::response::IntoResponse;
    use axum::http::StatusCode;

    #[test]
    fn test_oauth_error_response_serialization() {
        let error = OAuthErrorResponse(OAuthError::invalid_request(
            "Missing required parameter",
        ));

        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[test]
    fn test_bearer_error_response() {
        let error = BearerError::invalid_token();
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_bearer_error_insufficient_scope() {
        let error = BearerError::insufficient_scope();
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_authorization_error_access_denied() {
        let error = AuthorizationError::access_denied("User denied access", Some("xyz".to_string()));
        
        assert_eq!(error.error, "access_denied");
        assert_eq!(error.error_description, Some("User denied access".to_string()));
        assert_eq!(error.state, Some("xyz".to_string()));
    }

    #[test]
    fn test_authorization_error_invalid_request() {
        let error = AuthorizationError::invalid_request(
            "Missing required parameter",
            Some("abc".to_string()),
        );
        
        assert_eq!(error.error, "invalid_request");
        assert_eq!(error.error_description, Some("Missing required parameter".to_string()));
        assert_eq!(error.state, Some("abc".to_string()));
    }
}