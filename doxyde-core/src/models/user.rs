use anyhow::Result;
use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub id: Option<i64>,
    pub email: String,
    pub username: String,
    pub password_hash: String,
    pub is_active: bool,
    pub is_admin: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    /// Create a new user with a hashed password
    pub fn new(email: String, username: String, password: &str) -> Result<Self> {
        // Validate inputs first
        Self::validate_email(&email).map_err(|e| anyhow::anyhow!("Invalid email: {}", e))?;
        Self::validate_username(&username)
            .map_err(|e| anyhow::anyhow!("Invalid username: {}", e))?;

        // Allow empty password for testing scenarios, but it's not recommended in production
        // The password validation should be done at the handler/service level

        let password_hash = Self::hash_password(password)?;
        let now = Utc::now();

        Ok(Self {
            id: None,
            email,
            username,
            password_hash,
            is_active: true,
            is_admin: false,
            created_at: now,
            updated_at: now,
        })
    }

    /// Hash a password using Argon2
    pub fn hash_password(password: &str) -> Result<String> {
        use argon2::password_hash::rand_core::OsRng;

        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?
            .to_string();
        Ok(password_hash)
    }

    /// Set a new password for the user
    pub fn set_password(&mut self, password: &str) -> Result<()> {
        self.password_hash = Self::hash_password(password)?;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Verify a password against the stored hash
    pub fn verify_password(&self, password: &str) -> Result<bool> {
        use argon2::password_hash::{PasswordHash, PasswordVerifier};

        let parsed_hash = PasswordHash::new(&self.password_hash)
            .map_err(|e| anyhow::anyhow!("Invalid password hash format: {}", e))?;

        match Argon2::default().verify_password(password.as_bytes(), &parsed_hash) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Validate email format
    pub fn validate_email(email: &str) -> Result<(), String> {
        if email.is_empty() {
            return Err("Email cannot be empty".to_string());
        }

        if email.len() > 255 {
            return Err("Email cannot exceed 255 characters".to_string());
        }

        // Simple email regex - not perfect but good enough
        // Allow single char before @ but disallow leading/trailing dots
        let email_regex = Regex::new(r"^[a-zA-Z0-9]([a-zA-Z0-9._%+-]*[a-zA-Z0-9])?@[a-zA-Z0-9]([a-zA-Z0-9.-]*[a-zA-Z0-9])?\.[a-zA-Z]{2,}$")
            .map_err(|e| format!("Failed to compile email regex: {}", e))?;

        if !email_regex.is_match(email) {
            return Err("Invalid email format".to_string());
        }

        Ok(())
    }

    /// Validate username format
    pub fn validate_username(username: &str) -> Result<(), String> {
        if username.is_empty() {
            return Err("Username cannot be empty".to_string());
        }

        if username.len() < 3 {
            return Err("Username must be at least 3 characters".to_string());
        }

        if username.len() > 50 {
            return Err("Username cannot exceed 50 characters".to_string());
        }

        // Username must start with letter, can contain letters, numbers, underscore, hyphen
        let username_regex = Regex::new(r"^[a-zA-Z][a-zA-Z0-9_-]*$")
            .map_err(|e| format!("Failed to compile username regex: {}", e))?;

        if !username_regex.is_match(username) {
            return Err("Username must start with a letter and contain only letters, numbers, underscores, and hyphens".to_string());
        }

        Ok(())
    }

    /// Validate all user fields
    pub fn is_valid(&self) -> Result<(), String> {
        Self::validate_email(&self.email)?;
        Self::validate_username(&self.username)?;

        if self.password_hash.is_empty() {
            return Err("Password hash cannot be empty".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_user() {
        let user = User::new(
            "test@example.com".to_string(),
            "testuser".to_string(),
            "password123",
        )
        .unwrap();

        assert!(user.id.is_none());
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.username, "testuser");
        assert_ne!(user.password_hash, "password123"); // Should be hashed
        assert!(user.is_active);
        assert!(!user.is_admin);
    }

    #[test]
    fn test_new_user_with_different_passwords_have_different_hashes() {
        let user1 = User::new(
            "test1@example.com".to_string(),
            "user1".to_string(),
            "password123",
        )
        .unwrap();

        let user2 = User::new(
            "test2@example.com".to_string(),
            "user2".to_string(),
            "password456",
        )
        .unwrap();

        assert_ne!(user1.password_hash, user2.password_hash);
    }

    #[test]
    fn test_new_user_timestamps() {
        let before = Utc::now();
        let user = User::new(
            "test@example.com".to_string(),
            "testuser".to_string(),
            "password123",
        )
        .unwrap();
        let after = Utc::now();

        assert!(user.created_at >= before);
        assert!(user.created_at <= after);
        assert_eq!(user.created_at, user.updated_at);
    }

    #[test]
    fn test_hash_password() {
        let hash1 = User::hash_password("password123").unwrap();
        let hash2 = User::hash_password("password123").unwrap();

        // Same password should produce different hashes (due to salt)
        assert_ne!(hash1, hash2);

        // Hashes should be valid Argon2 format
        assert!(hash1.starts_with("$argon2"));
        assert!(hash2.starts_with("$argon2"));
    }

    #[test]
    fn test_hash_password_empty() {
        // Empty password should still hash successfully
        let hash = User::hash_password("").unwrap();
        assert!(hash.starts_with("$argon2"));
    }

    #[test]
    fn test_hash_password_long() {
        let long_password = "a".repeat(1000);
        let hash = User::hash_password(&long_password).unwrap();
        assert!(hash.starts_with("$argon2"));
    }

    #[test]
    fn test_verify_password_correct() {
        let user = User::new(
            "test@example.com".to_string(),
            "testuser".to_string(),
            "correct_password",
        )
        .unwrap();

        assert!(user.verify_password("correct_password").unwrap());
    }

    #[test]
    fn test_verify_password_incorrect() {
        let user = User::new(
            "test@example.com".to_string(),
            "testuser".to_string(),
            "correct_password",
        )
        .unwrap();

        assert!(!user.verify_password("wrong_password").unwrap());
    }

    #[test]
    fn test_set_password() {
        let mut user = User::new(
            "test@example.com".to_string(),
            "testuser".to_string(),
            "old_password",
        )
        .unwrap();

        let old_updated_at = user.updated_at;

        // Set new password
        user.set_password("new_password").unwrap();

        // Password should be updated
        assert!(user.verify_password("new_password").unwrap());
        assert!(!user.verify_password("old_password").unwrap());

        // updated_at should be changed
        assert!(user.updated_at > old_updated_at);
    }

    #[test]
    fn test_verify_password_empty() {
        let user = User::new("test@example.com".to_string(), "testuser".to_string(), "").unwrap();

        assert!(user.verify_password("").unwrap());
        assert!(!user.verify_password("not_empty").unwrap());
    }

    #[test]
    fn test_verify_password_invalid_hash() {
        let mut user = User::new(
            "test@example.com".to_string(),
            "testuser".to_string(),
            "password",
        )
        .unwrap();

        // Set invalid hash
        user.password_hash = "invalid_hash".to_string();

        // Should return error for invalid hash format
        assert!(user.verify_password("password").is_err());
    }

    #[test]
    fn test_validate_email_valid() {
        assert!(User::validate_email("user@example.com").is_ok());
        assert!(User::validate_email("user.name@example.com").is_ok());
        assert!(User::validate_email("user+tag@example.co.uk").is_ok());
        assert!(User::validate_email("user123@test-domain.org").is_ok());
    }

    #[test]
    fn test_validate_email_invalid() {
        assert!(User::validate_email("").is_err());
        assert!(User::validate_email("not-an-email").is_err());
        assert!(User::validate_email("@example.com").is_err());
        assert!(User::validate_email("user@").is_err());
        assert!(User::validate_email("user@.com").is_err());
        assert!(User::validate_email("user@example").is_err());
        assert!(User::validate_email("user @example.com").is_err());
        assert!(User::validate_email("user@exam ple.com").is_err());
    }

    #[test]
    fn test_validate_email_too_long() {
        let long_email = format!("{}@example.com", "a".repeat(250));
        let result = User::validate_email(&long_email);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("exceed 255"));
    }

    #[test]
    fn test_validate_email_edge_cases() {
        // Valid edge cases
        assert!(User::validate_email("a@b.co").is_ok());
        assert!(User::validate_email("test.multiple.dots@example.com").is_ok());
        assert!(User::validate_email("1234567890@example.com").is_ok());

        // Invalid edge cases
        assert!(User::validate_email("double@@example.com").is_err());
        assert!(User::validate_email("trailing.dot.@example.com").is_err());
    }

    #[test]
    fn test_validate_username_valid() {
        assert!(User::validate_username("user").is_ok());
        assert!(User::validate_username("User123").is_ok());
        assert!(User::validate_username("user_name").is_ok());
        assert!(User::validate_username("user-name").is_ok());
        assert!(User::validate_username("JohnDoe2024").is_ok());
    }

    #[test]
    fn test_validate_username_invalid() {
        assert!(User::validate_username("").is_err());
        assert!(User::validate_username("ab").is_err()); // Too short
        assert!(User::validate_username("123user").is_err()); // Starts with number
        assert!(User::validate_username("_user").is_err()); // Starts with underscore
        assert!(User::validate_username("-user").is_err()); // Starts with hyphen
        assert!(User::validate_username("user name").is_err()); // Contains space
        assert!(User::validate_username("user@name").is_err()); // Contains @
        assert!(User::validate_username("user.name").is_err()); // Contains dot
    }

    #[test]
    fn test_validate_username_length() {
        // Minimum length (3)
        assert!(User::validate_username("abc").is_ok());

        // Too short
        let result = User::validate_username("ab");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("at least 3"));

        // Maximum length (50)
        let max_username = "a".repeat(50);
        assert!(User::validate_username(&max_username).is_ok());

        // Too long
        let long_username = "a".repeat(51);
        let result = User::validate_username(&long_username);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("exceed 50"));
    }

    #[test]
    fn test_is_valid() {
        let user = User::new(
            "valid@example.com".to_string(),
            "validuser".to_string(),
            "password",
        )
        .unwrap();

        assert!(user.is_valid().is_ok());
    }

    #[test]
    fn test_is_valid_invalid_email() {
        let mut user = User::new(
            "valid@example.com".to_string(),
            "validuser".to_string(),
            "password",
        )
        .unwrap();

        user.email = "invalid-email".to_string();
        let result = user.is_valid();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid email"));
    }

    #[test]
    fn test_is_valid_invalid_username() {
        let mut user = User::new(
            "valid@example.com".to_string(),
            "validuser".to_string(),
            "password",
        )
        .unwrap();

        user.username = "ab".to_string(); // Too short
        let result = user.is_valid();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("at least 3"));
    }

    #[test]
    fn test_is_valid_empty_password_hash() {
        let mut user = User::new(
            "valid@example.com".to_string(),
            "validuser".to_string(),
            "password",
        )
        .unwrap();

        user.password_hash = "".to_string();
        let result = user.is_valid();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Password hash cannot be empty"));
    }

    #[test]
    fn test_new_with_invalid_email() {
        let result = User::new(
            "invalid-email".to_string(),
            "validuser".to_string(),
            "password",
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid email"));
    }

    #[test]
    fn test_new_with_invalid_username() {
        let result = User::new(
            "valid@example.com".to_string(),
            "ab".to_string(), // Too short
            "password",
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid username"));
    }

    #[test]
    fn test_new_with_empty_password() {
        // Empty password is allowed at the model level but should be validated at handler level
        let result = User::new("valid@example.com".to_string(), "validuser".to_string(), "");

        assert!(result.is_ok());
        let user = result.unwrap();
        assert!(user.verify_password("").unwrap());
        assert!(!user.verify_password("not_empty").unwrap());
    }
}
