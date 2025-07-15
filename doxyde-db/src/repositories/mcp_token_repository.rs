use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use doxyde_core::models::McpToken;
use sqlx::SqlitePool;

pub struct McpTokenRepository {
    pool: SqlitePool,
}

impl McpTokenRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, token: &McpToken) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO mcp_tokens (id, user_id, site_id, name, created_at, last_used_at, revoked_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
            token.id,
            token.user_id,
            token.site_id,
            token.name,
            token.created_at,
            token.last_used_at,
            token.revoked_at
        )
        .execute(&self.pool)
        .await
        .context("Failed to create MCP token")?;

        Ok(())
    }

    pub async fn find_by_id(&self, id: &str) -> Result<Option<McpToken>> {
        let row = sqlx::query!(
            r#"
            SELECT 
                id as "id!",
                user_id as "user_id!",
                site_id as "site_id!",
                name as "name!",
                created_at as "created_at!: DateTime<Utc>",
                last_used_at as "last_used_at: DateTime<Utc>",
                revoked_at as "revoked_at: DateTime<Utc>"
            FROM mcp_tokens
            WHERE id = ?
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to find MCP token by id")?;

        Ok(row.map(|r| McpToken {
            id: r.id,
            user_id: r.user_id,
            site_id: r.site_id,
            name: r.name,
            created_at: r.created_at,
            last_used_at: r.last_used_at,
            revoked_at: r.revoked_at,
        }))
    }

    pub async fn find_by_user(&self, user_id: i64) -> Result<Vec<McpToken>> {
        let rows = sqlx::query!(
            r#"
            SELECT 
                id as "id!",
                user_id as "user_id!",
                site_id as "site_id!",
                name as "name!",
                created_at as "created_at!: DateTime<Utc>",
                last_used_at as "last_used_at: DateTime<Utc>",
                revoked_at as "revoked_at: DateTime<Utc>"
            FROM mcp_tokens
            WHERE user_id = ?
            ORDER BY created_at DESC
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to find MCP tokens by user")?;

        Ok(rows
            .into_iter()
            .map(|r| McpToken {
                id: r.id,
                user_id: r.user_id,
                site_id: r.site_id,
                name: r.name,
                created_at: r.created_at,
                last_used_at: r.last_used_at,
                revoked_at: r.revoked_at,
            })
            .collect())
    }

    pub async fn update_last_used(&self, id: &str) -> Result<()> {
        let now = Utc::now();
        sqlx::query!(
            r#"
            UPDATE mcp_tokens
            SET last_used_at = ?
            WHERE id = ?
            "#,
            now,
            id
        )
        .execute(&self.pool)
        .await
        .context("Failed to update last_used_at")?;

        Ok(())
    }

    pub async fn revoke(&self, id: &str) -> Result<()> {
        let now = Utc::now();
        sqlx::query!(
            r#"
            UPDATE mcp_tokens
            SET revoked_at = ?
            WHERE id = ?
            "#,
            now,
            id
        )
        .execute(&self.pool)
        .await
        .context("Failed to revoke MCP token")?;

        Ok(())
    }

    pub async fn delete_revoked_tokens(&self, older_than: DateTime<Utc>) -> Result<u64> {
        let result = sqlx::query!(
            r#"
            DELETE FROM mcp_tokens
            WHERE revoked_at IS NOT NULL AND revoked_at < ?
            "#,
            older_than
        )
        .execute(&self.pool)
        .await
        .context("Failed to delete old revoked tokens")?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use doxyde_core::models::McpToken;

    async fn setup_test_db(pool: &SqlitePool) -> Result<()> {
        // Create users table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                email TEXT NOT NULL UNIQUE,
                username TEXT NOT NULL UNIQUE,
                password_hash TEXT NOT NULL,
                is_active BOOLEAN NOT NULL DEFAULT 1,
                is_admin BOOLEAN NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            )
            "#,
        )
        .execute(pool)
        .await?;

        // Create sites table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sites (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                domain TEXT NOT NULL UNIQUE,
                title TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            )
            "#,
        )
        .execute(pool)
        .await?;

        // Create mcp_tokens table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS mcp_tokens (
                id TEXT PRIMARY KEY,
                user_id INTEGER NOT NULL,
                site_id INTEGER NOT NULL,
                name TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                last_used_at TEXT,
                revoked_at TEXT,
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
                FOREIGN KEY (site_id) REFERENCES sites(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(pool)
        .await?;

        // Insert test user
        sqlx::query(
            r#"
            INSERT INTO users (id, email, username, password_hash)
            VALUES (1, 'test@example.com', 'testuser', 'hash')
            "#,
        )
        .execute(pool)
        .await?;

        // Insert test site
        sqlx::query(
            r#"
            INSERT INTO sites (id, domain, title)
            VALUES (1, 'example.com', 'Test Site')
            "#,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_and_find_token(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        let repo = McpTokenRepository::new(pool);
        let token = McpToken::new(1, 1, "Test Token".to_string());

        repo.create(&token).await?;

        let found = repo.find_by_id(&token.id).await?;
        assert!(found.is_some());

        let found = found.unwrap();
        assert_eq!(found.user_id, 1);
        assert_eq!(found.site_id, 1);
        assert_eq!(found.name, "Test Token");
        assert!(found.is_valid());

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_user(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        let repo = McpTokenRepository::new(pool);

        // Insert second site for testing
        sqlx::query(
            r#"
            INSERT INTO sites (id, domain, title)
            VALUES (2, 'example2.com', 'Test Site 2')
            "#,
        )
        .execute(&repo.pool)
        .await?;

        // Insert second user for testing
        sqlx::query(
            r#"
            INSERT INTO users (id, email, username, password_hash)
            VALUES (2, 'test2@example.com', 'testuser2', 'hash2')
            "#,
        )
        .execute(&repo.pool)
        .await?;

        let token1 = McpToken::new(1, 1, "Token 1".to_string());
        let token2 = McpToken::new(1, 2, "Token 2".to_string());
        let token3 = McpToken::new(2, 1, "Token 3".to_string());

        repo.create(&token1).await?;
        repo.create(&token2).await?;
        repo.create(&token3).await?;

        let user1_tokens = repo.find_by_user(1).await?;
        assert_eq!(user1_tokens.len(), 2);

        let user2_tokens = repo.find_by_user(2).await?;
        assert_eq!(user2_tokens.len(), 1);

        Ok(())
    }

    #[sqlx::test]
    async fn test_update_last_used(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        let repo = McpTokenRepository::new(pool);
        let token = McpToken::new(1, 1, "Test Token".to_string());

        repo.create(&token).await?;
        assert!(token.last_used_at.is_none());

        repo.update_last_used(&token.id).await?;

        let found = repo.find_by_id(&token.id).await?.unwrap();
        assert!(found.last_used_at.is_some());

        Ok(())
    }

    #[sqlx::test]
    async fn test_revoke_token(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        let repo = McpTokenRepository::new(pool);
        let token = McpToken::new(1, 1, "Test Token".to_string());

        repo.create(&token).await?;

        let found = repo.find_by_id(&token.id).await?.unwrap();
        assert!(found.is_valid());

        repo.revoke(&token.id).await?;

        let found = repo.find_by_id(&token.id).await?.unwrap();
        assert!(!found.is_valid());
        assert!(found.revoked_at.is_some());

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_revoked_tokens(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;
        let repo = McpTokenRepository::new(pool);

        let mut token1 = McpToken::new(1, 1, "Token 1".to_string());
        let mut token2 = McpToken::new(1, 1, "Token 2".to_string());
        let token3 = McpToken::new(1, 1, "Token 3".to_string());

        token1.revoke();
        token2.revoke();

        repo.create(&token1).await?;
        repo.create(&token2).await?;
        repo.create(&token3).await?;

        let cutoff = Utc::now() + Duration::seconds(1);
        let deleted = repo.delete_revoked_tokens(cutoff).await?;
        assert_eq!(deleted, 2);

        let remaining = repo.find_by_user(1).await?;
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].name, "Token 3");

        Ok(())
    }
}
