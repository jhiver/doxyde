use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

#[derive(Debug, Clone)]
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
    pub fn is_valid(&self) -> bool {
        self.used_at.is_none() && Utc::now() < self.expires_at
    }
}

pub struct AuthorizationCodeRepository {
    pool: SqlitePool,
}

impl AuthorizationCodeRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, code: &AuthorizationCode) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO oauth_authorization_codes (
                code, client_id, user_id, mcp_token_id, redirect_uri,
                scope, code_challenge, code_challenge_method, expires_at,
                created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            code.code,
            code.client_id,
            code.user_id,
            code.mcp_token_id,
            code.redirect_uri,
            code.scope,
            code.code_challenge,
            code.code_challenge_method,
            code.expires_at,
            code.created_at
        )
        .execute(&self.pool)
        .await
        .context("Failed to create authorization code")?;

        Ok(())
    }

    pub async fn find_by_code(&self, code: &str) -> Result<Option<AuthorizationCode>> {
        let row = sqlx::query!(
            r#"
            SELECT 
                code as "code!",
                client_id as "client_id!",
                user_id as "user_id!",
                mcp_token_id as "mcp_token_id!",
                redirect_uri as "redirect_uri!",
                scope,
                code_challenge,
                code_challenge_method,
                expires_at as "expires_at!: DateTime<Utc>",
                used_at as "used_at: DateTime<Utc>",
                created_at as "created_at!: DateTime<Utc>"
            FROM oauth_authorization_codes
            WHERE code = ?
            "#,
            code
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to find authorization code")?;

        Ok(row.map(|r| AuthorizationCode {
            code: r.code,
            client_id: r.client_id,
            user_id: r.user_id,
            mcp_token_id: r.mcp_token_id,
            redirect_uri: r.redirect_uri,
            scope: r.scope,
            code_challenge: r.code_challenge,
            code_challenge_method: r.code_challenge_method,
            expires_at: r.expires_at,
            used_at: r.used_at,
            created_at: r.created_at,
        }))
    }

    pub async fn mark_used(&self, code: &str) -> Result<()> {
        let now = Utc::now();
        sqlx::query!(
            r#"
            UPDATE oauth_authorization_codes
            SET used_at = ?
            WHERE code = ?
            "#,
            now,
            code
        )
        .execute(&self.pool)
        .await
        .context("Failed to mark authorization code as used")?;

        Ok(())
    }

    pub async fn delete_expired(&self) -> Result<u64> {
        let now = Utc::now();
        let result = sqlx::query!(
            r#"
            DELETE FROM oauth_authorization_codes
            WHERE expires_at < ?
            "#,
            now
        )
        .execute(&self.pool)
        .await
        .context("Failed to delete expired authorization codes")?;

        Ok(result.rows_affected())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[sqlx::test]
    async fn test_create_and_find_code(pool: SqlitePool) -> Result<()> {
        // Setup tables
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS oauth_authorization_codes (
                code TEXT PRIMARY KEY,
                client_id TEXT NOT NULL,
                user_id INTEGER NOT NULL,
                mcp_token_id TEXT NOT NULL,
                redirect_uri TEXT NOT NULL,
                scope TEXT,
                code_challenge TEXT,
                code_challenge_method TEXT,
                expires_at TEXT NOT NULL,
                used_at TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now'))
            )
            "#,
        )
        .execute(&pool)
        .await?;

        let repo = AuthorizationCodeRepository::new(pool);

        let code = AuthorizationCode {
            code: "test-code-123".to_string(),
            client_id: "client-123".to_string(),
            user_id: 1,
            mcp_token_id: "token-123".to_string(),
            redirect_uri: "http://localhost:3000/callback".to_string(),
            scope: Some("mcp:read".to_string()),
            code_challenge: Some("challenge".to_string()),
            code_challenge_method: Some("S256".to_string()),
            expires_at: Utc::now() + Duration::minutes(10),
            used_at: None,
            created_at: Utc::now(),
        };

        repo.create(&code).await?;

        let found = repo.find_by_code(&code.code).await?;
        assert!(found.is_some());

        let found = found.unwrap();
        assert_eq!(found.client_id, "client-123");
        assert!(found.used_at.is_none());

        // Mark as used
        repo.mark_used(&code.code).await?;

        let found = repo.find_by_code(&code.code).await?.unwrap();
        assert!(found.used_at.is_some());

        Ok(())
    }
}