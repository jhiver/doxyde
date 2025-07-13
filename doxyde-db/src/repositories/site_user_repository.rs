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
use doxyde_core::models::permission::SiteUser;
use sqlx::SqlitePool;

pub struct SiteUserRepository {
    pool: SqlitePool,
}

impl SiteUserRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, site_user: &SiteUser) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO site_users (site_id, user_id, role, created_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(site_user.site_id)
        .bind(site_user.user_id)
        .bind(site_user.role.as_str())
        .bind(site_user.created_at)
        .execute(&self.pool)
        .await
        .context("Failed to create site_user")?;

        Ok(())
    }

    pub async fn find_by_site_and_user(
        &self,
        site_id: i64,
        user_id: i64,
    ) -> Result<Option<SiteUser>> {
        let row = sqlx::query_as::<_, (i64, i64, String, String)>(
            r#"
            SELECT site_id, user_id, role, created_at
            FROM site_users
            WHERE site_id = ? AND user_id = ?
            "#,
        )
        .bind(site_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to find site_user")?;

        match row {
            Some((site_id, user_id, role_str, created_at_str)) => {
                // Parse role
                let role = match role_str.as_str() {
                    "viewer" => doxyde_core::models::permission::SiteRole::Viewer,
                    "editor" => doxyde_core::models::permission::SiteRole::Editor,
                    "owner" => doxyde_core::models::permission::SiteRole::Owner,
                    _ => return Err(anyhow::anyhow!("Invalid role: {}", role_str)),
                };

                // Parse datetime
                let created_at = if created_at_str.contains('T') {
                    chrono::DateTime::parse_from_rfc3339(&created_at_str)
                        .context("Failed to parse created_at as RFC3339")?
                        .with_timezone(&chrono::Utc)
                } else {
                    chrono::NaiveDateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S")
                        .context("Failed to parse created_at as SQLite format")?
                        .and_utc()
                };

                Ok(Some(SiteUser {
                    site_id,
                    user_id,
                    role,
                    created_at,
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn list_by_site(&self, site_id: i64) -> Result<Vec<SiteUser>> {
        let rows = sqlx::query_as::<_, (i64, i64, String, String)>(
            r#"
            SELECT site_id, user_id, role, created_at
            FROM site_users
            WHERE site_id = ?
            ORDER BY 
                CASE role 
                    WHEN 'owner' THEN 3
                    WHEN 'editor' THEN 2
                    WHEN 'viewer' THEN 1
                END DESC,
                user_id ASC
            "#,
        )
        .bind(site_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list site_users by site")?;

        let mut site_users = Vec::new();
        for (site_id, user_id, role_str, created_at_str) in rows {
            // Parse role
            let role = match role_str.as_str() {
                "viewer" => doxyde_core::models::permission::SiteRole::Viewer,
                "editor" => doxyde_core::models::permission::SiteRole::Editor,
                "owner" => doxyde_core::models::permission::SiteRole::Owner,
                _ => return Err(anyhow::anyhow!("Invalid role: {}", role_str)),
            };

            // Parse datetime
            let created_at = if created_at_str.contains('T') {
                chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .context("Failed to parse created_at as RFC3339")?
                    .with_timezone(&chrono::Utc)
            } else {
                chrono::NaiveDateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S")
                    .context("Failed to parse created_at as SQLite format")?
                    .and_utc()
            };

            site_users.push(SiteUser {
                site_id,
                user_id,
                role,
                created_at,
            });
        }

        Ok(site_users)
    }

    pub async fn list_by_user(&self, user_id: i64) -> Result<Vec<SiteUser>> {
        let rows = sqlx::query_as::<_, (i64, i64, String, String)>(
            r#"
            SELECT site_id, user_id, role, created_at
            FROM site_users
            WHERE user_id = ?
            ORDER BY 
                CASE role 
                    WHEN 'owner' THEN 3
                    WHEN 'editor' THEN 2
                    WHEN 'viewer' THEN 1
                END DESC,
                site_id ASC
            "#,
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list site_users by user")?;

        let mut site_users = Vec::new();
        for (site_id, user_id, role_str, created_at_str) in rows {
            // Parse role
            let role = match role_str.as_str() {
                "viewer" => doxyde_core::models::permission::SiteRole::Viewer,
                "editor" => doxyde_core::models::permission::SiteRole::Editor,
                "owner" => doxyde_core::models::permission::SiteRole::Owner,
                _ => return Err(anyhow::anyhow!("Invalid role: {}", role_str)),
            };

            // Parse datetime
            let created_at = if created_at_str.contains('T') {
                chrono::DateTime::parse_from_rfc3339(&created_at_str)
                    .context("Failed to parse created_at as RFC3339")?
                    .with_timezone(&chrono::Utc)
            } else {
                chrono::NaiveDateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S")
                    .context("Failed to parse created_at as SQLite format")?
                    .and_utc()
            };

            site_users.push(SiteUser {
                site_id,
                user_id,
                role,
                created_at,
            });
        }

        Ok(site_users)
    }

    pub async fn update_role(
        &self,
        site_id: i64,
        user_id: i64,
        new_role: doxyde_core::models::permission::SiteRole,
    ) -> Result<()> {
        let rows_affected = sqlx::query(
            r#"
            UPDATE site_users
            SET role = ?
            WHERE site_id = ? AND user_id = ?
            "#,
        )
        .bind(new_role.as_str())
        .bind(site_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .context("Failed to update site_user role")?
        .rows_affected();

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Site user association not found"));
        }

        Ok(())
    }

    pub async fn delete(&self, site_id: i64, user_id: i64) -> Result<()> {
        let rows_affected = sqlx::query("DELETE FROM site_users WHERE site_id = ? AND user_id = ?")
            .bind(site_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .context("Failed to delete site_user")?
            .rows_affected();

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Site user association not found"));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    async fn setup_test_db(pool: &SqlitePool) -> Result<()> {
        // Create users table first
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

        // Create site_users table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS site_users (
                site_id INTEGER NOT NULL,
                user_id INTEGER NOT NULL,
                role TEXT NOT NULL DEFAULT 'viewer',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                PRIMARY KEY (site_id, user_id),
                FOREIGN KEY (site_id) REFERENCES sites(id) ON DELETE CASCADE,
                FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
                CHECK (role IN ('viewer', 'editor', 'owner'))
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

        let repo = SiteUserRepository::new(pool.clone());

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

    async fn create_test_site(pool: &SqlitePool) -> Result<i64> {
        let result = sqlx::query("INSERT INTO sites (domain, title) VALUES (?, ?)")
            .bind("example.com")
            .bind("Example Site")
            .execute(pool)
            .await?;

        Ok(result.last_insert_rowid())
    }

    #[sqlx::test]
    async fn test_create_site_user_success() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let user_id = create_test_user(&pool).await?;
        let site_id = create_test_site(&pool).await?;

        let repo = SiteUserRepository::new(pool.clone());
        let site_user = SiteUser::new(
            site_id,
            user_id,
            doxyde_core::models::permission::SiteRole::Editor,
        );

        repo.create(&site_user).await?;

        // Verify it was created
        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM site_users WHERE site_id = ? AND user_id = ?")
                .bind(site_id)
                .bind(user_id)
                .fetch_one(&pool)
                .await?;
        assert_eq!(count.0, 1);

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_site_user_duplicate_fails() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let user_id = create_test_user(&pool).await?;
        let site_id = create_test_site(&pool).await?;

        let repo = SiteUserRepository::new(pool);
        let site_user = SiteUser::new(
            site_id,
            user_id,
            doxyde_core::models::permission::SiteRole::Viewer,
        );

        // First should succeed
        repo.create(&site_user).await?;

        // Second should fail (duplicate primary key)
        let result = repo.create(&site_user).await;
        assert!(result.is_err());

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_site_user_invalid_site_fails() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let user_id = create_test_user(&pool).await?;

        let repo = SiteUserRepository::new(pool);
        let site_user = SiteUser::new(
            999,
            user_id,
            doxyde_core::models::permission::SiteRole::Owner,
        ); // Non-existent site

        let result = repo.create(&site_user).await;
        assert!(result.is_err());

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_site_user_invalid_user_fails() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let site_id = create_test_site(&pool).await?;

        let repo = SiteUserRepository::new(pool);
        let site_user = SiteUser::new(
            site_id,
            999,
            doxyde_core::models::permission::SiteRole::Editor,
        ); // Non-existent user

        let result = repo.create(&site_user).await;
        assert!(result.is_err());

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_site_user_all_roles() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let site_id = create_test_site(&pool).await?;

        let repo = SiteUserRepository::new(pool.clone());

        // Test all three roles
        let roles = [
            doxyde_core::models::permission::SiteRole::Viewer,
            doxyde_core::models::permission::SiteRole::Editor,
            doxyde_core::models::permission::SiteRole::Owner,
        ];

        for (i, role) in roles.iter().enumerate() {
            let result =
                sqlx::query("INSERT INTO users (email, username, password_hash) VALUES (?, ?, ?)")
                    .bind(format!("user{}@example.com", i))
                    .bind(format!("user{}", i))
                    .bind("hashed_password")
                    .execute(&pool)
                    .await?;
            let user_id = result.last_insert_rowid();

            let site_user = SiteUser::new(site_id, user_id, *role);
            repo.create(&site_user).await?;

            // Verify role was saved correctly
            let saved_role: (String,) =
                sqlx::query_as("SELECT role FROM site_users WHERE site_id = ? AND user_id = ?")
                    .bind(site_id)
                    .bind(user_id)
                    .fetch_one(&pool)
                    .await?;
            assert_eq!(saved_role.0, role.as_str());
        }

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_site_and_user_existing() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let user_id = create_test_user(&pool).await?;
        let site_id = create_test_site(&pool).await?;

        let repo = SiteUserRepository::new(pool.clone());
        let site_user = SiteUser::new(
            site_id,
            user_id,
            doxyde_core::models::permission::SiteRole::Editor,
        );

        // Create the association
        repo.create(&site_user).await?;

        // Find it
        let found = repo.find_by_site_and_user(site_id, user_id).await?;
        assert!(found.is_some());

        let found_su = found.unwrap();
        assert_eq!(found_su.site_id, site_id);
        assert_eq!(found_su.user_id, user_id);
        assert_eq!(
            found_su.role,
            doxyde_core::models::permission::SiteRole::Editor
        );

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_site_and_user_non_existing() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let user_id = create_test_user(&pool).await?;
        let site_id = create_test_site(&pool).await?;

        let repo = SiteUserRepository::new(pool);

        // Try to find non-existing association
        let found = repo.find_by_site_and_user(site_id, user_id).await?;
        assert!(found.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_site_and_user_wrong_site() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let user_id = create_test_user(&pool).await?;
        let site_id = create_test_site(&pool).await?;

        // Create second site
        let result = sqlx::query("INSERT INTO sites (domain, title) VALUES (?, ?)")
            .bind("other.com")
            .bind("Other Site")
            .execute(&pool)
            .await?;
        let other_site_id = result.last_insert_rowid();

        let repo = SiteUserRepository::new(pool.clone());
        let site_user = SiteUser::new(
            site_id,
            user_id,
            doxyde_core::models::permission::SiteRole::Owner,
        );

        // Create association with first site
        repo.create(&site_user).await?;

        // Try to find with wrong site
        let found = repo.find_by_site_and_user(other_site_id, user_id).await?;
        assert!(found.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_site_and_user_wrong_user() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let user_id = create_test_user(&pool).await?;
        let site_id = create_test_site(&pool).await?;

        // Create second user
        let result =
            sqlx::query("INSERT INTO users (email, username, password_hash) VALUES (?, ?, ?)")
                .bind("other@example.com")
                .bind("otheruser")
                .bind("hashed_password")
                .execute(&pool)
                .await?;
        let other_user_id = result.last_insert_rowid();

        let repo = SiteUserRepository::new(pool.clone());
        let site_user = SiteUser::new(
            site_id,
            user_id,
            doxyde_core::models::permission::SiteRole::Viewer,
        );

        // Create association with first user
        repo.create(&site_user).await?;

        // Try to find with wrong user
        let found = repo.find_by_site_and_user(site_id, other_user_id).await?;
        assert!(found.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_site_and_user_with_sqlite_datetime() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let user_id = create_test_user(&pool).await?;
        let site_id = create_test_site(&pool).await?;

        // Insert with SQLite datetime format
        sqlx::query(
            r#"
            INSERT INTO site_users (site_id, user_id, role, created_at)
            VALUES (?, ?, ?, datetime('now'))
            "#,
        )
        .bind(site_id)
        .bind(user_id)
        .bind("owner")
        .execute(&pool)
        .await?;

        let repo = SiteUserRepository::new(pool);
        let found = repo.find_by_site_and_user(site_id, user_id).await?;

        assert!(found.is_some());
        let site_user = found.unwrap();
        assert_eq!(
            site_user.role,
            doxyde_core::models::permission::SiteRole::Owner
        );

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_by_site_empty() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let site_id = create_test_site(&pool).await?;

        let repo = SiteUserRepository::new(pool);
        let users = repo.list_by_site(site_id).await?;

        assert_eq!(users.len(), 0);

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_by_site_single_user() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let user_id = create_test_user(&pool).await?;
        let site_id = create_test_site(&pool).await?;

        let repo = SiteUserRepository::new(pool.clone());
        let site_user = SiteUser::new(
            site_id,
            user_id,
            doxyde_core::models::permission::SiteRole::Editor,
        );
        repo.create(&site_user).await?;

        let users = repo.list_by_site(site_id).await?;

        assert_eq!(users.len(), 1);
        assert_eq!(users[0].user_id, user_id);
        assert_eq!(
            users[0].role,
            doxyde_core::models::permission::SiteRole::Editor
        );

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_by_site_multiple_users_ordered() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let site_id = create_test_site(&pool).await?;

        let repo = SiteUserRepository::new(pool.clone());

        // Create users with different roles
        let mut user_ids = Vec::new();
        for i in 0..3 {
            let result =
                sqlx::query("INSERT INTO users (email, username, password_hash) VALUES (?, ?, ?)")
                    .bind(format!("user{}@example.com", i))
                    .bind(format!("user{}", i))
                    .bind("hashed_password")
                    .execute(&pool)
                    .await?;
            user_ids.push(result.last_insert_rowid());
        }

        // Add users with different roles
        let site_user1 = SiteUser::new(
            site_id,
            user_ids[0],
            doxyde_core::models::permission::SiteRole::Viewer,
        );
        let site_user2 = SiteUser::new(
            site_id,
            user_ids[1],
            doxyde_core::models::permission::SiteRole::Owner,
        );
        let site_user3 = SiteUser::new(
            site_id,
            user_ids[2],
            doxyde_core::models::permission::SiteRole::Editor,
        );

        repo.create(&site_user1).await?;
        repo.create(&site_user2).await?;
        repo.create(&site_user3).await?;

        let users = repo.list_by_site(site_id).await?;

        assert_eq!(users.len(), 3);

        // Should be ordered by role DESC (Owner > Editor > Viewer), then by user_id ASC
        assert_eq!(
            users[0].role,
            doxyde_core::models::permission::SiteRole::Owner
        );
        assert_eq!(
            users[1].role,
            doxyde_core::models::permission::SiteRole::Editor
        );
        assert_eq!(
            users[2].role,
            doxyde_core::models::permission::SiteRole::Viewer
        );

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_by_site_non_existent_site() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteUserRepository::new(pool);
        let users = repo.list_by_site(999).await?;

        assert_eq!(users.len(), 0);

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_by_site_excludes_other_sites() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let user_id = create_test_user(&pool).await?;
        let site1_id = create_test_site(&pool).await?;

        // Create second site
        let result = sqlx::query("INSERT INTO sites (domain, title) VALUES (?, ?)")
            .bind("other.com")
            .bind("Other Site")
            .execute(&pool)
            .await?;
        let site2_id = result.last_insert_rowid();

        let repo = SiteUserRepository::new(pool.clone());

        // Add user to both sites
        let site_user1 = SiteUser::new(
            site1_id,
            user_id,
            doxyde_core::models::permission::SiteRole::Owner,
        );
        let site_user2 = SiteUser::new(
            site2_id,
            user_id,
            doxyde_core::models::permission::SiteRole::Viewer,
        );

        repo.create(&site_user1).await?;
        repo.create(&site_user2).await?;

        // List users for site1
        let users = repo.list_by_site(site1_id).await?;

        assert_eq!(users.len(), 1);
        assert_eq!(users[0].site_id, site1_id);
        assert_eq!(
            users[0].role,
            doxyde_core::models::permission::SiteRole::Owner
        );

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_by_user_empty() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let user_id = create_test_user(&pool).await?;

        let repo = SiteUserRepository::new(pool);
        let sites = repo.list_by_user(user_id).await?;

        assert_eq!(sites.len(), 0);

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_by_user_single_site() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let user_id = create_test_user(&pool).await?;
        let site_id = create_test_site(&pool).await?;

        let repo = SiteUserRepository::new(pool.clone());
        let site_user = SiteUser::new(
            site_id,
            user_id,
            doxyde_core::models::permission::SiteRole::Viewer,
        );
        repo.create(&site_user).await?;

        let sites = repo.list_by_user(user_id).await?;

        assert_eq!(sites.len(), 1);
        assert_eq!(sites[0].site_id, site_id);
        assert_eq!(
            sites[0].role,
            doxyde_core::models::permission::SiteRole::Viewer
        );

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_by_user_multiple_sites_ordered() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let user_id = create_test_user(&pool).await?;

        let repo = SiteUserRepository::new(pool.clone());

        // Create multiple sites
        let mut site_ids = Vec::new();
        for i in 0..3 {
            let result = sqlx::query("INSERT INTO sites (domain, title) VALUES (?, ?)")
                .bind(format!("site{}.com", i))
                .bind(format!("Site {}", i))
                .execute(&pool)
                .await?;
            site_ids.push(result.last_insert_rowid());
        }

        // Add user to sites with different roles
        let site_user1 = SiteUser::new(
            site_ids[0],
            user_id,
            doxyde_core::models::permission::SiteRole::Editor,
        );
        let site_user2 = SiteUser::new(
            site_ids[1],
            user_id,
            doxyde_core::models::permission::SiteRole::Viewer,
        );
        let site_user3 = SiteUser::new(
            site_ids[2],
            user_id,
            doxyde_core::models::permission::SiteRole::Owner,
        );

        repo.create(&site_user1).await?;
        repo.create(&site_user2).await?;
        repo.create(&site_user3).await?;

        let sites = repo.list_by_user(user_id).await?;

        assert_eq!(sites.len(), 3);

        // Should be ordered by role DESC (Owner > Editor > Viewer), then by site_id ASC
        assert_eq!(
            sites[0].role,
            doxyde_core::models::permission::SiteRole::Owner
        );
        assert_eq!(
            sites[1].role,
            doxyde_core::models::permission::SiteRole::Editor
        );
        assert_eq!(
            sites[2].role,
            doxyde_core::models::permission::SiteRole::Viewer
        );

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_by_user_excludes_other_users() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let user1_id = create_test_user(&pool).await?;
        let site_id = create_test_site(&pool).await?;

        // Create second user
        let result =
            sqlx::query("INSERT INTO users (email, username, password_hash) VALUES (?, ?, ?)")
                .bind("user2@example.com")
                .bind("user2")
                .bind("hashed_password")
                .execute(&pool)
                .await?;
        let user2_id = result.last_insert_rowid();

        let repo = SiteUserRepository::new(pool.clone());

        // Add both users to the site
        let site_user1 = SiteUser::new(
            site_id,
            user1_id,
            doxyde_core::models::permission::SiteRole::Owner,
        );
        let site_user2 = SiteUser::new(
            site_id,
            user2_id,
            doxyde_core::models::permission::SiteRole::Editor,
        );

        repo.create(&site_user1).await?;
        repo.create(&site_user2).await?;

        // List sites for user1
        let sites = repo.list_by_user(user1_id).await?;

        assert_eq!(sites.len(), 1);
        assert_eq!(sites[0].user_id, user1_id);
        assert_eq!(
            sites[0].role,
            doxyde_core::models::permission::SiteRole::Owner
        );

        Ok(())
    }

    #[sqlx::test]
    async fn test_update_role_success() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let user_id = create_test_user(&pool).await?;
        let site_id = create_test_site(&pool).await?;

        let repo = SiteUserRepository::new(pool.clone());

        // Create with viewer role
        let site_user = SiteUser::new(
            site_id,
            user_id,
            doxyde_core::models::permission::SiteRole::Viewer,
        );
        repo.create(&site_user).await?;

        // Update to editor role
        repo.update_role(
            site_id,
            user_id,
            doxyde_core::models::permission::SiteRole::Editor,
        )
        .await?;

        // Verify it was updated
        let found = repo.find_by_site_and_user(site_id, user_id).await?;
        assert!(found.is_some());
        assert_eq!(
            found.unwrap().role,
            doxyde_core::models::permission::SiteRole::Editor
        );

        Ok(())
    }

    #[sqlx::test]
    async fn test_update_role_non_existing_fails() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let user_id = create_test_user(&pool).await?;
        let site_id = create_test_site(&pool).await?;

        let repo = SiteUserRepository::new(pool);

        // Try to update non-existing association
        let result = repo
            .update_role(
                site_id,
                user_id,
                doxyde_core::models::permission::SiteRole::Owner,
            )
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));

        Ok(())
    }

    #[sqlx::test]
    async fn test_update_role_all_transitions() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let user_id = create_test_user(&pool).await?;
        let site_id = create_test_site(&pool).await?;

        let repo = SiteUserRepository::new(pool.clone());

        // Start with viewer
        let site_user = SiteUser::new(
            site_id,
            user_id,
            doxyde_core::models::permission::SiteRole::Viewer,
        );
        repo.create(&site_user).await?;

        // Viewer -> Editor
        repo.update_role(
            site_id,
            user_id,
            doxyde_core::models::permission::SiteRole::Editor,
        )
        .await?;
        let found = repo.find_by_site_and_user(site_id, user_id).await?.unwrap();
        assert_eq!(
            found.role,
            doxyde_core::models::permission::SiteRole::Editor
        );

        // Editor -> Owner
        repo.update_role(
            site_id,
            user_id,
            doxyde_core::models::permission::SiteRole::Owner,
        )
        .await?;
        let found = repo.find_by_site_and_user(site_id, user_id).await?.unwrap();
        assert_eq!(found.role, doxyde_core::models::permission::SiteRole::Owner);

        // Owner -> Viewer (downgrade)
        repo.update_role(
            site_id,
            user_id,
            doxyde_core::models::permission::SiteRole::Viewer,
        )
        .await?;
        let found = repo.find_by_site_and_user(site_id, user_id).await?.unwrap();
        assert_eq!(
            found.role,
            doxyde_core::models::permission::SiteRole::Viewer
        );

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_site_user_success() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let user_id = create_test_user(&pool).await?;
        let site_id = create_test_site(&pool).await?;

        let repo = SiteUserRepository::new(pool.clone());
        let site_user = SiteUser::new(
            site_id,
            user_id,
            doxyde_core::models::permission::SiteRole::Editor,
        );

        // Create the association
        repo.create(&site_user).await?;

        // Delete it
        repo.delete(site_id, user_id).await?;

        // Verify it's gone
        let found = repo.find_by_site_and_user(site_id, user_id).await?;
        assert!(found.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_site_user_non_existing_fails() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let user_id = create_test_user(&pool).await?;
        let site_id = create_test_site(&pool).await?;

        let repo = SiteUserRepository::new(pool);

        // Try to delete non-existing association
        let result = repo.delete(site_id, user_id).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_site_user_only_deletes_specified() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let user1_id = create_test_user(&pool).await?;
        let site_id = create_test_site(&pool).await?;

        // Create second user
        let result =
            sqlx::query("INSERT INTO users (email, username, password_hash) VALUES (?, ?, ?)")
                .bind("user2@example.com")
                .bind("user2")
                .bind("hashed_password")
                .execute(&pool)
                .await?;
        let user2_id = result.last_insert_rowid();

        let repo = SiteUserRepository::new(pool.clone());

        // Add both users to the site
        let site_user1 = SiteUser::new(
            site_id,
            user1_id,
            doxyde_core::models::permission::SiteRole::Owner,
        );
        let site_user2 = SiteUser::new(
            site_id,
            user2_id,
            doxyde_core::models::permission::SiteRole::Editor,
        );

        repo.create(&site_user1).await?;
        repo.create(&site_user2).await?;

        // Delete only the first user
        repo.delete(site_id, user1_id).await?;

        // Verify first is gone
        let found1 = repo.find_by_site_and_user(site_id, user1_id).await?;
        assert!(found1.is_none());

        // Verify second still exists
        let found2 = repo.find_by_site_and_user(site_id, user2_id).await?;
        assert!(found2.is_some());

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_cascades_when_user_deleted() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let user_id = create_test_user(&pool).await?;
        let site_id = create_test_site(&pool).await?;

        let repo = SiteUserRepository::new(pool.clone());
        let site_user = SiteUser::new(
            site_id,
            user_id,
            doxyde_core::models::permission::SiteRole::Owner,
        );

        // Create the association
        repo.create(&site_user).await?;

        // Delete the user (should cascade to site_users)
        sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(user_id)
            .execute(&pool)
            .await?;

        // Verify association is gone
        let found = repo.find_by_site_and_user(site_id, user_id).await?;
        assert!(found.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_cascades_when_site_deleted() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let user_id = create_test_user(&pool).await?;
        let site_id = create_test_site(&pool).await?;

        let repo = SiteUserRepository::new(pool.clone());
        let site_user = SiteUser::new(
            site_id,
            user_id,
            doxyde_core::models::permission::SiteRole::Viewer,
        );

        // Create the association
        repo.create(&site_user).await?;

        // Delete the site (should cascade to site_users)
        sqlx::query("DELETE FROM sites WHERE id = ?")
            .bind(site_id)
            .execute(&pool)
            .await?;

        // Verify association is gone
        let found = repo.find_by_site_and_user(site_id, user_id).await?;
        assert!(found.is_none());

        Ok(())
    }
}
