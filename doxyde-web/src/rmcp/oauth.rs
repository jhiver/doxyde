// Doxyde - A modern, AI-native CMS built with Rust
// Copyright (C) 2025 Doxyde Project Contributors
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use anyhow::{Context, Result};
use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use base64::Engine;
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

use crate::{error::AppError, session::get_current_user, AppState};

#[derive(Debug, Serialize)]
pub struct McpToken {
    pub id: i64,
    pub site_id: i64,
    pub name: String,
    pub scopes: Option<String>,
    pub created_by: i64,
    pub expires_at: Option<String>,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateTokenRequest {
    pub name: String,
    pub scopes: Option<String>,
    pub expires_in_days: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct CreateTokenResponse {
    pub token: String,
    pub token_info: McpToken,
}

#[derive(Debug, Deserialize)]
pub struct ListTokensQuery {
    pub site_id: Option<i64>,
}

pub struct TokenInfo {
    pub id: i64,
    pub site_id: i64,
    pub scopes: Option<String>,
}

pub async fn validate_token(db: &SqlitePool, token: &str) -> Result<Option<TokenInfo>> {
    // Hash the token
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let token_hash = format!("{:x}", hasher.finalize());

    // Look up token
    let result = sqlx::query!(
        r#"
        SELECT id, site_id, scopes, expires_at
        FROM mcp_tokens
        WHERE token_hash = ?
        "#,
        token_hash
    )
    .fetch_optional(db)
    .await
    .context("Failed to validate token")?;

    if let Some(row) = result {
        // Check expiration
        if let Some(expires_at) = row.expires_at {
            let expiry = chrono::DateTime::parse_from_rfc3339(&expires_at)
                .context("Failed to parse expiry date")?
                .with_timezone(&Utc);
            if Utc::now() > expiry {
                return Ok(None); // Token expired
            }
        }

        // Update last_used_at
        let _ = sqlx::query!(
            r#"
            UPDATE mcp_tokens
            SET last_used_at = datetime('now')
            WHERE id = ?
            "#,
            row.id
        )
        .execute(db)
        .await;

        Ok(Some(TokenInfo {
            id: row.id.unwrap_or(0),
            site_id: row.site_id,
            scopes: row.scopes,
        }))
    } else {
        Ok(None)
    }
}

pub async fn create_token(
    State(state): State<AppState>,
    session: axum_extra::extract::CookieJar,
    Json(request): Json<CreateTokenRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = get_current_user(&state.db, &session)
        .await?
        .ok_or_else(|| AppError::unauthorized("Not logged in"))?;

    // Check if user is admin
    if !user.is_admin {
        return Err(AppError::forbidden("Only admins can create MCP tokens"));
    }

    // Generate random token
    let token_bytes: [u8; 32] = rand::random();
    use base64::Engine;
    let token = base64::engine::general_purpose::STANDARD.encode(&token_bytes);

    // Hash the token for storage
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let token_hash = format!("{:x}", hasher.finalize());

    // Calculate expiry
    let expires_at = request.expires_in_days.map(|days| {
        (Utc::now() + Duration::days(days)).to_rfc3339()
    });

    // Insert token (use site_id 1 for now - TODO: implement proper multi-site support)
    let site_id = 1i64;
    let user_id = user.id.unwrap_or(0);
    let result = sqlx::query!(
        r#"
        INSERT INTO mcp_tokens (site_id, token_hash, name, scopes, created_by, expires_at)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
        site_id,
        token_hash,
        request.name,
        request.scopes,
        user_id,
        expires_at
    )
    .execute(&state.db)
    .await
    .context("Failed to create token")?;

    let token_id = result.last_insert_rowid();

    // Fetch the created token
    let token_info = sqlx::query_as!(
        McpToken,
        r#"
        SELECT id as "id: i64", site_id as "site_id: i64", name, scopes, created_by as "created_by: i64",
               expires_at, created_at, last_used_at
        FROM mcp_tokens
        WHERE id = ?
        "#,
        token_id
    )
    .fetch_one(&state.db)
    .await
    .context("Failed to fetch created token")?;

    Ok(Json(CreateTokenResponse {
        token,
        token_info,
    }))
}

pub async fn list_tokens(
    State(state): State<AppState>,
    session: axum_extra::extract::CookieJar,
    Query(query): Query<ListTokensQuery>,
) -> Result<impl IntoResponse, AppError> {
    let user = get_current_user(&state.db, &session)
        .await?
        .ok_or_else(|| AppError::unauthorized("Not logged in"))?;

    // Check if user is admin
    if !user.is_admin {
        return Err(AppError::forbidden("Only admins can list MCP tokens"));
    }

    let site_id = query.site_id.unwrap_or(1); // TODO: implement proper multi-site support

    let tokens = sqlx::query_as!(
        McpToken,
        r#"
        SELECT id as "id: i64", site_id as "site_id: i64", name, scopes, created_by as "created_by: i64",
               expires_at, created_at, last_used_at
        FROM mcp_tokens
        WHERE site_id = ?
        ORDER BY created_at DESC
        "#,
        site_id
    )
    .fetch_all(&state.db)
    .await
    .context("Failed to list tokens")?;

    Ok(Json(tokens))
}

pub async fn revoke_token(
    State(state): State<AppState>,
    session: axum_extra::extract::CookieJar,
    Path(token_id): Path<i64>,
) -> Result<impl IntoResponse, AppError> {
    let user = get_current_user(&state.db, &session)
        .await?
        .ok_or_else(|| AppError::unauthorized("Not logged in"))?;

    // Check if user is admin
    if !user.is_admin {
        return Err(AppError::forbidden("Only admins can revoke MCP tokens"));
    }

    // Delete token (use site_id 1 for now - TODO: implement proper multi-site support)
    let site_id = 1i64;
    let result = sqlx::query!(
        r#"
        DELETE FROM mcp_tokens
        WHERE id = ? AND site_id = ?
        "#,
        token_id,
        site_id
    )
    .execute(&state.db)
    .await
    .context("Failed to revoke token")?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found("Token not found"));
    }

