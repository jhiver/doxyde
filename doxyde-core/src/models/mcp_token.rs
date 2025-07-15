use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToken {
    pub id: String,
    pub user_id: i64,
    pub site_id: i64,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
}

impl McpToken {
    pub fn new(user_id: i64, site_id: i64, name: String) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            user_id,
            site_id,
            name,
            created_at: Utc::now(),
            last_used_at: None,
            revoked_at: None,
        }
    }

    pub fn is_valid(&self) -> bool {
        self.revoked_at.is_none()
    }

    pub fn revoke(&mut self) {
        self.revoked_at = Some(Utc::now());
    }

    pub fn update_last_used(&mut self) {
        self.last_used_at = Some(Utc::now());
    }

    pub fn validate_name(name: &str) -> Result<()> {
        if name.trim().is_empty() {
            anyhow::bail!("Token name cannot be empty");
        }
        if name.len() > 255 {
            anyhow::bail!("Token name must be 255 characters or less");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_token() {
        let token = McpToken::new(1, 2, "Test Token".to_string());
        assert_eq!(token.user_id, 1);
        assert_eq!(token.site_id, 2);
        assert_eq!(token.name, "Test Token");
        assert!(token.is_valid());
        assert!(token.last_used_at.is_none());
        assert!(token.revoked_at.is_none());
    }

    #[test]
    fn test_token_id_is_uuid() {
        let token = McpToken::new(1, 2, "Test".to_string());
        let uuid = uuid::Uuid::parse_str(&token.id);
        assert!(uuid.is_ok());
    }

    #[test]
    fn test_revoke_token() {
        let mut token = McpToken::new(1, 2, "Test".to_string());
        assert!(token.is_valid());

        token.revoke();
        assert!(!token.is_valid());
        assert!(token.revoked_at.is_some());
    }

    #[test]
    fn test_update_last_used() {
        let mut token = McpToken::new(1, 2, "Test".to_string());
        assert!(token.last_used_at.is_none());

        token.update_last_used();
        assert!(token.last_used_at.is_some());
    }

    #[test]
    fn test_validate_name() {
        assert!(McpToken::validate_name("Valid Name").is_ok());
        assert!(McpToken::validate_name("").is_err());
        assert!(McpToken::validate_name("   ").is_err());
        assert!(McpToken::validate_name(&"a".repeat(256)).is_err());
    }
}
