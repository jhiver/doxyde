use anyhow::Result;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    Form, Json,
};
use axum_extra::{
    headers::{authorization::Basic, Authorization},
    TypedHeader,
};
use serde::{Deserialize, Serialize};

use crate::{error::AppError, state::AppState};

use super::{
    client_registration::verify_client_secret,
    errors::OAuthErrorResponse,
    models::{hash_token, verify_pkce, AccessToken, OAuthError, RefreshToken},
};

/// Token request
#[derive(Debug, Deserialize)]
pub struct TokenRequest {
    pub grant_type: String,
    pub code: Option<String>,
    pub redirect_uri: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub code_verifier: Option<String>,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}

/// Token response
#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub refresh_token: Option<String>,
    pub scope: Option<String>,
}

/// OAuth2 token endpoint
pub async fn token_handler(
    State(state): State<AppState>,
    auth_header: Option<TypedHeader<Authorization<Basic>>>,
    Form(request): Form<TokenRequest>,
) -> Result<Response, AppError> {
    match request.grant_type.as_str() {
        "authorization_code" => {
            match handle_authorization_code_grant_inner(state, auth_header, request).await {
                Ok(response) => Ok((StatusCode::OK, Json(response)).into_response()),
                Err(oauth_err) => Ok(oauth_err.into_response()),
            }
        }
        "refresh_token" => {
            match handle_refresh_token_grant_inner(state, auth_header, request).await {
                Ok(response) => Ok((StatusCode::OK, Json(response)).into_response()),
                Err(oauth_err) => Ok(oauth_err.into_response()),
            }
        }
        _ => Ok(
            OAuthErrorResponse(OAuthError::unsupported_grant_type(&format!(
                "Grant type '{}' is not supported",
                request.grant_type
            )))
            .into_response(),
        ),
    }
}

