use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// OAuth2 client for dynamic registration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthClient {
    pub client_id: String,
    pub client_secret_hash: Option<String>,
    pub client_name: String,
    pub redirect_uris: Vec<String>,
    pub grant_types: Vec<String>,
    pub response_types: Vec<String>,
    pub scope: Option<String>,
    pub token_endpoint_auth_method: String,
    pub created_at: DateTime<Utc>,
    pub created_by_token_id: Option<String>,
}

impl OAuthClient {
    pub fn new(
        client_name: String,
        redirect_uris: Vec<String>,
        created_by_token_id: Option<String>,
    ) -> Self {
        Self {
            client_id: uuid::Uuid::new_v4().to_string(),
            client_secret_hash: None,
            client_name,
            redirect_uris,
            grant_types: vec!["authorization_code".to_string(), "refresh_token".to_string()],
            response_types: vec!["code".to_string()],
            scope: Some("mcp:read mcp:write".to_string()),
            token_endpoint_auth_method: "client_secret_basic".to_string(),
            created_at: Utc::now(),
            created_by_token_id,
        }
    }

    pub fn validate_redirect_uri(&self, uri: &str) -> bool {
        self.redirect_uris.iter().any(|allowed| {
            // Exact match
            if allowed == uri {
                return true;
            }
            // Allow localhost with any port if pattern matches
            if allowed.starts_with("http://localhost:") && uri.starts_with("http://localhost:") {
                return true;
            }
            // Allow claude:// scheme
            if allowed == "claude://callback" && uri == "claude://callback" {
                return true;
            }
            false
        })
    }
}

