use anyhow::{Context, Result};
use doxyde_core::models::session::Session;
use sqlx::SqlitePool;

pub struct SessionRepository {
    pool: SqlitePool,
}

impl SessionRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, session: &Session) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO sessions (id, user_id, expires_at, created_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(&session.id)
        .bind(session.user_id)
        .bind(session.expires_at)
        .bind(session.created_at)
        .execute(&self.pool)
        .await
        .context("Failed to create session")?;

        Ok(())
    }

    pub async fn find_by_id(&self, id: &str) -> Result<Option<Session>> {
        let row = sqlx::query_as::<_, (String, i64, String, String)>(
            r#"
            SELECT id, user_id, expires_at, created_at
            FROM sessions
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to find session by id")?;

        match row {
            Some((id, user_id, expires_at_str, created_at_str)) => {
                // Parse datetime strings
                let expires_at = if expires_at_str.contains('T') {
                    chrono::DateTime::parse_from_rfc3339(&expires_at_str)
                        .context("Failed to parse expires_at as RFC3339")?
                        .with_timezone(&chrono::Utc)
                } else {
                    chrono::NaiveDateTime::parse_from_str(&expires_at_str, "%Y-%m-%d %H:%M:%S")
                        .context("Failed to parse expires_at as SQLite format")?
                        .and_utc()
                };

                let created_at = if created_at_str.contains('T') {
                    chrono::DateTime::parse_from_rfc3339(&created_at_str)
                        .context("Failed to parse created_at as RFC3339")?
                        .with_timezone(&chrono::Utc)
                } else {
                    chrono::NaiveDateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S")
                        .context("Failed to parse created_at as SQLite format")?
                        .and_utc()
                };

                Ok(Some(Session {
                    id,
                    user_id,
                    expires_at,
                    created_at,
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn find_by_user_id(&self, user_id: i64) -> Result<Vec<Session>> {
        let rows = sqlx::query_as::<_, (String, i64, String, String)>(
            r#"
            SELECT id, user_id, expires_at, created_at
            FROM sessions
            WHERE user_id = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to find sessions by user_id")?;

        let mut sessions = Vec::new();
        for (id, user_id, expires_at_str, created_at_str) in rows {
            // Parse datetime strings
            let expires_at = if expires_at_str.contains('T') {
                chrono::DateTime::parse_from_rfc3339(&expires_at_str)
                    .context("Failed to parse expires_at as RFC3339")?
                    .with_timezone(&chrono::Utc)
            } else {
                chrono::NaiveDateTime::parse_from_str(&expires_at_str, "%Y-%m-%d %H:%M:%S")
                    .context("Failed to parse expires_at as SQLite format")?
                    .and_utc()
            };

            let created_at = if created_at_str.contains('T') {
                chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .context("Failed to parse created_at as RFC3339")?
                    .with_timezone(&chrono::Utc)
            } else {
                chrono::NaiveDateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S")
                    .context("Failed to parse created_at as SQLite format")?
                    .and_utc()
            };

            sessions.push(Session {
                id,
                user_id,
                expires_at,
                created_at,
            });
        }

        Ok(sessions)
    }

    pub async fn delete_expired(&self) -> Result<u64> {
        // Get current time in RFC3339 format for comparison
        let now = chrono::Utc::now().to_rfc3339();

        let result = sqlx::query(
            r#"
            DELETE FROM sessions
            WHERE expires_at < ?
            "#,
        )
        .bind(now)
        .execute(&self.pool)
        .await
        .context("Failed to delete expired sessions")?;

        Ok(result.rows_affected())
    }

    pub async fn delete(&self, id: &str) -> Result<()> {
        let rows_affected = sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete session")?
            .rows_affected();

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Session not found"));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn setup_test_db(pool: &SqlitePool) -> Result<()> {
        // Create users table first (sessions depend on it)
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
        .execute(pool)
        .await?;

        Ok(())
    }

    #[sqlx::test]
    async fn test_new_creates_repository() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;

        let repo = SessionRepository::new(pool.clone());

        // Verify we can access the pool by doing a simple query
        let _result = sqlx::query("SELECT 1").fetch_one(&repo.pool).await?;

        Ok(())
    }

    async fn create_test_user(pool: &SqlitePool) -> Result<i64> {
        let result =
            sqlx::query("INSERT INTO users (email, username, password_hash) VALUES (?, ?, ?)")
                .bind("test@example.com")
                .bind("testuser")
                .bind("hashed_password")
                .execute(pool)
                .await?;

        Ok(result.last_insert_rowid())
    }

    #[sqlx::test]
    async fn test_create_session_success() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let user_id = create_test_user(&pool).await?;

        let repo = SessionRepository::new(pool.clone());
        let session = Session::new(user_id);

        repo.create(&session).await?;

        // Verify it was created
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sessions WHERE id = ?")
            .bind(&session.id)
            .fetch_one(&pool)
            .await?;
        assert_eq!(count.0, 1);

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_session_duplicate_id_fails() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let user_id = create_test_user(&pool).await?;

        let repo = SessionRepository::new(pool);
        let session = Session::new(user_id);

        // First should succeed
        repo.create(&session).await?;

        // Second with same ID should fail
        let result = repo.create(&session).await;
        assert!(result.is_err());

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_session_invalid_user_fails() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SessionRepository::new(pool);
        let session = Session::new(999); // Non-existent user

        let result = repo.create(&session).await;
        assert!(result.is_err());

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_id_existing() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let user_id = create_test_user(&pool).await?;

        let repo = SessionRepository::new(pool.clone());
        let session = Session::new(user_id);

        // Create the session
        repo.create(&session).await?;

        // Find it
        let found = repo.find_by_id(&session.id).await?;
        assert!(found.is_some());

        let found_session = found.unwrap();
        assert_eq!(found_session.id, session.id);
        assert_eq!(found_session.user_id, session.user_id);
        assert_eq!(found_session.expires_at, session.expires_at);

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_id_non_existing() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SessionRepository::new(pool);

        let found = repo.find_by_id("non-existent-session-id").await?;
        assert!(found.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_id_empty_string() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SessionRepository::new(pool);

        let found = repo.find_by_id("").await?;
        assert!(found.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_id_with_sqlite_datetime_format() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let user_id = create_test_user(&pool).await?;

        // Manually insert with SQLite datetime format
        sqlx::query(
            r#"
            INSERT INTO sessions (id, user_id, expires_at, created_at)
            VALUES (?, ?, datetime('now', '+1 hour'), datetime('now'))
            "#,
        )
        .bind("test-session-id")
        .bind(user_id)
        .execute(&pool)
        .await?;

        let repo = SessionRepository::new(pool);
        let found = repo.find_by_id("test-session-id").await?;

        assert!(found.is_some());
        let session = found.unwrap();
        assert_eq!(session.id, "test-session-id");
        assert_eq!(session.user_id, user_id);

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_id_with_rfc3339_datetime_format() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let user_id = create_test_user(&pool).await?;

        // Manually insert with RFC3339 datetime format
        let expires_at = chrono::Utc::now() + chrono::Duration::hours(1);
        let created_at = chrono::Utc::now();

        sqlx::query(
            r#"
            INSERT INTO sessions (id, user_id, expires_at, created_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind("test-session-id")
        .bind(user_id)
        .bind(expires_at.to_rfc3339())
        .bind(created_at.to_rfc3339())
        .execute(&pool)
        .await?;

        let repo = SessionRepository::new(pool);
        let found = repo.find_by_id("test-session-id").await?;

        assert!(found.is_some());
        let session = found.unwrap();
        assert_eq!(session.id, "test-session-id");
        assert_eq!(session.user_id, user_id);

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_user_id_multiple_sessions() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let user_id = create_test_user(&pool).await?;

        let repo = SessionRepository::new(pool.clone());

        // Create multiple sessions
        let session1 = Session::new(user_id);
        let session2 = Session::new(user_id);
        let session3 = Session::new(user_id);

        repo.create(&session1).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await; // Small delay to ensure different timestamps
        repo.create(&session2).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        repo.create(&session3).await?;

        // Find all sessions for the user
        let sessions = repo.find_by_user_id(user_id).await?;
        assert_eq!(sessions.len(), 3);

        // Should be ordered by created_at DESC (newest first)
        assert_eq!(sessions[0].id, session3.id);
        assert_eq!(sessions[1].id, session2.id);
        assert_eq!(sessions[2].id, session1.id);

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_user_id_no_sessions() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let user_id = create_test_user(&pool).await?;

        let repo = SessionRepository::new(pool);

        // Find sessions for user with no sessions
        let sessions = repo.find_by_user_id(user_id).await?;
        assert_eq!(sessions.len(), 0);

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_user_id_non_existent_user() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SessionRepository::new(pool);

        // Find sessions for non-existent user
        let sessions = repo.find_by_user_id(999).await?;
        assert_eq!(sessions.len(), 0);

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_user_id_other_users_sessions_not_included() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        // Create two users
        let user1_id = create_test_user(&pool).await?;

        let result =
            sqlx::query("INSERT INTO users (email, username, password_hash) VALUES (?, ?, ?)")
                .bind("test2@example.com")
                .bind("testuser2")
                .bind("hashed_password")
                .execute(&pool)
                .await?;
        let user2_id = result.last_insert_rowid();

        let repo = SessionRepository::new(pool.clone());

        // Create sessions for both users
        let session1_user1 = Session::new(user1_id);
        let session2_user1 = Session::new(user1_id);
        let session_user2 = Session::new(user2_id);

        repo.create(&session1_user1).await?;
        repo.create(&session2_user1).await?;
        repo.create(&session_user2).await?;

        // Find sessions for user1
        let sessions = repo.find_by_user_id(user1_id).await?;
        assert_eq!(sessions.len(), 2);

        // Verify they're only user1's sessions
        assert!(sessions.iter().all(|s| s.user_id == user1_id));
        assert!(!sessions.iter().any(|s| s.id == session_user2.id));

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_expired_removes_expired_sessions() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let user_id = create_test_user(&pool).await?;

        let repo = SessionRepository::new(pool.clone());

        // Create sessions with different expiration times
        let now = chrono::Utc::now();

        // Expired session (1 hour ago)
        sqlx::query(
            r#"
            INSERT INTO sessions (id, user_id, expires_at, created_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind("expired-session")
        .bind(user_id)
        .bind((now - chrono::Duration::hours(1)).to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&pool)
        .await?;

        // Valid session (expires in 1 hour)
        sqlx::query(
            r#"
            INSERT INTO sessions (id, user_id, expires_at, created_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind("valid-session")
        .bind(user_id)
        .bind((now + chrono::Duration::hours(1)).to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&pool)
        .await?;

        // Delete expired sessions
        let deleted_count = repo.delete_expired().await?;
        assert_eq!(deleted_count, 1);

        // Verify expired session is gone
        let expired = repo.find_by_id("expired-session").await?;
        assert!(expired.is_none());

        // Verify valid session still exists
        let valid = repo.find_by_id("valid-session").await?;
        assert!(valid.is_some());

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_expired_no_expired_sessions() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let user_id = create_test_user(&pool).await?;

        let repo = SessionRepository::new(pool.clone());

        // Create only valid sessions
        let session1 = Session::new(user_id);
        let session2 = Session::new(user_id);

        repo.create(&session1).await?;
        repo.create(&session2).await?;

        // Delete expired sessions (should be none)
        let deleted_count = repo.delete_expired().await?;
        assert_eq!(deleted_count, 0);

        // Verify all sessions still exist
        let sessions = repo.find_by_user_id(user_id).await?;
        assert_eq!(sessions.len(), 2);

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_expired_multiple_expired() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let user_id = create_test_user(&pool).await?;

        let repo = SessionRepository::new(pool.clone());

        // Create multiple expired sessions
        let now = chrono::Utc::now();

        for i in 1..=3 {
            sqlx::query(
                r#"
                INSERT INTO sessions (id, user_id, expires_at, created_at)
                VALUES (?, ?, ?, ?)
                "#,
            )
            .bind(format!("expired-session-{}", i))
            .bind(user_id)
            .bind((now - chrono::Duration::hours(i as i64)).to_rfc3339())
            .bind(now.to_rfc3339())
            .execute(&pool)
            .await?;
        }

        // Create one valid session
        repo.create(&Session::new(user_id)).await?;

        // Delete expired sessions
        let deleted_count = repo.delete_expired().await?;
        assert_eq!(deleted_count, 3);

        // Verify only valid session remains
        let sessions = repo.find_by_user_id(user_id).await?;
        assert_eq!(sessions.len(), 1);

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_session_success() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let user_id = create_test_user(&pool).await?;

        let repo = SessionRepository::new(pool.clone());
        let session = Session::new(user_id);

        // Create the session
        repo.create(&session).await?;

        // Delete it
        repo.delete(&session.id).await?;

        // Verify it's gone
        let found = repo.find_by_id(&session.id).await?;
        assert!(found.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_session_non_existing_fails() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SessionRepository::new(pool);

        let result = repo.delete("non-existent-session").await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Session not found"));

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_session_empty_id_fails() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SessionRepository::new(pool);

        let result = repo.delete("").await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Session not found"));

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_session_only_deletes_specified() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let user_id = create_test_user(&pool).await?;

        let repo = SessionRepository::new(pool.clone());
        let session1 = Session::new(user_id);
        let session2 = Session::new(user_id);

        // Create two sessions
        repo.create(&session1).await?;
        repo.create(&session2).await?;

        // Delete only the first one
        repo.delete(&session1.id).await?;

        // Verify first is gone
        let found1 = repo.find_by_id(&session1.id).await?;
        assert!(found1.is_none());

        // Verify second still exists
        let found2 = repo.find_by_id(&session2.id).await?;
        assert!(found2.is_some());

        Ok(())
    }
}