async fn handle_authorization_code_grant_inner(
    state: AppState,
    auth_header: Option<TypedHeader<Authorization<Basic>>>,
    request: TokenRequest,
) -> Result<TokenResponse, OAuthErrorResponse> {
    // Extract client credentials first (before moving from request)
    let (client_id, client_secret) = extract_client_credentials(auth_header, &request)?;

    // Extract authorization code
    let code = request
        .code
        .ok_or_else(|| OAuthErrorResponse(OAuthError::invalid_request("code is required")))?;

    // Extract redirect_uri
    let redirect_uri = request.redirect_uri.ok_or_else(|| {
        OAuthErrorResponse(OAuthError::invalid_request("redirect_uri is required"))
    })?;

    // Extract code_verifier if present (needed later)
    let code_verifier = request.code_verifier;

    // Get authorization code from database
    let auth_repo = doxyde_db::repositories::AuthorizationCodeRepository::new(state.db.clone());
    let db_auth_code = auth_repo
        .find_by_code(&code)
        .await
        .map_err(|_| OAuthErrorResponse(OAuthError::invalid_grant("Invalid authorization code")))?
        .ok_or_else(|| {
            OAuthErrorResponse(OAuthError::invalid_grant("Invalid authorization code"))
        })?;

    // Convert to web model
    let mut auth_code = super::models::AuthorizationCode {
        code: db_auth_code.code,
        client_id: db_auth_code.client_id,
        user_id: db_auth_code.user_id,
        mcp_token_id: db_auth_code.mcp_token_id,
        redirect_uri: db_auth_code.redirect_uri,
        scope: db_auth_code.scope,
        code_challenge: db_auth_code.code_challenge,
        code_challenge_method: db_auth_code.code_challenge_method,
        expires_at: db_auth_code.expires_at,
        used_at: db_auth_code.used_at,
        created_at: db_auth_code.created_at,
    };

    // Check if code is valid
    if !auth_code.is_valid() {
        return Err(OAuthErrorResponse(OAuthError::invalid_grant(
            "Authorization code has expired or already been used",
        )));
    }

    // Validate redirect_uri matches
    if auth_code.redirect_uri != redirect_uri {
        return Err(OAuthErrorResponse(OAuthError::invalid_grant(
            "redirect_uri does not match",
        )));
    }

    // Validate client
    if auth_code.client_id != client_id {
        return Err(OAuthErrorResponse(OAuthError::invalid_client(
            "client_id does not match authorization code",
        )));
    }

    // Get client from database
    let oauth_repo = doxyde_db::repositories::OAuthClientRepository::new(state.db.clone());
    let client = oauth_repo
        .find_by_id(&client_id)
        .await
        .map_err(|_| OAuthErrorResponse(OAuthError::invalid_client("Unknown client")))?
        .ok_or_else(|| OAuthErrorResponse(OAuthError::invalid_client("Unknown client")))?;

    // Verify client secret if client is confidential
    if let Some(client_secret_hash) = &client.client_secret_hash {
        let secret = client_secret.ok_or_else(|| {
            OAuthErrorResponse(OAuthError::invalid_client("client_secret is required"))
        })?;

        if !verify_client_secret(&secret, client_secret_hash) {
            return Err(OAuthErrorResponse(OAuthError::invalid_client(
                "Invalid client credentials",
            )));
        }
    }

    // Verify PKCE if present
    if let Some(code_challenge) = &auth_code.code_challenge {
        let verifier = code_verifier.ok_or_else(|| {
            OAuthErrorResponse(OAuthError::invalid_request(
                "code_verifier is required for PKCE",
            ))
        })?;

        let method = auth_code.code_challenge_method.as_deref().unwrap_or("S256");
        if !verify_pkce(&verifier, code_challenge, method) {
            return Err(OAuthErrorResponse(OAuthError::invalid_grant(
                "Invalid code_verifier",
            )));
        }
    }

    // Mark code as used
    auth_code.mark_used();
    auth_repo
        .mark_used(&auth_code.code)
        .await
        .map_err(|_| OAuthErrorResponse(OAuthError::server_error("Failed to mark code as used")))?;

    // Create access token
    let access_token = AccessToken::new(
        auth_code.client_id.clone(),
        auth_code.user_id,
        auth_code.mcp_token_id.clone(),
        auth_code.scope.clone(),
    );

    // Create refresh token
    let refresh_token = RefreshToken::new(
        auth_code.client_id,
        auth_code.user_id,
        auth_code.mcp_token_id,
        auth_code.scope.clone(),
    );

    // Convert to DB models
    let db_access_token = doxyde_db::repositories::AccessToken {
        token: access_token.token.clone(),
        token_hash: access_token.token_hash,
        client_id: access_token.client_id,
        user_id: access_token.user_id,
        mcp_token_id: access_token.mcp_token_id,
        scope: access_token.scope.clone(),
        expires_at: access_token.expires_at,
        created_at: access_token.created_at,
    };

    let db_refresh_token = doxyde_db::repositories::RefreshToken {
        token: refresh_token.token.clone(),
        token_hash: refresh_token.token_hash,
        client_id: refresh_token.client_id,
        user_id: refresh_token.user_id,
        mcp_token_id: refresh_token.mcp_token_id,
        scope: refresh_token.scope.clone(),
        expires_at: refresh_token.expires_at,
        used_at: None,
        created_at: refresh_token.created_at,
    };

    // Save tokens
    let token_repo = doxyde_db::repositories::AccessTokenRepository::new(state.db.clone());
    token_repo.create(&db_access_token).await.map_err(|_| {
        OAuthErrorResponse(OAuthError::server_error("Failed to create access token"))
    })?;

    let refresh_repo = doxyde_db::repositories::RefreshTokenRepository::new(state.db.clone());
    refresh_repo.create(&db_refresh_token).await.map_err(|_| {
        OAuthErrorResponse(OAuthError::server_error("Failed to create refresh token"))
    })?;

    // Build response
    Ok(TokenResponse {
        access_token: access_token.token,
        token_type: "Bearer".to_string(),
        expires_in: 3600, // 1 hour
        refresh_token: Some(refresh_token.token),
        scope: access_token.scope,
    })
}

