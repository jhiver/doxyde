use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

#[derive(Debug, Clone)]
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
    pub fn is_valid(&self) -> bool {
        Utc::now() < self.expires_at
    }
}

pub struct AccessTokenRepository {
    pool: SqlitePool,
}

impl AccessTokenRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, token: &AccessToken) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO oauth_access_tokens (
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
        .context("Failed to create access token")?;

        Ok(())
    }

    pub async fn find_by_hash(&self, token_hash: &str) -> Result<Option<AccessToken>> {
        let row = sqlx::query!(
            r#"
            SELECT 
                token_hash as "token_hash!",
                client_id as "client_id!",
                user_id as "user_id!",
                mcp_token_id as "mcp_token_id!",
                scope,
                expires_at as "expires_at!: DateTime<Utc>",
                created_at as "created_at!: DateTime<Utc>"
            FROM oauth_access_tokens
            WHERE token_hash = ?
            "#,
            token_hash
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to find access token")?;

        Ok(row.map(|r| AccessToken {
            token: String::new(), // Token is not stored in DB, only hash
            token_hash: r.token_hash,
            client_id: r.client_id,
            user_id: r.user_id,
            mcp_token_id: r.mcp_token_id,
            scope: r.scope,
            expires_at: r.expires_at,
            created_at: r.created_at,
        }))
    }

    pub async fn delete_by_hash(&self, token_hash: &str) -> Result<()> {
        sqlx::query!(
            r#"
            DELETE FROM oauth_access_tokens
            WHERE token_hash = ?
            "#,
            token_hash
        )
        .execute(&self.pool)
        .await
        .context("Failed to delete access token")?;

        Ok(())
    }

    pub async fn delete_by_mcp_token(&self, mcp_token_id: &str) -> Result<u64> {
        let result = sqlx::query!(
            r#"
            DELETE FROM oauth_access_tokens
            WHERE mcp_token_id = ?
            "#,
            mcp_token_id
        )
        .execute(&self.pool)
        .await
        .context("Failed to delete access tokens by MCP token")?;

        Ok(result.rows_affected())
    }

    pub async fn delete_expired(&self) -> Result<u64> {
        let now = Utc::now();
        let result = sqlx::query!(
            r#"
            DELETE FROM oauth_access_tokens
            WHERE expires_at < ?
            "#,
            now
        )
        .execute(&self.pool)
        .await
        .context("Failed to delete expired access tokens")?;

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
            CREATE TABLE IF NOT EXISTS oauth_access_tokens (
                token_hash TEXT PRIMARY KEY,
                client_id TEXT NOT NULL,
                user_id INTEGER NOT NULL,
                mcp_token_id TEXT NOT NULL,
                scope TEXT,
                expires_at TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )
            "#,
        )
        .execute(&pool)
        .await?;

        let repo = AccessTokenRepository::new(pool);

        let token = AccessToken {
            token: "test_token_123".to_string(),
            token_hash: "hashed_token_123".to_string(),
            client_id: "client-123".to_string(),
            user_id: 1,
            mcp_token_id: "mcp-token-123".to_string(),
            scope: Some("mcp:read mcp:write".to_string()),
            expires_at: Utc::now() + Duration::hours(1),
            created_at: Utc::now(),
        };

        repo.create(&token).await?;

        let found = repo.find_by_hash(&token.token_hash).await?;
        assert!(found.is_some());

        let found = found.unwrap();
        assert_eq!(found.client_id, "client-123");
        assert_eq!(found.mcp_token_id, "mcp-token-123");

        // Delete by hash
        repo.delete_by_hash(&token.token_hash).await?;

        let found = repo.find_by_hash(&token.token_hash).await?;
        assert!(found.is_none());

        Ok(())
    }
}
