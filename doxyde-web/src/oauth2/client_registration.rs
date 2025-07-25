use anyhow::Result;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};

use crate::{error::AppError, state::AppState};

use super::{
    errors::OAuthErrorResponse,
    models::{OAuthClient, OAuthError},
};

/// Dynamic Client Registration Request (RFC 7591)
#[derive(Debug, Deserialize)]
pub struct ClientRegistrationRequest {
    pub client_name: String,
    pub redirect_uris: Vec<String>,
    #[serde(default)]
    pub grant_types: Vec<String>,
    #[serde(default)]
    pub response_types: Vec<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default = "default_token_auth_method")]
    pub token_endpoint_auth_method: String,
}

fn default_token_auth_method() -> String {
    "client_secret_basic".to_string()
}

/// Dynamic Client Registration Response
#[derive(Debug, Serialize)]
pub struct ClientRegistrationResponse {
    pub client_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_secret: Option<String>,
    pub client_name: String,
    pub redirect_uris: Vec<String>,
    pub grant_types: Vec<String>,
    pub response_types: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    pub token_endpoint_auth_method: String,
    pub client_id_issued_at: i64,
    pub client_secret_expires_at: i64,
}

/// Dynamic Client Registration endpoint
/// https://datatracker.ietf.org/doc/html/rfc7591
pub async fn client_registration_handler(
    State(state): State<AppState>,
    Json(request): Json<ClientRegistrationRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Validate request
    if request.client_name.trim().is_empty() {
        return Ok(
            OAuthErrorResponse(OAuthError::invalid_request("client_name is required"))
                .into_response(),
        );
    }

    if request.redirect_uris.is_empty() {
        return Ok(OAuthErrorResponse(OAuthError::invalid_request(
            "redirect_uris must contain at least one URI",
        ))
        .into_response());
    }

    // Validate redirect URIs
    for uri in &request.redirect_uris {
        if !is_valid_redirect_uri(uri) {
            return Ok(OAuthErrorResponse(OAuthError::invalid_request(&format!(
                "Invalid redirect_uri: {}",
                uri
            )))
            .into_response());
        }
    }

    // Set defaults if not provided
    let grant_types = if request.grant_types.is_empty() {
        vec!["authorization_code".to_string()]
    } else {
        request.grant_types
    };

    let response_types = if request.response_types.is_empty() {
        vec!["code".to_string()]
    } else {
        request.response_types
    };

    // Validate grant types and response types
    for grant_type in &grant_types {
        if !["authorization_code", "refresh_token"].contains(&grant_type.as_str()) {
            return Ok(OAuthErrorResponse(OAuthError::invalid_request(&format!(
                "Unsupported grant_type: {}",
                grant_type
            )))
            .into_response());
        }
    }

    for response_type in &response_types {
        if response_type != "code" {
            return Ok(OAuthErrorResponse(OAuthError::invalid_request(&format!(
                "Unsupported response_type: {}",
                response_type
            )))
            .into_response());
        }
    }

    // Create new client
    let mut client = OAuthClient::new(request.client_name, request.redirect_uris, None);
    client.grant_types = grant_types;
    client.response_types = response_types;
    client.scope = request.scope;
    client.token_endpoint_auth_method = request.token_endpoint_auth_method;

    // Generate client secret for confidential clients
    let client_secret = if client.token_endpoint_auth_method != "none" {
        let secret = super::models::generate_secure_token(32);
        // Hash the secret for storage
        client.client_secret_hash = Some(hash_client_secret(&secret));
        Some(secret)
    } else {
        None
    };

    // Convert to DB model
    let db_client = doxyde_db::repositories::OAuthClient {
        client_id: client.client_id.clone(),
        client_secret_hash: client.client_secret_hash.clone(),
        client_name: client.client_name.clone(),
        redirect_uris: client.redirect_uris.clone(),
        grant_types: client.grant_types.clone(),
        response_types: client.response_types.clone(),
        scope: client.scope.clone(),
        token_endpoint_auth_method: client.token_endpoint_auth_method.clone(),
        created_at: client.created_at,
        created_by_token_id: client.created_by_token_id.clone(),
    };

    // Save client to database
    let oauth_repo = doxyde_db::repositories::OAuthClientRepository::new(state.db.clone());
    oauth_repo.create(&db_client).await?;

    // Build response
    let response = ClientRegistrationResponse {
        client_id: client.client_id,
        client_secret,
        client_name: client.client_name,
        redirect_uris: client.redirect_uris,
        grant_types: client.grant_types,
        response_types: client.response_types,
        scope: client.scope,
        token_endpoint_auth_method: client.token_endpoint_auth_method,
        client_id_issued_at: client.created_at.timestamp(),
        client_secret_expires_at: 0, // Never expires
    };

    Ok((StatusCode::CREATED, Json(response)).into_response())
}

/// Validate redirect URI format
fn is_valid_redirect_uri(uri: &str) -> bool {
    // Allow localhost with any port
    if uri.starts_with("http://localhost:") {
        return true;
    }

    // Allow claude:// scheme
    if uri.starts_with("claude://") {
        return true;
    }

    // Parse as URL and validate
    match url::Url::parse(uri) {
        Ok(url) => {
            // Must have a scheme
            if url.scheme().is_empty() {
                return false;
            }
            // Must not have a fragment
            if url.fragment().is_some() {
                return false;
            }
            // For HTTPS, must have a host
            if url.scheme() == "https" && url.host().is_none() {
                return false;
            }
            true
        }
        Err(_) => false,
    }
}

/// Hash client secret using Argon2
pub fn hash_client_secret(secret: &str) -> String {
    use argon2::{
        password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
        Argon2,
    };

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();

    argon2
        .hash_password(secret.as_bytes(), &salt)
        .unwrap()
        .to_string()
}

/// Verify client secret
pub fn verify_client_secret(secret: &str, hash: &str) -> bool {
    use argon2::{password_hash::PasswordVerifier, Argon2};

    let parsed_hash = match argon2::PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };

    Argon2::default()
        .verify_password(secret.as_bytes(), &parsed_hash)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redirect_uri_validation() {
        assert!(is_valid_redirect_uri("http://localhost:3000/callback"));
        assert!(is_valid_redirect_uri("http://localhost:8080"));
        assert!(is_valid_redirect_uri("https://example.com/callback"));
        assert!(is_valid_redirect_uri("claude://callback"));

        assert!(!is_valid_redirect_uri(""));
        assert!(!is_valid_redirect_uri("not-a-url"));
        assert!(!is_valid_redirect_uri(
            "https://example.com/callback#fragment"
        ));
    }

    #[test]
    fn test_client_secret_hashing() {
        let secret = "test_secret_123";
        let hash = hash_client_secret(secret);

        assert!(verify_client_secret(secret, &hash));
        assert!(!verify_client_secret("wrong_secret", &hash));
    }
}
