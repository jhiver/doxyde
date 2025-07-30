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
use doxyde_core::models::user::User;
use sqlx::SqlitePool;

pub struct UserRepository {
    pool: SqlitePool,
}

impl UserRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, user: &User) -> Result<i64> {
        if user.is_valid().is_err() {
            return Err(anyhow::anyhow!("Invalid user: {:?}", user.is_valid().err()));
        }

        let result = sqlx::query(
            r#"
            INSERT INTO users (email, username, password_hash, is_active, is_admin, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&user.email)
        .bind(&user.username)
        .bind(&user.password_hash)
        .bind(user.is_active)
        .bind(user.is_admin)
        .bind(user.created_at)
        .bind(user.updated_at)
        .execute(&self.pool)
        .await
        .context("Failed to create user")?;

        Ok(result.last_insert_rowid())
    }

    pub async fn find_by_id(&self, id: i64) -> Result<Option<User>> {
        let row = sqlx::query_as::<_, (i64, String, String, String, bool, bool, String, String)>(
            r#"
            SELECT id, email, username, password_hash, is_active, is_admin, created_at, updated_at
            FROM users
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to find user by id")?;

        match row {
            Some((
                id,
                email,
                username,
                password_hash,
                is_active,
                is_admin,
                created_at_str,
                updated_at_str,
            )) => {
                // Parse datetime strings
                let created_at = if created_at_str.contains('T') {
                    chrono::DateTime::parse_from_rfc3339(&created_at_str)
                        .context("Failed to parse created_at as RFC3339")?
                        .with_timezone(&chrono::Utc)
                } else {
                    chrono::NaiveDateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S")
                        .context("Failed to parse created_at as SQLite format")?
                        .and_utc()
                };

                let updated_at = if updated_at_str.contains('T') {
                    chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                        .context("Failed to parse updated_at as RFC3339")?
                        .with_timezone(&chrono::Utc)
                } else {
                    chrono::NaiveDateTime::parse_from_str(&updated_at_str, "%Y-%m-%d %H:%M:%S")
                        .context("Failed to parse updated_at as SQLite format")?
                        .and_utc()
                };

                Ok(Some(User {
                    id: Some(id),
                    email,
                    username,
                    password_hash,
                    is_active,
                    is_admin,
                    created_at,
                    updated_at,
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>> {
        let row = sqlx::query_as::<_, (i64, String, String, String, bool, bool, String, String)>(
            r#"
            SELECT id, email, username, password_hash, is_active, is_admin, created_at, updated_at
            FROM users
            WHERE email = ?
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to find user by email")?;

        match row {
            Some((
                id,
                email,
                username,
                password_hash,
                is_active,
                is_admin,
                created_at_str,
                updated_at_str,
            )) => {
                // Parse datetime strings
                let created_at = if created_at_str.contains('T') {
                    chrono::DateTime::parse_from_rfc3339(&created_at_str)
                        .context("Failed to parse created_at as RFC3339")?
                        .with_timezone(&chrono::Utc)
                } else {
                    chrono::NaiveDateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S")
                        .context("Failed to parse created_at as SQLite format")?
                        .and_utc()
                };

                let updated_at = if updated_at_str.contains('T') {
                    chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                        .context("Failed to parse updated_at as RFC3339")?
                        .with_timezone(&chrono::Utc)
                } else {
                    chrono::NaiveDateTime::parse_from_str(&updated_at_str, "%Y-%m-%d %H:%M:%S")
                        .context("Failed to parse updated_at as SQLite format")?
                        .and_utc()
                };

                Ok(Some(User {
                    id: Some(id),
                    email,
                    username,
                    password_hash,
                    is_active,
                    is_admin,
                    created_at,
                    updated_at,
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn find_by_username(&self, username: &str) -> Result<Option<User>> {
        let row = sqlx::query_as::<_, (i64, String, String, String, bool, bool, String, String)>(
            r#"
            SELECT id, email, username, password_hash, is_active, is_admin, created_at, updated_at
            FROM users
            WHERE username = ?
            "#,
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to find user by username")?;

        match row {
            Some((
                id,
                email,
                username,
                password_hash,
                is_active,
                is_admin,
                created_at_str,
                updated_at_str,
            )) => {
                // Parse datetime strings
                let created_at = if created_at_str.contains('T') {
                    chrono::DateTime::parse_from_rfc3339(&created_at_str)
                        .context("Failed to parse created_at as RFC3339")?
                        .with_timezone(&chrono::Utc)
                } else {
                    chrono::NaiveDateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S")
                        .context("Failed to parse created_at as SQLite format")?
                        .and_utc()
                };

                let updated_at = if updated_at_str.contains('T') {
                    chrono::DateTime::parse_from_rfc3339(&updated_at_str)
                        .context("Failed to parse updated_at as RFC3339")?
                        .with_timezone(&chrono::Utc)
                } else {
                    chrono::NaiveDateTime::parse_from_str(&updated_at_str, "%Y-%m-%d %H:%M:%S")
                        .context("Failed to parse updated_at as SQLite format")?
                        .and_utc()
                };

                Ok(Some(User {
                    id: Some(id),
                    email,
                    username,
                    password_hash,
                    is_active,
                    is_admin,
                    created_at,
                    updated_at,
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn update(&self, user: &User) -> Result<()> {
        if user.id.is_none() {
            return Err(anyhow::anyhow!("Cannot update user without id"));
        }

        if user.is_valid().is_err() {
            return Err(anyhow::anyhow!("Invalid user: {:?}", user.is_valid().err()));
        }

        let rows_affected = sqlx::query(
            r#"
            UPDATE users
            SET email = ?, username = ?, password_hash = ?, is_active = ?, is_admin = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&user.email)
        .bind(&user.username)
        .bind(&user.password_hash)
        .bind(user.is_active)
        .bind(user.is_admin)
        .bind(user.updated_at)
        .bind(user.id.ok_or_else(|| anyhow::anyhow!("User has no ID"))?)
        .execute(&self.pool)
        .await
        .context("Failed to update user")?
        .rows_affected();

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("User not found"));
        }

        Ok(())
    }

    pub async fn delete(&self, id: i64) -> Result<()> {
        let rows_affected = sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete user")?
            .rows_affected();

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("User not found"));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

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

        Ok(())
    }

    #[sqlx::test]
    async fn test_new_creates_repository() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;

        let repo = UserRepository::new(pool.clone());

        // Verify we can access the pool by doing a simple query
        let _result = sqlx::query("SELECT 1").fetch_one(&repo.pool).await?;

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_user_success() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = UserRepository::new(pool.clone());
        let user = User::new(
            "test@example.com".to_string(),
            "testuser".to_string(),
            "password123",
        )?;

        let id = repo.create(&user).await?;
        assert!(id > 0);

        // Verify it was created
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE id = ?")
            .bind(id)
            .fetch_one(&pool)
            .await?;
        assert_eq!(count.0, 1);

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_user_duplicate_email_fails() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = UserRepository::new(pool);
        let user1 = User::new(
            "test@example.com".to_string(),
            "user1".to_string(),
            "password123",
        )?;
        let user2 = User::new(
            "test@example.com".to_string(), // Same email
            "user2".to_string(),
            "password456",
        )?;

        // First should succeed
        repo.create(&user1).await?;

        // Second should fail
        let result = repo.create(&user2).await;
        assert!(result.is_err());

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_user_duplicate_username_fails() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = UserRepository::new(pool);
        let user1 = User::new(
            "test1@example.com".to_string(),
            "testuser".to_string(),
            "password123",
        )?;
        let user2 = User::new(
            "test2@example.com".to_string(),
            "testuser".to_string(), // Same username
            "password456",
        )?;

        // First should succeed
        repo.create(&user1).await?;

        // Second should fail
        let result = repo.create(&user2).await;
        assert!(result.is_err());

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_user_invalid_fails() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = UserRepository::new(pool);
        let mut user = User::new(
            "test@example.com".to_string(),
            "testuser".to_string(),
            "password123",
        )?;

        // Make it invalid
        user.email = "invalid-email".to_string();

        let result = repo.create(&user).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid user"));

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_id_existing() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = UserRepository::new(pool);
        let user = User::new(
            "test@example.com".to_string(),
            "testuser".to_string(),
            "password123",
        )?;

        let id = repo.create(&user).await?;

        // Find the user
        let found = repo.find_by_id(id).await?;
        assert!(found.is_some());

        let found_user = found.unwrap();
        assert_eq!(found_user.id, Some(id));
        assert_eq!(found_user.email, user.email);
        assert_eq!(found_user.username, user.username);
        assert_eq!(found_user.password_hash, user.password_hash);
        assert_eq!(found_user.is_active, user.is_active);
        assert_eq!(found_user.is_admin, user.is_admin);

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_id_non_existing() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = UserRepository::new(pool);

        let found = repo.find_by_id(999).await?;
        assert!(found.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_id_zero_and_negative() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = UserRepository::new(pool);

        let found = repo.find_by_id(0).await?;
        assert!(found.is_none());

        let found = repo.find_by_id(-1).await?;
        assert!(found.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_email_existing() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = UserRepository::new(pool);
        let user = User::new(
            "test@example.com".to_string(),
            "testuser".to_string(),
            "password123",
        )?;

        let id = repo.create(&user).await?;

        // Find by email
        let found = repo.find_by_email("test@example.com").await?;
        assert!(found.is_some());

        let found_user = found.unwrap();
        assert_eq!(found_user.id, Some(id));
        assert_eq!(found_user.email, "test@example.com");

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_email_non_existing() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = UserRepository::new(pool);

        let found = repo.find_by_email("nonexistent@example.com").await?;
        assert!(found.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_email_case_sensitive() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = UserRepository::new(pool);
        let user = User::new(
            "test@example.com".to_string(),
            "testuser".to_string(),
            "password123",
        )?;

        repo.create(&user).await?;

        // SQLite is case-insensitive by default for LIKE but case-sensitive for =
        // This test documents the actual behavior
        let found = repo.find_by_email("TEST@EXAMPLE.COM").await?;
        // In SQLite with = operator, this will be None (case-sensitive)
        assert!(found.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_username_existing() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = UserRepository::new(pool);
        let user = User::new(
            "test@example.com".to_string(),
            "testuser".to_string(),
            "password123",
        )?;

        let id = repo.create(&user).await?;

        // Find by username
        let found = repo.find_by_username("testuser").await?;
        assert!(found.is_some());

        let found_user = found.unwrap();
        assert_eq!(found_user.id, Some(id));
        assert_eq!(found_user.username, "testuser");

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_username_non_existing() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = UserRepository::new(pool);

        let found = repo.find_by_username("nonexistent").await?;
        assert!(found.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_update_user_success() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = UserRepository::new(pool);
        let mut user = User::new(
            "test@example.com".to_string(),
            "testuser".to_string(),
            "password123",
        )?;

        let id = repo.create(&user).await?;
        user.id = Some(id);

        // Update fields
        user.email = "newemail@example.com".to_string();
        user.username = "newusername".to_string();
        user.is_admin = true;
        user.updated_at = chrono::Utc::now();

        repo.update(&user).await?;

        // Verify update
        let found = repo.find_by_id(id).await?.unwrap();
        assert_eq!(found.email, "newemail@example.com");
        assert_eq!(found.username, "newusername");
        assert!(found.is_admin);

        Ok(())
    }

    #[sqlx::test]
    async fn test_update_user_without_id_fails() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = UserRepository::new(pool);
        let user = User::new(
            "test@example.com".to_string(),
            "testuser".to_string(),
            "password123",
        )?;

        let result = repo.update(&user).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("without id"));

        Ok(())
    }

    #[sqlx::test]
    async fn test_update_user_invalid_fails() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = UserRepository::new(pool);
        let mut user = User::new(
            "test@example.com".to_string(),
            "testuser".to_string(),
            "password123",
        )?;

        let id = repo.create(&user).await?;
        user.id = Some(id);
        user.email = "invalid-email".to_string(); // Make it invalid

        let result = repo.update(&user).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid user"));

        Ok(())
    }

    #[sqlx::test]
    async fn test_update_user_non_existing_fails() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = UserRepository::new(pool);
        let mut user = User::new(
            "test@example.com".to_string(),
            "testuser".to_string(),
            "password123",
        )?;
        user.id = Some(999); // Non-existent ID

        let result = repo.update(&user).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("User not found"));

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_user_success() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = UserRepository::new(pool);
        let user = User::new(
            "test@example.com".to_string(),
            "testuser".to_string(),
            "password123",
        )?;

        let id = repo.create(&user).await?;

        // Delete the user
        repo.delete(id).await?;

        // Verify it's gone
        let found = repo.find_by_id(id).await?;
        assert!(found.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_user_non_existing_fails() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = UserRepository::new(pool);

        let result = repo.delete(999).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("User not found"));

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_user_cascades_sessions() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        // Create sessions table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                user_id INTEGER NOT NULL,
                expires_at TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&pool)
        .await?;

        let repo = UserRepository::new(pool.clone());
        let user = User::new(
            "test@example.com".to_string(),
            "testuser".to_string(),
            "password123",
        )?;

        let user_id = repo.create(&user).await?;

        // Create a session for the user
        sqlx::query("INSERT INTO sessions (id, user_id, expires_at) VALUES (?, ?, ?)")
            .bind("test-session-id")
            .bind(user_id)
            .bind("2025-01-01 00:00:00")
            .execute(&pool)
            .await?;

        // Delete the user
        repo.delete(user_id).await?;

        // Verify session was cascaded
        let session_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM sessions WHERE user_id = ?")
                .bind(user_id)
                .fetch_one(&pool)
                .await?;
        assert_eq!(session_count.0, 0);

        Ok(())
    }
}