/// OAuth2 authorization code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorizationCode {
    pub code: String,
    pub client_id: String,
    pub user_id: i64,
    pub mcp_token_id: String,
    pub redirect_uri: String,
    pub scope: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl AuthorizationCode {
    pub fn new(
        client_id: String,
        user_id: i64,
        mcp_token_id: String,
        redirect_uri: String,
        scope: Option<String>,
        code_challenge: Option<String>,
        code_challenge_method: Option<String>,
    ) -> Self {
        Self {
            code: generate_secure_token(32),
            client_id,
            user_id,
            mcp_token_id,
            redirect_uri,
            scope,
            code_challenge,
            code_challenge_method,
            expires_at: Utc::now() + Duration::minutes(10),
            used_at: None,
            created_at: Utc::now(),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.used_at.is_none() && Utc::now() < self.expires_at
    }

    pub fn mark_used(&mut self) {
        self.used_at = Some(Utc::now());
    }
}

/// OAuth2 access token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessToken {
    pub token: String,
    pub token_hash: String,
    pub client_id: String,
    pub user_id: i64,
    pub mcp_token_id: String,
    pub scope: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl AccessToken {
    pub fn new(
        client_id: String,
        user_id: i64,
        mcp_token_id: String,
        scope: Option<String>,
    ) -> Self {
        let token = generate_secure_token(64);
        let token_hash = hash_token(&token);

        Self {
            token,
            token_hash,
            client_id,
            user_id,
            mcp_token_id,
            scope,
            expires_at: Utc::now() + Duration::hours(1),
            created_at: Utc::now(),
        }
    }

    pub fn is_valid(&self) -> bool {
        Utc::now() < self.expires_at
    }
}

/// OAuth2 refresh token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshToken {
    pub token: String,
    pub token_hash: String,
    pub client_id: String,
    pub user_id: i64,
    pub mcp_token_id: String,
    pub scope: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl RefreshToken {
    pub fn new(
        client_id: String,
        user_id: i64,
        mcp_token_id: String,
        scope: Option<String>,
    ) -> Self {
        let token = generate_secure_token(64);
        let token_hash = hash_token(&token);

        Self {
            token,
            token_hash,
            client_id,
            user_id,
            mcp_token_id,
            scope,
            expires_at: Utc::now() + Duration::days(30),
            used_at: None,
            created_at: Utc::now(),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.used_at.is_none() && Utc::now() < self.expires_at
    }

    pub fn mark_used(&mut self) {
        self.used_at = Some(Utc::now());
    }
}

/// OAuth2 error response
#[derive(Debug, Serialize, Deserialize)]
pub struct OAuthError {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_uri: Option<String>,
}

impl OAuthError {
    pub fn invalid_request(description: &str) -> Self {
        Self {
            error: "invalid_request".to_string(),
            error_description: Some(description.to_string()),
            error_uri: None,
        }
    }

    pub fn invalid_client(description: &str) -> Self {
        Self {
            error: "invalid_client".to_string(),
            error_description: Some(description.to_string()),
            error_uri: None,
        }
    }

    pub fn invalid_grant(description: &str) -> Self {
        Self {
            error: "invalid_grant".to_string(),
            error_description: Some(description.to_string()),
            error_uri: None,
        }
    }

    pub fn unauthorized_client(description: &str) -> Self {
        Self {
            error: "unauthorized_client".to_string(),
            error_description: Some(description.to_string()),
            error_uri: None,
        }
    }

    pub fn unsupported_grant_type(description: &str) -> Self {
        Self {
            error: "unsupported_grant_type".to_string(),
            error_description: Some(description.to_string()),
            error_uri: None,
        }
    }

    pub fn invalid_scope(description: &str) -> Self {
        Self {
            error: "invalid_scope".to_string(),
            error_description: Some(description.to_string()),
            error_uri: None,
        }
    }

    pub fn server_error(description: &str) -> Self {
        Self {
            error: "server_error".to_string(),
            error_description: Some(description.to_string()),
            error_uri: None,
        }
    }
}

/// Generate a cryptographically secure random token
pub fn generate_secure_token(length: usize) -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    
    (0..length)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Hash a token using SHA256
pub fn hash_token(token: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Verify PKCE code challenge
pub fn verify_pkce(code_verifier: &str, code_challenge: &str, method: &str) -> bool {
    if method != "S256" {
        return false;
    }

    use sha2::{Digest, Sha256};
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
    
    let mut hasher = Sha256::new();
    hasher.update(code_verifier.as_bytes());
    let computed = URL_SAFE_NO_PAD.encode(hasher.finalize());
    
    computed == code_challenge
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oauth_client_creation() {
        let client = OAuthClient::new(
            "Test Client".to_string(),
            vec!["http://localhost:3000/callback".to_string()],
            Some("token123".to_string()),
        );

        assert_eq!(client.client_name, "Test Client");
        assert_eq!(client.grant_types, vec!["authorization_code", "refresh_token"]);
        assert_eq!(client.response_types, vec!["code"]);
    }

    #[test]
    fn test_redirect_uri_validation() {
        let client = OAuthClient::new(
            "Test Client".to_string(),
            vec![
                "http://localhost:3000/callback".to_string(),
                "http://localhost:*".to_string(),
                "claude://callback".to_string(),
            ],
            None,
        );

        assert!(client.validate_redirect_uri("http://localhost:3000/callback"));
        assert!(client.validate_redirect_uri("http://localhost:8080/callback"));
        assert!(client.validate_redirect_uri("claude://callback"));
        assert!(!client.validate_redirect_uri("http://example.com/callback"));
    }

    #[test]
    fn test_authorization_code_validity() {
        let mut code = AuthorizationCode::new(
            "client123".to_string(),
            1,
            "token123".to_string(),
            "http://localhost:3000/callback".to_string(),
            Some("mcp:read".to_string()),
            None,
            None,
        );

        assert!(code.is_valid());
        
        code.mark_used();
        assert!(!code.is_valid());
    }

    #[test]
    fn test_pkce_verification() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        let challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";
        
        assert!(verify_pkce(verifier, challenge, "S256"));
        assert!(!verify_pkce(verifier, "wrong_challenge", "S256"));
    }

    #[test]
    fn test_token_generation() {
        let token1 = generate_secure_token(32);
        let token2 = generate_secure_token(32);
        
        assert_eq!(token1.len(), 32);
        assert_eq!(token2.len(), 32);
        assert_ne!(token1, token2);
    }
}