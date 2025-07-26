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
    extract::{Form, Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect},
    Json,
};
use axum_extra::extract::cookie::Cookie;
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
#[derive(Debug, Deserialize, Serialize)]
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
    State(state): State<AppState>,
    Json(request): Json<ClientRegistrationRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    // Generate client credentials
    let client_id = format!("mcp_{}", uuid::Uuid::new_v4());
    let client_secret = base64::engine::general_purpose::STANDARD.encode(&rand::random::<[u8; 32]>());
    
    // Hash the client secret
    let mut hasher = Sha256::new();
    hasher.update(client_secret.as_bytes());
    let client_secret_hash = format!("{:x}", hasher.finalize());
    
    // Default values
    let grant_types = request.grant_types.unwrap_or_else(|| vec!["authorization_code".to_string()]);
    let response_types = request.response_types.unwrap_or_else(|| vec!["code".to_string()]);
    let scope = request.scope.unwrap_or_else(|| "mcp:read mcp:write".to_string());
    let token_endpoint_auth_method = request.token_endpoint_auth_method.unwrap_or_else(|| "client_secret_basic".to_string());
    
    // Store in database
    let redirect_uris_json = serde_json::to_string(&request.redirect_uris).unwrap();
    let grant_types_json = serde_json::to_string(&grant_types).unwrap();
    let response_types_json = serde_json::to_string(&response_types).unwrap();
    
    match sqlx::query!(
        r#"
        INSERT INTO oauth_clients 
        (client_id, client_secret_hash, client_name, redirect_uris, grant_types, response_types, scope, token_endpoint_auth_method)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        client_id,
        client_secret_hash,
        request.client_name,
        redirect_uris_json,
        grant_types_json,
        response_types_json,
        scope,
        token_endpoint_auth_method
    )
    .execute(&state.db)
    .await {
        Ok(_) => {
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
        Err(e) => {
            eprintln!("Failed to register OAuth client: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn authorize(
    State(state): State<AppState>,
    session: axum_extra::extract::CookieJar,
    Query(request): Query<AuthorizationRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Validate client_id exists
    let client = match sqlx::query!(
        r#"
        SELECT client_name, redirect_uris, scope
        FROM oauth_clients
        WHERE client_id = ?
        "#,
        request.client_id
    )
    .fetch_optional(&state.db)
    .await
    .context("Failed to check client")? {
        Some(client) => client,
        None => {
            let error_url = format!("{}?error=invalid_client&error_description=Unknown+client", 
                request.redirect_uri);
            return Ok(Redirect::to(&error_url).into_response());
        }
    };
    
    // Validate redirect_uri
    let redirect_uris: Vec<String> = serde_json::from_str(&client.redirect_uris)
        .context("Failed to parse redirect URIs")?;
    if !redirect_uris.contains(&request.redirect_uri) {
        return Err(AppError::bad_request("Invalid redirect_uri"));
    }
    
    // Check if user is authenticated
    let user = match get_current_user(&state.db, &session).await? {
        Some(user) => user,
        None => {
            // Redirect to login, preserving OAuth parameters
            let oauth_params = serde_urlencoded::to_string(&request)
                .context("Failed to serialize OAuth params")?;
            let login_url = format!("/.login?return_to=/.oauth/authorize?{}", 
                urlencoding::encode(&oauth_params));
            return Ok(Redirect::to(&login_url).into_response());
        }
    };
    
    // Parse requested scopes
    let requested_scopes: Vec<&str> = request.scope
        .as_deref()
        .unwrap_or("mcp:read")
        .split(' ')
        .collect();
    
    // Show consent screen
    let mut context = tera::Context::new();
    context.insert("client_name", &client.client_name);
    context.insert("client_id", &request.client_id);
    context.insert("redirect_uri", &request.redirect_uri);
    context.insert("response_type", &request.response_type);
    context.insert("scope", &request.scope.as_deref().unwrap_or("mcp:read"));
    context.insert("scopes", &requested_scopes);
    context.insert("user", &user);
    
    if let Some(state_param) = &request.state {
        context.insert("state", state_param);
    }
    if let Some(challenge) = &request.code_challenge {
        context.insert("code_challenge", challenge);
        context.insert("code_challenge_method", 
            &request.code_challenge_method.as_deref().unwrap_or("S256"));
    }
    
    // Add CSRF token
    let csrf_token = uuid::Uuid::new_v4().to_string();
    context.insert("csrf_token", &csrf_token);
    
    // Store CSRF token in session for validation
    let session = session.add(Cookie::new("oauth_csrf", csrf_token));
    
    let html = state.templates
        .render("oauth_consent.html", &context)
        .context("Failed to render OAuth consent template")?;
    
    Ok((session, Html(html)).into_response())
}

pub async fn token(
    State(state): State<AppState>,
    Json(request): Json<TokenRequest>,
) -> impl IntoResponse {
    // Only support authorization_code grant type for now
    if request.grant_type != "authorization_code" {
        let error_response = serde_json::json!({
            "error": "unsupported_grant_type",
            "error_description": "Only authorization_code grant type is supported"
        });
        
        let mut headers = HeaderMap::new();
        add_cors_headers(&mut headers);
        headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
        
        return (StatusCode::BAD_REQUEST, headers, Json(error_response)).into_response();
    }
    
    let code = match &request.code {
        Some(code) => code,
        None => {
            let error_response = serde_json::json!({
                "error": "invalid_request",
                "error_description": "Missing authorization code"
            });
            
            let mut headers = HeaderMap::new();
            add_cors_headers(&mut headers);
            headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
            
            return (StatusCode::BAD_REQUEST, headers, Json(error_response)).into_response();
        }
    };
    
    // Retrieve and validate authorization code
    let auth_code = match sqlx::query!(
        r#"
        SELECT client_id, user_id, redirect_uri, scope, code_challenge, code_challenge_method, expires_at, used_at
        FROM oauth_authorization_codes
        WHERE code = ?
        "#,
        code
    )
    .fetch_optional(&state.db)
    .await {
        Ok(Some(row)) => row,
        Ok(None) => {
            let error_response = serde_json::json!({
                "error": "invalid_grant",
                "error_description": "Invalid authorization code"
            });
            
            let mut headers = HeaderMap::new();
            add_cors_headers(&mut headers);
            headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
            
            return (StatusCode::BAD_REQUEST, headers, Json(error_response)).into_response();
        }
        Err(e) => {
            eprintln!("Database error: {}", e);
            let error_response = serde_json::json!({
                "error": "server_error",
                "error_description": "Internal server error"
            });
            
            let mut headers = HeaderMap::new();
            add_cors_headers(&mut headers);
            headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
            
            return (StatusCode::INTERNAL_SERVER_ERROR, headers, Json(error_response)).into_response();
        }
    };
    
    // Check if code was already used
    if auth_code.used_at.is_some() {
        let error_response = serde_json::json!({
            "error": "invalid_grant",
            "error_description": "Authorization code already used"
        });
        
        let mut headers = HeaderMap::new();
        add_cors_headers(&mut headers);
        headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
        
        return (StatusCode::BAD_REQUEST, headers, Json(error_response)).into_response();
    }
    
    // Check if code expired
    let expires_at = chrono::DateTime::parse_from_rfc3339(&auth_code.expires_at)
        .unwrap()
        .with_timezone(&Utc);
    if Utc::now() > expires_at {
        let error_response = serde_json::json!({
            "error": "invalid_grant",
            "error_description": "Authorization code expired"
        });
        
        let mut headers = HeaderMap::new();
        add_cors_headers(&mut headers);
        headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
        
        return (StatusCode::BAD_REQUEST, headers, Json(error_response)).into_response();
    }
    
    // Validate PKCE if present
    if let Some(challenge) = auth_code.code_challenge {
        let verifier = match &request.code_verifier {
            Some(v) => v,
            None => {
                let error_response = serde_json::json!({
                    "error": "invalid_request",
                    "error_description": "Missing code_verifier"
                });
                
                let mut headers = HeaderMap::new();
                add_cors_headers(&mut headers);
                headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
                
                return (StatusCode::BAD_REQUEST, headers, Json(error_response)).into_response();
            }
        };
        
        // Verify PKCE challenge
        let method = auth_code.code_challenge_method.as_deref().unwrap_or("S256");
        let computed_challenge = if method == "S256" {
            let mut hasher = Sha256::new();
            hasher.update(verifier.as_bytes());
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hasher.finalize())
        } else {
            verifier.clone()
        };
        
        if computed_challenge != challenge {
            let error_response = serde_json::json!({
                "error": "invalid_grant",
                "error_description": "Invalid code_verifier"
            });
            
            let mut headers = HeaderMap::new();
            add_cors_headers(&mut headers);
            headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
            
            return (StatusCode::BAD_REQUEST, headers, Json(error_response)).into_response();
        }
    }
    
    // Mark code as used
    let _ = sqlx::query!(
        "UPDATE oauth_authorization_codes SET used_at = datetime('now') WHERE code = ?",
        code
    )
    .execute(&state.db)
    .await;
    
    // Generate access token
    let access_token = format!("mcp_token_{}", uuid::Uuid::new_v4());
    let mut hasher = Sha256::new();
    hasher.update(access_token.as_bytes());
    let token_hash = format!("{:x}", hasher.finalize());
    
    let expires_at = (Utc::now() + Duration::hours(1)).to_rfc3339();
    
    // Store access token
    match sqlx::query!(
        r#"
        INSERT INTO oauth_access_tokens 
        (token_hash, client_id, user_id, mcp_token_id, scope, expires_at)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
        token_hash,
        auth_code.client_id,
        auth_code.user_id,
        "1", // Dummy MCP token ID for now
        auth_code.scope,
        expires_at
    )
    .execute(&state.db)
    .await {
        Ok(_) => {
            let response = TokenResponse {
                access_token,
                token_type: "Bearer".to_string(),
                expires_in: 3600,
                scope: auth_code.scope.unwrap_or_else(|| "mcp:read".to_string()),
                refresh_token: None, // Not implementing refresh tokens yet
            };
            
            let mut headers = HeaderMap::new();
            add_cors_headers(&mut headers);
            headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
            
            (StatusCode::OK, headers, Json(response)).into_response()
        }
        Err(e) => {
            eprintln!("Failed to create access token: {}", e);
            let error_response = serde_json::json!({
                "error": "server_error",
                "error_description": "Failed to create access token"
            });
            
            let mut headers = HeaderMap::new();
            add_cors_headers(&mut headers);
            headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
            
            (StatusCode::INTERNAL_SERVER_ERROR, headers, Json(error_response)).into_response()
        }
    }
}

// OAuth consent form submission
#[derive(Debug, Deserialize)]
pub struct AuthorizeConsentRequest {
    pub csrf_token: String,
    pub action: String, // "allow" or "deny"
    pub client_id: String,
    pub redirect_uri: String,
    pub response_type: String,
    pub scope: String,
    pub state: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
}

pub async fn authorize_consent(
    State(state): State<AppState>,
    session: axum_extra::extract::CookieJar,
    Form(request): Form<AuthorizeConsentRequest>,
) -> Result<impl IntoResponse, AppError> {
    // Validate CSRF token
    let stored_csrf = session.get("oauth_csrf")
        .and_then(|c| Some(c.value().to_string()));
    
    if stored_csrf != Some(request.csrf_token.clone()) {
        return Err(AppError::bad_request("Invalid CSRF token"));
    }
    
    // Remove CSRF token from session
    let session = session.remove(Cookie::from("oauth_csrf"));
    
    // Check if user denied access
    if request.action == "deny" {
        let mut redirect_url = format!("{}?error=access_denied", request.redirect_uri);
        if let Some(state) = &request.state {
            redirect_url.push_str(&format!("&state={}", urlencoding::encode(state)));
        }
        return Ok((session, Redirect::to(&redirect_url)).into_response());
    }
    
    // Get current user
    let user = get_current_user(&state.db, &session).await?
        .ok_or_else(|| AppError::unauthorized("Not authenticated"))?;
    
    // Generate authorization code
    let code = format!("code_{}", uuid::Uuid::new_v4());
    let expires_at = (Utc::now() + Duration::minutes(10)).to_rfc3339();
    
    // For now, use a dummy MCP token ID - in production, you'd create or lookup an MCP token
    let mcp_token_id = "1"; 
    let user_id = user.id.unwrap_or(0);
    
    // Store authorization code
    match sqlx::query!(
        r#"
        INSERT INTO oauth_authorization_codes 
        (code, client_id, user_id, mcp_token_id, redirect_uri, scope, code_challenge, code_challenge_method, expires_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
        code,
        request.client_id,
        user_id,
        mcp_token_id,
        request.redirect_uri,
        request.scope,
        request.code_challenge,
        request.code_challenge_method,
        expires_at
    )
    .execute(&state.db)
    .await {
        Ok(_) => {
            // Redirect back to client with authorization code
            let mut redirect_url = format!("{}?code={}", request.redirect_uri, code);
            if let Some(state) = &request.state {
                redirect_url.push_str(&format!("&state={}", urlencoding::encode(state)));
            }
            Ok((session, Redirect::to(&redirect_url)).into_response())
        }
        Err(e) => {
            eprintln!("Failed to store authorization code: {}", e);
            let mut redirect_url = format!("{}?error=server_error", request.redirect_uri);
            if let Some(state) = &request.state {
                redirect_url.push_str(&format!("&state={}", urlencoding::encode(state)));
            }
            Ok((session, Redirect::to(&redirect_url)).into_response())
        }
    }
}

pub async fn oauth_options() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    add_cors_headers(&mut headers);
    (StatusCode::NO_CONTENT, headers)
}