    Ok(StatusCode::NO_CONTENT)
}

// OAuth2 Dynamic Client Registration structures
#[derive(Debug, Deserialize)]
pub struct ClientRegistrationRequest {
    pub client_name: String,
    pub redirect_uris: Vec<String>,
    pub grant_types: Option<Vec<String>>,
    pub response_types: Option<Vec<String>>,
    pub scope: Option<String>,
    pub token_endpoint_auth_method: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ClientRegistrationResponse {
    pub client_id: String,
    pub client_secret: Option<String>,
    pub client_name: String,
    pub redirect_uris: Vec<String>,
    pub grant_types: Vec<String>,
    pub response_types: Vec<String>,
    pub scope: String,
    pub token_endpoint_auth_method: String,
    pub client_id_issued_at: i64,
    pub client_secret_expires_at: i64,
}

// OAuth2 Authorization Request
#[derive(Debug, Deserialize)]
pub struct AuthorizationRequest {
    pub response_type: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: Option<String>,
    pub state: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
}

// OAuth2 Token Request
#[derive(Debug, Deserialize)]
pub struct TokenRequest {
    pub grant_type: String,
    pub code: Option<String>,
    pub redirect_uri: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub code_verifier: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: i64,
    pub scope: String,
    pub refresh_token: Option<String>,
}

fn add_cors_headers(headers: &mut HeaderMap) {
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        "*".parse().unwrap(),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        "GET, POST, OPTIONS".parse().unwrap(),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        "Authorization, Content-Type, MCP-Protocol-Version".parse().unwrap(),
    );
    headers.insert(
        header::ACCESS_CONTROL_MAX_AGE,
        "3600".parse().unwrap(),
    );
}

pub async fn register_client(
    Json(request): Json<ClientRegistrationRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    // Generate client credentials
    let client_id = format!("mcp_{}", uuid::Uuid::new_v4());
    let client_secret = base64::engine::general_purpose::STANDARD.encode(&rand::random::<[u8; 32]>());
    
    // Hash the client secret
    let mut hasher = Sha256::new();
    hasher.update(client_secret.as_bytes());
    let _client_secret_hash = format!("{:x}", hasher.finalize());
    
    // Default values
    let grant_types = request.grant_types.unwrap_or_else(|| vec!["authorization_code".to_string()]);
    let response_types = request.response_types.unwrap_or_else(|| vec!["code".to_string()]);
    let scope = request.scope.unwrap_or_else(|| "mcp:read mcp:write".to_string());
    let token_endpoint_auth_method = request.token_endpoint_auth_method.unwrap_or_else(|| "client_secret_basic".to_string());
    
    // For now, we'll store this in memory or return a mock response
    // In a real implementation, you'd save to the oauth_clients table
    
    let response = ClientRegistrationResponse {
        client_id,
        client_secret: Some(client_secret),
        client_name: request.client_name,
        redirect_uris: request.redirect_uris,
        grant_types,
        response_types,
        scope,
        token_endpoint_auth_method,
        client_id_issued_at: Utc::now().timestamp(),
        client_secret_expires_at: 0, // Never expires
    };
    
    let mut headers = HeaderMap::new();
    add_cors_headers(&mut headers);
    headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
    
    Ok((StatusCode::CREATED, headers, Json(response)))
}

pub async fn authorize(
    Query(_request): Query<AuthorizationRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    // For now, return a simple error response
    // In a real implementation, you'd show a login/consent screen
    let error_response = serde_json::json!({
        "error": "unsupported_response_type",
        "error_description": "Authorization endpoint not yet implemented"
    });
    
    let mut headers = HeaderMap::new();
    add_cors_headers(&mut headers);
    headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
    
    Ok((StatusCode::BAD_REQUEST, headers, Json(error_response)))
}

pub async fn token(
    Json(_request): Json<TokenRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    // For now, return a simple error response
    // In a real implementation, you'd validate the authorization code and issue tokens
    let error_response = serde_json::json!({
        "error": "unsupported_grant_type",
        "error_description": "Token endpoint not yet implemented"
    });
    
    let mut headers = HeaderMap::new();
    add_cors_headers(&mut headers);
    headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
    
    Ok((StatusCode::BAD_REQUEST, headers, Json(error_response)))
}

pub async fn oauth_options() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    add_cors_headers(&mut headers);
    (StatusCode::NO_CONTENT, headers)
}