async fn handle_refresh_token_grant_inner(
    state: AppState,
    auth_header: Option<TypedHeader<Authorization<Basic>>>,
    request: TokenRequest,
) -> Result<TokenResponse, OAuthErrorResponse> {
    // Extract client credentials first (before moving from request)
    let (client_id, client_secret) = extract_client_credentials(auth_header, &request)?;

    // Extract refresh token
    let refresh_token = request.refresh_token.ok_or_else(|| {
        OAuthErrorResponse(OAuthError::invalid_request("refresh_token is required"))
    })?;

    // Extract scope if present (needed later)
    let requested_scope = request.scope;

    // Get refresh token from database
    let refresh_repo = doxyde_db::repositories::RefreshTokenRepository::new(state.db.clone());
    let token_hash = hash_token(&refresh_token);
    let db_refresh_token = refresh_repo
        .find_by_hash(&token_hash)
        .await
        .map_err(|_| OAuthErrorResponse(OAuthError::invalid_grant("Invalid refresh token")))?
        .ok_or_else(|| OAuthErrorResponse(OAuthError::invalid_grant("Invalid refresh token")))?;

    // Convert to web model
    let mut stored_token = super::models::RefreshToken {
        token: db_refresh_token.token,
        token_hash: db_refresh_token.token_hash,
        client_id: db_refresh_token.client_id,
        user_id: db_refresh_token.user_id,
        mcp_token_id: db_refresh_token.mcp_token_id,
        scope: db_refresh_token.scope,
        expires_at: db_refresh_token.expires_at,
        used_at: db_refresh_token.used_at,
        created_at: db_refresh_token.created_at,
    };

    // Check if token is valid
    if !stored_token.is_valid() {
        return Err(OAuthErrorResponse(OAuthError::invalid_grant(
            "Refresh token has expired or already been used",
        )));
    }

    // Validate client
    if stored_token.client_id != client_id {
        return Err(OAuthErrorResponse(OAuthError::invalid_client(
            "client_id does not match refresh token",
        )));
    }

    // Get client from database
    let oauth_repo = doxyde_db::repositories::OAuthClientRepository::new(state.db.clone());
    let client = oauth_repo
        .find_by_id(&client_id)
        .await
        .map_err(|_| OAuthErrorResponse(OAuthError::invalid_client("Unknown client")))?
        .ok_or_else(|| OAuthErrorResponse(OAuthError::invalid_client("Unknown client")))?;

    // Verify client secret if client is confidential
    if let Some(client_secret_hash) = &client.client_secret_hash {
        let secret = client_secret.ok_or_else(|| {
            OAuthErrorResponse(OAuthError::invalid_client("client_secret is required"))
        })?;

        if !verify_client_secret(&secret, client_secret_hash) {
            return Err(OAuthErrorResponse(OAuthError::invalid_client(
                "Invalid client credentials",
            )));
        }
    }

    // Check requested scope
    let scope = if let Some(requested_scope) = requested_scope {
        // Validate requested scope is subset of original scope
        if let Some(original_scope) = &stored_token.scope {
            if !is_scope_subset(&requested_scope, original_scope) {
                return Err(OAuthErrorResponse(OAuthError::invalid_scope(
                    "Requested scope exceeds original grant",
                )));
            }
        }
        Some(requested_scope)
    } else {
        stored_token.scope.clone()
    };

    // Mark old refresh token as used (rotation)
    stored_token.mark_used();
    refresh_repo.mark_used(&token_hash).await.map_err(|_| {
        OAuthErrorResponse(OAuthError::server_error("Failed to mark token as used"))
    })?;

    // Create new access token
    let access_token = AccessToken::new(
        stored_token.client_id.clone(),
        stored_token.user_id,
        stored_token.mcp_token_id.clone(),
        scope.clone(),
    );

    // Create new refresh token (rotation)
    let new_refresh_token = RefreshToken::new(
        stored_token.client_id,
        stored_token.user_id,
        stored_token.mcp_token_id,
        scope.clone(),
    );

    // Convert to DB models
    let db_access_token = doxyde_db::repositories::AccessToken {
        token: access_token.token.clone(),
        token_hash: access_token.token_hash,
        client_id: access_token.client_id,
        user_id: access_token.user_id,
        mcp_token_id: access_token.mcp_token_id,
        scope: access_token.scope.clone(),
        expires_at: access_token.expires_at,
        created_at: access_token.created_at,
    };

    let db_new_refresh_token = doxyde_db::repositories::RefreshToken {
        token: new_refresh_token.token.clone(),
        token_hash: new_refresh_token.token_hash,
        client_id: new_refresh_token.client_id,
        user_id: new_refresh_token.user_id,
        mcp_token_id: new_refresh_token.mcp_token_id,
        scope: new_refresh_token.scope.clone(),
        expires_at: new_refresh_token.expires_at,
        used_at: None,
        created_at: new_refresh_token.created_at,
    };

    // Save tokens
    let token_repo = doxyde_db::repositories::AccessTokenRepository::new(state.db.clone());
    token_repo.create(&db_access_token).await.map_err(|_| {
        OAuthErrorResponse(OAuthError::server_error("Failed to create access token"))
    })?;
    refresh_repo
        .create(&db_new_refresh_token)
        .await
        .map_err(|_| {
            OAuthErrorResponse(OAuthError::server_error("Failed to create refresh token"))
        })?;

    // Build response
    Ok(TokenResponse {
        access_token: access_token.token,
        token_type: "Bearer".to_string(),
        expires_in: 3600, // 1 hour
        refresh_token: Some(new_refresh_token.token),
        scope,
    })
}

