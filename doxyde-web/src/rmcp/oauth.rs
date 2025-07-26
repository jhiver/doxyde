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
    http::StatusCode,
    response::IntoResponse,
    Json,
};
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