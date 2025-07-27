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
use chrono::Utc;
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;

#[derive(Debug)]
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

    // First check OAuth access tokens
    let oauth_result = sqlx::query!(
        r#"
        SELECT oat.mcp_token_id as "mcp_token_id!", oat.scope, oat.expires_at,
               mt.id as "id!: i64", mt.site_id as "site_id!: i64"
        FROM oauth_access_tokens oat
        INNER JOIN mcp_tokens mt ON mt.id = CAST(oat.mcp_token_id AS INTEGER)
        WHERE oat.token_hash = ?
        "#,
        token_hash
    )
    .fetch_optional(db)
    .await
    .context("Failed to validate OAuth token")?;

    if let Some(row) = oauth_result {
        // Check expiration
        let expiry = chrono::DateTime::parse_from_rfc3339(&row.expires_at)
            .context("Failed to parse expiry date")?
            .with_timezone(&Utc);
        if Utc::now() > expiry {
            return Ok(None); // Token expired
        }

        // Update last_used_at on the MCP token
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

        return Ok(Some(TokenInfo {
            id: row.id,
            site_id: row.site_id,
            scopes: row.scope,
        }));
    }

    // Fall back to checking MCP tokens directly
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_hash_consistency() {
        let token = "test-token";
        let mut hasher1 = Sha256::new();
        hasher1.update(token.as_bytes());
        let hash1 = format!("{:x}", hasher1.finalize());

        let mut hasher2 = Sha256::new();
        hasher2.update(token.as_bytes());
        let hash2 = format!("{:x}", hasher2.finalize());

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_token_info_struct() {
        let token_info = TokenInfo {
            id: 1,
            site_id: 2,
            scopes: Some("read write".to_string()),
        };
        assert_eq!(token_info.id, 1);
        assert_eq!(token_info.site_id, 2);
        assert_eq!(token_info.scopes, Some("read write".to_string()));
    }

    // Note: Full integration tests with database would require
    // running with SQLX_OFFLINE=false or using a test setup
    // For now, we test the logic that doesn't require database
}