/// Extract client credentials from Basic auth header or request body
fn extract_client_credentials(
    auth_header: Option<TypedHeader<Authorization<Basic>>>,
    request: &TokenRequest,
) -> Result<(String, Option<String>), OAuthErrorResponse> {
    if let Some(TypedHeader(auth)) = auth_header {
        // Client credentials in Authorization header
        Ok((
            auth.username().to_string(),
            Some(auth.password().to_string()),
        ))
    } else if let (Some(client_id), client_secret) = (&request.client_id, &request.client_secret) {
        // Client credentials in request body
        Ok((client_id.clone(), client_secret.clone()))
    } else {
        Err(OAuthErrorResponse(OAuthError::invalid_client(
            "Client authentication required",
        )))
    }
}

/// Check if requested scope is a subset of granted scope
fn is_scope_subset(requested: &str, granted: &str) -> bool {
    let requested_scopes: std::collections::HashSet<&str> = requested.split_whitespace().collect();
    let granted_scopes: std::collections::HashSet<&str> = granted.split_whitespace().collect();

    requested_scopes.is_subset(&granted_scopes)
}

/// Token revocation endpoint
pub async fn revoke_handler(
    State(state): State<AppState>,
    auth_header: Option<TypedHeader<Authorization<Basic>>>,
    Form(request): Form<RevokeRequest>,
) -> Result<Response, AppError> {
    match revoke_handler_inner(state, auth_header, request).await {
        Ok(_) => Ok(StatusCode::OK.into_response()),
        Err(oauth_err) => Ok(oauth_err.into_response()),
    }
}

async fn revoke_handler_inner(
    state: AppState,
    auth_header: Option<TypedHeader<Authorization<Basic>>>,
    request: RevokeRequest,
) -> Result<(), OAuthErrorResponse> {
    // Extract client credentials
    let (client_id, client_secret) = extract_client_credentials_for_revoke(auth_header, &request)?;

    // Get client from database
    let oauth_repo = doxyde_db::repositories::OAuthClientRepository::new(state.db.clone());
    let client = oauth_repo
        .find_by_id(&client_id)
        .await
        .map_err(|_| OAuthErrorResponse(OAuthError::invalid_client("Unknown client")))?
        .ok_or_else(|| OAuthErrorResponse(OAuthError::invalid_client("Unknown client")))?;

    // Verify client secret if client is confidential
    if let Some(client_secret_hash) = &client.client_secret_hash {
        let secret = client_secret.ok_or_else(|| {
            OAuthErrorResponse(OAuthError::invalid_client("client_secret is required"))
        })?;

        if !verify_client_secret(&secret, client_secret_hash) {
            return Err(OAuthErrorResponse(OAuthError::invalid_client(
                "Invalid client credentials",
            )));
        }
    }

    // Revoke token based on type hint
    let token_hash = hash_token(&request.token);

    match request.token_type_hint.as_deref() {
        Some("access_token") | None => {
            // Try to revoke as access token first
            let token_repo = doxyde_db::repositories::AccessTokenRepository::new(state.db.clone());
            if token_repo.delete_by_hash(&token_hash).await.is_ok() {
                return Ok(());
            }

            // If not found as access token and no hint, try refresh token
            if request.token_type_hint.is_none() {
                let refresh_repo =
                    doxyde_db::repositories::RefreshTokenRepository::new(state.db.clone());
                let _ = refresh_repo.mark_used(&token_hash).await;
            }
        }
        Some("refresh_token") => {
            let refresh_repo =
                doxyde_db::repositories::RefreshTokenRepository::new(state.db.clone());
            let _ = refresh_repo.mark_used(&token_hash).await;
        }
        _ => {
            // Unknown token type hint, ignore
        }
    }

    // Always return success (per RFC 7009)
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct RevokeRequest {
    pub token: String,
    pub token_type_hint: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
}

fn extract_client_credentials_for_revoke(
    auth_header: Option<TypedHeader<Authorization<Basic>>>,
    request: &RevokeRequest,
) -> Result<(String, Option<String>), OAuthErrorResponse> {
    if let Some(TypedHeader(auth)) = auth_header {
        Ok((
            auth.username().to_string(),
            Some(auth.password().to_string()),
        ))
    } else if let (Some(client_id), client_secret) = (&request.client_id, &request.client_secret) {
        Ok((client_id.clone(), client_secret.clone()))
    } else {
        Err(OAuthErrorResponse(OAuthError::invalid_client(
            "Client authentication required",
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_subset() {
        assert!(is_scope_subset("mcp:read", "mcp:read mcp:write"));
        assert!(is_scope_subset("mcp:read mcp:write", "mcp:read mcp:write"));
        assert!(!is_scope_subset("mcp:read mcp:write", "mcp:read"));
        assert!(!is_scope_subset("mcp:admin", "mcp:read mcp:write"));
    }
}
