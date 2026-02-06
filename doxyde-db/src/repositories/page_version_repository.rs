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
use doxyde_core::models::version::PageVersion;
use sqlx::SqlitePool;

fn parse_version_row(
    row: (i64, i64, i32, Option<String>, bool, String),
) -> Result<PageVersion> {
    let (id, page_id, version_number, created_by, is_published, created_at_str) = row;
    let created_at = if created_at_str.contains('T') {
        chrono::DateTime::parse_from_rfc3339(&created_at_str)
            .context("Failed to parse created_at as RFC3339")?
            .with_timezone(&chrono::Utc)
    } else {
        chrono::NaiveDateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S")
            .context("Failed to parse created_at as SQLite format")?
            .and_utc()
    };

    Ok(PageVersion {
        id: Some(id),
        page_id,
        version_number,
        created_by,
        is_published,
        created_at,
    })
}

pub struct PageVersionRepository {
    pool: SqlitePool,
}

impl PageVersionRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, page_version: &PageVersion) -> Result<i64> {
        if page_version.is_valid().is_err() {
            return Err(anyhow::anyhow!(
                "Invalid page version: {:?}",
                page_version.is_valid().err()
            ));
        }

        let result = sqlx::query(
            r#"
            INSERT INTO page_versions (page_id, version_number, created_by, is_published, created_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(page_version.page_id)
        .bind(page_version.version_number)
        .bind(&page_version.created_by)
        .bind(page_version.is_published)
        .bind(page_version.created_at)
        .execute(&self.pool)
        .await
        .context("Failed to create page version")?;

        Ok(result.last_insert_rowid())
    }

    pub async fn find_by_id(&self, id: i64) -> Result<Option<PageVersion>> {
        let row = sqlx::query_as::<_, (i64, i64, i32, Option<String>, bool, String)>(
            r#"
            SELECT id, page_id, version_number, created_by, is_published, created_at
            FROM page_versions
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to find page version by id")?;

        match row {
            Some((id, page_id, version_number, created_by, is_published, created_at_str)) => {
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

                Ok(Some(PageVersion {
                    id: Some(id),
                    page_id,
                    version_number,
                    created_by,
                    is_published,
                    created_at,
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn list_by_page(&self, page_id: i64) -> Result<Vec<PageVersion>> {
        let rows = sqlx::query_as::<_, (i64, i64, i32, Option<String>, bool, String)>(
            r#"
            SELECT id, page_id, version_number, created_by, is_published, created_at
            FROM page_versions
            WHERE page_id = ?
            ORDER BY version_number DESC
            "#,
        )
        .bind(page_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list page versions")?;

        let mut versions = Vec::new();

        for (id, page_id, version_number, created_by, is_published, created_at_str) in rows {
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

            versions.push(PageVersion {
                id: Some(id),
                page_id,
                version_number,
                created_by,
                is_published,
                created_at,
            });
        }

        Ok(versions)
    }

    pub async fn get_latest(&self, page_id: i64) -> Result<Option<PageVersion>> {
        let row = sqlx::query_as::<_, (i64, i64, i32, Option<String>, bool, String)>(
            r#"
            SELECT id, page_id, version_number, created_by, is_published, created_at
            FROM page_versions
            WHERE page_id = ?
            ORDER BY version_number DESC
            LIMIT 1
            "#,
        )
        .bind(page_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get latest page version")?;

        match row {
            Some((id, page_id, version_number, created_by, is_published, created_at_str)) => {
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

                Ok(Some(PageVersion {
                    id: Some(id),
                    page_id,
                    version_number,
                    created_by,
                    is_published,
                    created_at,
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn get_next_version_number(&self, page_id: i64) -> Result<i32> {
        let row: Option<(Option<i32>,)> =
            sqlx::query_as("SELECT MAX(version_number) FROM page_versions WHERE page_id = ?")
                .bind(page_id)
                .fetch_optional(&self.pool)
                .await
                .context("Failed to get max version number")?;

        match row {
            Some((Some(max_version),)) => Ok(max_version + 1),
            _ => Ok(1), // First version
        }
    }

    /// Get the current draft version (unpublished) for a page, if any
    pub async fn get_draft(&self, page_id: i64) -> Result<Option<PageVersion>> {
        let row = sqlx::query_as::<_, (i64, i64, i32, Option<String>, bool, String)>(
            r#"
            SELECT id, page_id, version_number, created_by, is_published, created_at
            FROM page_versions
            WHERE page_id = ? AND is_published = 0
            ORDER BY version_number DESC
            LIMIT 1
            "#,
        )
        .bind(page_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get draft page version")?;

        match row {
            Some((id, page_id, version_number, created_by, is_published, created_at_str)) => {
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

                Ok(Some(PageVersion {
                    id: Some(id),
                    page_id,
                    version_number,
                    created_by,
                    is_published,
                    created_at,
                }))
            }
            None => Ok(None),
        }
    }

    /// Get the latest published version for a page
    pub async fn get_published(&self, page_id: i64) -> Result<Option<PageVersion>> {
        let row = sqlx::query_as::<_, (i64, i64, i32, Option<String>, bool, String)>(
            r#"
            SELECT id, page_id, version_number, created_by, is_published, created_at
            FROM page_versions
            WHERE page_id = ? AND is_published = 1
            ORDER BY version_number DESC
            LIMIT 1
            "#,
        )
        .bind(page_id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get published page version")?;

        match row {
            Some((id, page_id, version_number, created_by, is_published, created_at_str)) => {
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

                Ok(Some(PageVersion {
                    id: Some(id),
                    page_id,
                    version_number,
                    created_by,
                    is_published,
                    created_at,
                }))
            }
            None => Ok(None),
        }
    }

    /// Publish a draft version
    pub async fn publish(&self, version_id: i64) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE page_versions
            SET is_published = 1
            WHERE id = ?
            "#,
        )
        .bind(version_id)
        .execute(&self.pool)
        .await
        .context("Failed to publish page version")?;

        Ok(())
    }

    /// Find all published page versions (for multi-database architecture
    /// where each DB is already site-specific, no site_id filter needed)
    pub async fn find_all_published(&self) -> Result<Vec<PageVersion>> {
        let rows = sqlx::query_as::<_, (i64, i64, i32, Option<String>, bool, String)>(
            r#"
            SELECT pv.id, pv.page_id, pv.version_number, pv.created_by, pv.is_published, pv.created_at
            FROM page_versions pv
            WHERE pv.is_published = 1
            ORDER BY pv.created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to find all published page versions")?;

        let versions = rows
            .into_iter()
            .map(
                |(id, page_id, version_number, created_by, is_published, created_at_str)| {
                    let created_at = if created_at_str.contains('T') {
                        chrono::DateTime::parse_from_rfc3339(&created_at_str)
                            .context("Failed to parse created_at as RFC3339")
                            .map(|dt| dt.with_timezone(&chrono::Utc))
                    } else {
                        chrono::NaiveDateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S")
                            .context("Failed to parse created_at as SQLite format")
                            .map(|dt| dt.and_utc())
                    }
                    .unwrap_or_else(|_| chrono::Utc::now());

                    PageVersion {
                        id: Some(id),
                        page_id,
                        version_number,
                        created_by,
                        is_published,
                        created_at,
                    }
                },
            )
            .collect();

        Ok(versions)
    }

    /// List all versions of a page except the specified one
    pub async fn list_old_versions(
        &self,
        page_id: i64,
        exclude_version_id: i64,
    ) -> Result<Vec<PageVersion>> {
        let rows = sqlx::query_as::<_, (i64, i64, i32, Option<String>, bool, String)>(
            r#"
            SELECT id, page_id, version_number, created_by, is_published, created_at
            FROM page_versions
            WHERE page_id = ? AND id != ?
            ORDER BY version_number ASC
            "#,
        )
        .bind(page_id)
        .bind(exclude_version_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list old versions")?;

        rows.into_iter()
            .map(parse_version_row)
            .collect()
    }

    /// Delete multiple versions by ID (CASCADE deletes their components)
    pub async fn delete_versions(&self, version_ids: &[i64]) -> Result<u64> {
        if version_ids.is_empty() {
            return Ok(0);
        }

        let mut total_deleted = 0u64;
        for id in version_ids {
            let result = sqlx::query("DELETE FROM page_versions WHERE id = ?")
                .bind(id)
                .execute(&self.pool)
                .await
                .context("Failed to delete version")?;
            total_deleted += result.rows_affected();
        }

        Ok(total_deleted)
    }

    /// Unpublish a version (set is_published = 0)
    pub async fn unpublish(&self, version_id: i64) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE page_versions
            SET is_published = 0
            WHERE id = ?
            "#,
        )
        .bind(version_id)
        .execute(&self.pool)
        .await
        .context("Failed to unpublish page version")?;

        Ok(())
    }

    /// Delete a draft version and all its components
    pub async fn find_published_by_site(&self, site_id: i64) -> Result<Vec<PageVersion>> {
        let rows = sqlx::query_as::<_, (i64, i64, i32, Option<String>, bool, String)>(
            r#"
            SELECT pv.id, pv.page_id, pv.version_number, pv.created_by, pv.is_published, pv.created_at
            FROM page_versions pv
            JOIN pages p ON pv.page_id = p.id
            WHERE p.site_id = ? AND pv.is_published = 1
            ORDER BY pv.created_at DESC
            "#,
        )
        .bind(site_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to find published page versions by site")?;

        let versions = rows
            .into_iter()
            .map(
                |(id, page_id, version_number, created_by, is_published, created_at_str)| {
                    let created_at = if created_at_str.contains('T') {
                        chrono::DateTime::parse_from_rfc3339(&created_at_str)
                            .context("Failed to parse created_at as RFC3339")
                            .map(|dt| dt.with_timezone(&chrono::Utc))
                    } else {
                        chrono::NaiveDateTime::parse_from_str(&created_at_str, "%Y-%m-%d %H:%M:%S")
                            .context("Failed to parse created_at as SQLite format")
                            .map(|dt| dt.and_utc())
                    }
                    .unwrap_or_else(|_| chrono::Utc::now());

                    PageVersion {
                        id: Some(id),
                        page_id,
                        version_number,
                        created_by,
                        is_published,
                        created_at,
                    }
                },
            )
            .collect();

        Ok(versions)
    }

    pub async fn delete_draft(&self, version_id: i64) -> Result<()> {
        // Components will be cascade deleted due to foreign key constraint
        sqlx::query(
            r#"
            DELETE FROM page_versions
            WHERE id = ? AND is_published = 0
            "#,
        )
        .bind(version_id)
        .execute(&self.pool)
        .await
        .context("Failed to delete draft version")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    async fn setup_test_db(pool: &SqlitePool) -> Result<()> {
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

        // Create pages table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS pages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                site_id INTEGER NOT NULL,
                parent_page_id INTEGER,
                slug TEXT NOT NULL,
                title TEXT NOT NULL,
                description TEXT,
                keywords TEXT,
                template TEXT DEFAULT 'default',
                meta_json TEXT DEFAULT '{}',
                position INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (site_id) REFERENCES sites(id) ON DELETE CASCADE,
                FOREIGN KEY (parent_page_id) REFERENCES pages(id) ON DELETE CASCADE,
                UNIQUE(site_id, parent_page_id, slug)
            )
            "#,
        )
        .execute(pool)
        .await?;

        // Create page_versions table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS page_versions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                page_id INTEGER NOT NULL,
                version_number INTEGER NOT NULL,
                created_by TEXT,
                is_published BOOLEAN NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (page_id) REFERENCES pages(id) ON DELETE CASCADE,
                UNIQUE(page_id, version_number)
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

        let repo = PageVersionRepository::new(pool.clone());

        // Verify we can access the pool by doing a simple query
        let _result = sqlx::query("SELECT 1").fetch_one(&repo.pool).await?;

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_page_version_success() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        // Create test data
        sqlx::query("INSERT INTO sites (domain, title) VALUES (?, ?)")
            .bind("test.com")
            .bind("Test Site")
            .execute(&pool)
            .await?;

        let page_id = sqlx::query("INSERT INTO pages (site_id, slug, title) VALUES (?, ?, ?)")
            .bind(1)
            .bind("test-page")
            .bind("Test Page")
            .execute(&pool)
            .await?
            .last_insert_rowid();

        let repo = PageVersionRepository::new(pool.clone());
        let version = PageVersion::new(page_id, 1, Some("user@example.com".to_string()));

        let id = repo.create(&version).await?;
        assert!(id > 0);

        // Verify it was created
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM page_versions WHERE id = ?")
            .bind(id)
            .fetch_one(&pool)
            .await?;
        assert_eq!(count.0, 1);

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_page_version_without_creator() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        // Create test data
        sqlx::query("INSERT INTO sites (domain, title) VALUES (?, ?)")
            .bind("test.com")
            .bind("Test Site")
            .execute(&pool)
            .await?;

        let page_id = sqlx::query("INSERT INTO pages (site_id, slug, title) VALUES (?, ?, ?)")
            .bind(1)
            .bind("test-page")
            .bind("Test Page")
            .execute(&pool)
            .await?
            .last_insert_rowid();

        let repo = PageVersionRepository::new(pool);
        let version = PageVersion::new(page_id, 1, None);

        let id = repo.create(&version).await?;
        assert!(id > 0);

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_page_version_invalid_fails() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = PageVersionRepository::new(pool);

        // Invalid page_id
        let invalid_version = PageVersion::new(0, 1, None);
        let result = repo.create(&invalid_version).await;
        assert!(result.is_err());

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_page_version_duplicate_number_fails() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        // Create test data
        sqlx::query("INSERT INTO sites (domain, title) VALUES (?, ?)")
            .bind("test.com")
            .bind("Test Site")
            .execute(&pool)
            .await?;

        let page_id = sqlx::query("INSERT INTO pages (site_id, slug, title) VALUES (?, ?, ?)")
            .bind(1)
            .bind("test-page")
            .bind("Test Page")
            .execute(&pool)
            .await?
            .last_insert_rowid();

        let repo = PageVersionRepository::new(pool);

        // Create version 1
        let version1 = PageVersion::new(page_id, 1, None);
        repo.create(&version1).await?;

        // Try to create another version 1
        let version1_duplicate = PageVersion::new(page_id, 1, None);
        let result = repo.create(&version1_duplicate).await;
        assert!(result.is_err());

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_id_existing() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        // Setup test data
        sqlx::query("INSERT INTO sites (domain, title) VALUES ('test.com', 'Test')")
            .execute(&pool)
            .await?;
        let page_id =
            sqlx::query("INSERT INTO pages (site_id, slug, title) VALUES (1, 'page', 'Page')")
                .execute(&pool)
                .await?
                .last_insert_rowid();

        let repo = PageVersionRepository::new(pool);
        let version = PageVersion::new(page_id, 1, Some("user@example.com".to_string()));
        let id = repo.create(&version).await?;

        // Find it
        let found = repo.find_by_id(id).await?;
        assert!(found.is_some());

        let found = found.unwrap();
        assert_eq!(found.id, Some(id));
        assert_eq!(found.page_id, page_id);
        assert_eq!(found.version_number, 1);
        assert_eq!(found.created_by, Some("user@example.com".to_string()));

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_id_non_existing() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = PageVersionRepository::new(pool);
        let found = repo.find_by_id(999).await?;
        assert!(found.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_by_page_empty() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = PageVersionRepository::new(pool);
        let versions = repo.list_by_page(999).await?;
        assert!(versions.is_empty());

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_by_page_multiple_versions() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        // Setup test data
        sqlx::query("INSERT INTO sites (domain, title) VALUES ('test.com', 'Test')")
            .execute(&pool)
            .await?;
        let page_id =
            sqlx::query("INSERT INTO pages (site_id, slug, title) VALUES (1, 'page', 'Page')")
                .execute(&pool)
                .await?
                .last_insert_rowid();

        let repo = PageVersionRepository::new(pool);

        // Create multiple versions
        for i in 1..=3 {
            let version = PageVersion::new(page_id, i, Some(format!("user{}", i)));
            repo.create(&version).await?;
        }

        // List should be in descending order
        let versions = repo.list_by_page(page_id).await?;
        assert_eq!(versions.len(), 3);
        assert_eq!(versions[0].version_number, 3);
        assert_eq!(versions[1].version_number, 2);
        assert_eq!(versions[2].version_number, 1);

        Ok(())
    }

    #[sqlx::test]
    async fn test_get_latest_no_versions() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = PageVersionRepository::new(pool);
        let latest = repo.get_latest(999).await?;
        assert!(latest.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_get_latest_with_versions() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        // Setup test data
        sqlx::query("INSERT INTO sites (domain, title) VALUES ('test.com', 'Test')")
            .execute(&pool)
            .await?;
        let page_id =
            sqlx::query("INSERT INTO pages (site_id, slug, title) VALUES (1, 'page', 'Page')")
                .execute(&pool)
                .await?
                .last_insert_rowid();

        let repo = PageVersionRepository::new(pool);

        // Create versions in random order
        let v2 = PageVersion::new(page_id, 2, Some("user2".to_string()));
        repo.create(&v2).await?;

        let v1 = PageVersion::new(page_id, 1, Some("user1".to_string()));
        repo.create(&v1).await?;

        let v3 = PageVersion::new(page_id, 3, Some("user3".to_string()));
        repo.create(&v3).await?;

        // Should get version 3
        let latest = repo.get_latest(page_id).await?;
        assert!(latest.is_some());
        let latest = latest.unwrap();
        assert_eq!(latest.version_number, 3);
        assert_eq!(latest.created_by, Some("user3".to_string()));

        Ok(())
    }

    #[sqlx::test]
    async fn test_get_next_version_number_no_versions() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = PageVersionRepository::new(pool);
        let next = repo.get_next_version_number(999).await?;
        assert_eq!(next, 1);

        Ok(())
    }

    #[sqlx::test]
    async fn test_get_next_version_number_with_versions() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        // Setup test data
        sqlx::query("INSERT INTO sites (domain, title) VALUES ('test.com', 'Test')")
            .execute(&pool)
            .await?;
        let page_id =
            sqlx::query("INSERT INTO pages (site_id, slug, title) VALUES (1, 'page', 'Page')")
                .execute(&pool)
                .await?
                .last_insert_rowid();

        let repo = PageVersionRepository::new(pool);

        // Initially should be 1
        let next = repo.get_next_version_number(page_id).await?;
        assert_eq!(next, 1);

        // Create version 1
        let v1 = PageVersion::new(page_id, 1, None);
        repo.create(&v1).await?;

        // Now should be 2
        let next = repo.get_next_version_number(page_id).await?;
        assert_eq!(next, 2);

        // Create version 2
        let v2 = PageVersion::new(page_id, 2, None);
        repo.create(&v2).await?;

        // Now should be 3
        let next = repo.get_next_version_number(page_id).await?;
        assert_eq!(next, 3);

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_old_versions() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        sqlx::query("INSERT INTO sites (domain, title) VALUES ('test.com', 'Test')")
            .execute(&pool)
            .await?;
        let page_id =
            sqlx::query("INSERT INTO pages (site_id, slug, title) VALUES (1, 'page', 'Page')")
                .execute(&pool)
                .await?
                .last_insert_rowid();

        let repo = PageVersionRepository::new(pool);

        let v1 = PageVersion::new(page_id, 1, None);
        let v1_id = repo.create(&v1).await?;

        let v2 = PageVersion::new(page_id, 2, None);
        let v2_id = repo.create(&v2).await?;

        let v3 = PageVersion::new(page_id, 3, None);
        let v3_id = repo.create(&v3).await?;

        // Exclude v3, should get v1 and v2
        let old = repo.list_old_versions(page_id, v3_id).await?;
        assert_eq!(old.len(), 2);
        assert_eq!(old[0].id, Some(v1_id));
        assert_eq!(old[1].id, Some(v2_id));

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_old_versions_empty() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        sqlx::query("INSERT INTO sites (domain, title) VALUES ('test.com', 'Test')")
            .execute(&pool)
            .await?;
        let page_id =
            sqlx::query("INSERT INTO pages (site_id, slug, title) VALUES (1, 'page', 'Page')")
                .execute(&pool)
                .await?
                .last_insert_rowid();

        let repo = PageVersionRepository::new(pool);

        let v1 = PageVersion::new(page_id, 1, None);
        let v1_id = repo.create(&v1).await?;

        // Only one version, exclude it => empty
        let old = repo.list_old_versions(page_id, v1_id).await?;
        assert!(old.is_empty());

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_versions() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        sqlx::query("INSERT INTO sites (domain, title) VALUES ('test.com', 'Test')")
            .execute(&pool)
            .await?;
        let page_id =
            sqlx::query("INSERT INTO pages (site_id, slug, title) VALUES (1, 'page', 'Page')")
                .execute(&pool)
                .await?
                .last_insert_rowid();

        let repo = PageVersionRepository::new(pool);

        let v1 = PageVersion::new(page_id, 1, None);
        let v1_id = repo.create(&v1).await?;

        let v2 = PageVersion::new(page_id, 2, None);
        let v2_id = repo.create(&v2).await?;

        let v3 = PageVersion::new(page_id, 3, None);
        let _v3_id = repo.create(&v3).await?;

        // Delete v1 and v2
        let deleted = repo.delete_versions(&[v1_id, v2_id]).await?;
        assert_eq!(deleted, 2);

        // Only v3 should remain
        let remaining = repo.list_by_page(page_id).await?;
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].version_number, 3);

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_versions_empty() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = PageVersionRepository::new(pool);

        let deleted = repo.delete_versions(&[]).await?;
        assert_eq!(deleted, 0);

        Ok(())
    }

    #[sqlx::test]
    async fn test_unpublish_version() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        sqlx::query("INSERT INTO sites (domain, title) VALUES ('test.com', 'Test')")
            .execute(&pool)
            .await?;
        let page_id =
            sqlx::query("INSERT INTO pages (site_id, slug, title) VALUES (1, 'page', 'Page')")
                .execute(&pool)
                .await?
                .last_insert_rowid();

        let repo = PageVersionRepository::new(pool);

        let v1 = PageVersion::new(page_id, 1, None);
        let v1_id = repo.create(&v1).await?;

        // Publish it first
        repo.publish(v1_id).await?;
        let found = repo.find_by_id(v1_id).await?.ok_or(anyhow::anyhow!("not found"))?;
        assert!(found.is_published);

        // Unpublish
        repo.unpublish(v1_id).await?;
        let found = repo.find_by_id(v1_id).await?.ok_or(anyhow::anyhow!("not found"))?;
        assert!(!found.is_published);

        Ok(())
    }
}
