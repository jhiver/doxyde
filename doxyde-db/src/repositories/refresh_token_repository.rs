use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

#[derive(Debug, Clone)]
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
    pub fn is_valid(&self) -> bool {
        self.used_at.is_none() && Utc::now() < self.expires_at
    }
}

pub struct RefreshTokenRepository {
    pool: SqlitePool,
}

impl RefreshTokenRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, token: &RefreshToken) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO oauth_refresh_tokens (
                token_hash, client_id, user_id, mcp_token_id,
                scope, expires_at, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            token.token_hash,
            token.client_id,
            token.user_id,
            token.mcp_token_id,
            token.scope,
            token.expires_at,
            token.created_at
        )
        .execute(&self.pool)
        .await
        .context("Failed to create refresh token")?;

        Ok(())
    }

    pub async fn find_by_hash(&self, token_hash: &str) -> Result<Option<RefreshToken>> {
        let row = sqlx::query!(
            r#"
            SELECT 
                token_hash as "token_hash!",
                client_id as "client_id!",
                user_id as "user_id!",
                mcp_token_id as "mcp_token_id!",
                scope,
                expires_at as "expires_at!: DateTime<Utc>",
                used_at as "used_at: DateTime<Utc>",
                created_at as "created_at!: DateTime<Utc>"
            FROM oauth_refresh_tokens
            WHERE token_hash = ?
            "#,
            token_hash
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to find refresh token")?;

        Ok(row.map(|r| RefreshToken {
            token: String::new(), // Token is not stored in DB, only hash
            token_hash: r.token_hash,
            client_id: r.client_id,
            user_id: r.user_id,
            mcp_token_id: r.mcp_token_id,
            scope: r.scope,
            expires_at: r.expires_at,
            used_at: r.used_at,
            created_at: r.created_at,
        }))
    }

    pub async fn mark_used(&self, token_hash: &str) -> Result<()> {
        let now = Utc::now();
        sqlx::query!(
            r#"
            UPDATE oauth_refresh_tokens
            SET used_at = ?
            WHERE token_hash = ?
            "#,
            now,
            token_hash
        )
        .execute(&self.pool)
        .await
        .context("Failed to mark refresh token as used")?;

        Ok(())
    }

    pub async fn delete_by_mcp_token(&self, mcp_token_id: &str) -> Result<u64> {
        let result = sqlx::query!(
            r#"
            DELETE FROM oauth_refresh_tokens
            WHERE mcp_token_id = ?
            "#,
            mcp_token_id
        )
        .execute(&self.pool)
        .await
        .context("Failed to delete refresh tokens by MCP token")?;

        Ok(result.rows_affected())
    }

    pub async fn delete_expired(&self) -> Result<u64> {
        let now = Utc::now();
        let result = sqlx::query!(
            r#"
            DELETE FROM oauth_refresh_tokens
            WHERE expires_at < ? OR used_at IS NOT NULL
            "#,
            now
        )
        .execute(&self.pool)
        .await
        .context("Failed to delete expired or used refresh tokens")?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[sqlx::test]
    async fn test_create_and_find_token(pool: SqlitePool) -> Result<()> {
        // Setup tables
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS oauth_refresh_tokens (
                token_hash TEXT PRIMARY KEY,
                client_id TEXT NOT NULL,
                user_id INTEGER NOT NULL,
                mcp_token_id TEXT NOT NULL,
                scope TEXT,
                expires_at TEXT NOT NULL,
                used_at TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )
            "#,
        )
        .execute(&pool)
        .await?;

        let repo = RefreshTokenRepository::new(pool);

        let token = RefreshToken {
            token_hash: "hashed_refresh_123".to_string(),
            client_id: "client-123".to_string(),
            user_id: 1,
            mcp_token_id: "mcp-token-123".to_string(),
            scope: Some("mcp:read mcp:write".to_string()),
            expires_at: Utc::now() + Duration::days(30),
            used_at: None,
            created_at: Utc::now(),
        };

        repo.create(&token).await?;

        let found = repo.find_by_hash(&token.token_hash).await?;
        assert!(found.is_some());

        let found = found.unwrap();
        assert_eq!(found.client_id, "client-123");
        assert!(found.used_at.is_none());

        // Mark as used
        repo.mark_used(&token.token_hash).await?;

        let found = repo.find_by_hash(&token.token_hash).await?.unwrap();
        assert!(found.used_at.is_some());

        Ok(())
    }
}