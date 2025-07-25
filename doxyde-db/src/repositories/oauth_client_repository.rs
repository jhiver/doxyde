use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;

#[derive(Debug, Clone)]
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

pub struct OAuthClientRepository {
    pool: SqlitePool,
}

impl OAuthClientRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, client: &OAuthClient) -> Result<()> {
        let redirect_uris = serde_json::to_string(&client.redirect_uris)?;
        let grant_types = serde_json::to_string(&client.grant_types)?;
        let response_types = serde_json::to_string(&client.response_types)?;

        sqlx::query!(
            r#"
            INSERT INTO oauth_clients (
                client_id, client_secret_hash, client_name, redirect_uris,
                grant_types, response_types, scope, token_endpoint_auth_method,
                created_at, created_by_token_id
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            client.client_id,
            client.client_secret_hash,
            client.client_name,
            redirect_uris,
            grant_types,
            response_types,
            client.scope,
            client.token_endpoint_auth_method,
            client.created_at,
            client.created_by_token_id
        )
        .execute(&self.pool)
        .await
        .context("Failed to create OAuth client")?;

        Ok(())
    }

    pub async fn find_by_id(&self, client_id: &str) -> Result<Option<OAuthClient>> {
        let row = sqlx::query!(
            r#"
            SELECT 
                client_id as "client_id!",
                client_secret_hash,
                client_name as "client_name!",
                redirect_uris as "redirect_uris!",
                grant_types as "grant_types!",
                response_types as "response_types!",
                scope,
                token_endpoint_auth_method as "token_endpoint_auth_method!",
                created_at as "created_at!: DateTime<Utc>",
                created_by_token_id
            FROM oauth_clients
            WHERE client_id = ?
            "#,
            client_id
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to find OAuth client by id")?;

        match row {
            Some(r) => {
                let redirect_uris: Vec<String> = serde_json::from_str(&r.redirect_uris)?;
                let grant_types: Vec<String> = serde_json::from_str(&r.grant_types)?;
                let response_types: Vec<String> = serde_json::from_str(&r.response_types)?;

                Ok(Some(OAuthClient {
                    client_id: r.client_id,
                    client_secret_hash: r.client_secret_hash,
                    client_name: r.client_name,
                    redirect_uris,
                    grant_types,
                    response_types,
                    scope: r.scope,
                    token_endpoint_auth_method: r.token_endpoint_auth_method,
                    created_at: r.created_at,
                    created_by_token_id: r.created_by_token_id,
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn find_by_token(&self, token_id: &str) -> Result<Vec<OAuthClient>> {
        let rows = sqlx::query!(
            r#"
            SELECT 
                client_id as "client_id!",
                client_secret_hash,
                client_name as "client_name!",
                redirect_uris as "redirect_uris!",
                grant_types as "grant_types!",
                response_types as "response_types!",
                scope,
                token_endpoint_auth_method as "token_endpoint_auth_method!",
                created_at as "created_at!: DateTime<Utc>",
                created_by_token_id
            FROM oauth_clients
            WHERE created_by_token_id = ?
            ORDER BY created_at DESC
            "#,
            token_id
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to find OAuth clients by token")?;

        let mut clients = Vec::new();
        for r in rows {
            let redirect_uris: Vec<String> = serde_json::from_str(&r.redirect_uris)?;
            let grant_types: Vec<String> = serde_json::from_str(&r.grant_types)?;
            let response_types: Vec<String> = serde_json::from_str(&r.response_types)?;

            clients.push(OAuthClient {
                client_id: r.client_id,
                client_secret_hash: r.client_secret_hash,
                client_name: r.client_name,
                redirect_uris,
                grant_types,
                response_types,
                scope: r.scope,
                token_endpoint_auth_method: r.token_endpoint_auth_method,
                created_at: r.created_at,
                created_by_token_id: r.created_by_token_id,
            });
        }

        Ok(clients)
    }

    pub async fn delete(&self, client_id: &str) -> Result<()> {
        sqlx::query!(
            r#"
            DELETE FROM oauth_clients
            WHERE client_id = ?
            "#,
            client_id
        )
        .execute(&self.pool)
        .await
        .context("Failed to delete OAuth client")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test]
    async fn test_create_and_find_client(pool: SqlitePool) -> Result<()> {
        // Setup tables
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS oauth_clients (
                client_id TEXT PRIMARY KEY,
                client_secret_hash TEXT,
                client_name TEXT NOT NULL,
                redirect_uris TEXT NOT NULL,
                grant_types TEXT NOT NULL DEFAULT '["authorization_code"]',
                response_types TEXT NOT NULL DEFAULT '["code"]',
                scope TEXT DEFAULT 'mcp:read mcp:write',
                token_endpoint_auth_method TEXT DEFAULT 'client_secret_basic',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                created_by_token_id TEXT
            )
            "#,
        )
        .execute(&pool)
        .await?;

        let repo = OAuthClientRepository::new(pool);

        let client = OAuthClient {
            client_id: "test-client-123".to_string(),
            client_secret_hash: Some("hashed_secret".to_string()),
            client_name: "Test Client".to_string(),
            redirect_uris: vec!["http://localhost:3000/callback".to_string()],
            grant_types: vec!["authorization_code".to_string()],
            response_types: vec!["code".to_string()],
            scope: Some("mcp:read mcp:write".to_string()),
            token_endpoint_auth_method: "client_secret_basic".to_string(),
            created_at: Utc::now(),
            created_by_token_id: None,
        };

        repo.create(&client).await?;

        let found = repo.find_by_id(&client.client_id).await?;
        assert!(found.is_some());

        let found = found.unwrap();
        assert_eq!(found.client_name, "Test Client");
        assert_eq!(found.redirect_uris.len(), 1);

        Ok(())
    }
}