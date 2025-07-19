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
use doxyde_core::{Page, Site};
use sqlx::SqlitePool;

pub struct SiteRepository {
    pool: SqlitePool,
}

impl SiteRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, site: &Site) -> Result<i64> {
        // Use a transaction to ensure both site and root page are created atomically
        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to begin transaction")?;

        // Create the site
        let site_result = sqlx::query(
            r#"
            INSERT INTO sites (domain, title, created_at, updated_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(&site.domain)
        .bind(&site.title)
        .bind(site.created_at)
        .bind(site.updated_at)
        .execute(&mut *tx)
        .await
        .context("Failed to create site")?;

        let site_id = site_result.last_insert_rowid();

        // Create the root page for this site
        let root_page = Page::new(site_id, "".to_string(), "Home".to_string());

        sqlx::query(
            r#"
            INSERT INTO pages (site_id, parent_page_id, slug, title, description, keywords, template, meta_robots, canonical_url, og_image_url, structured_data_type, position, created_at, updated_at)
            VALUES (?, NULL, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(site_id)
        .bind(&root_page.slug)
        .bind(&root_page.title)
        .bind(&root_page.description)
        .bind(&root_page.keywords)
        .bind(&root_page.template)
        .bind(&root_page.meta_robots)
        .bind(&root_page.canonical_url)
        .bind(&root_page.og_image_url)
        .bind(&root_page.structured_data_type)
        .bind(root_page.position)
        .bind(root_page.created_at)
        .bind(root_page.updated_at)
        .execute(&mut *tx)
        .await
        .context("Failed to create root page")?;

        // Commit the transaction
        tx.commit().await.context("Failed to commit transaction")?;

        Ok(site_id)
    }

    pub async fn find_by_id(&self, id: i64) -> Result<Option<Site>> {
        let result = sqlx::query_as::<_, (i64, String, String, String, String)>(
            r#"
            SELECT id, domain, title, created_at, updated_at
            FROM sites
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to find site by id")?;

        match result {
            Some((id, domain, title, created_at_str, updated_at_str)) => {
                // SQLite stores datetime as "YYYY-MM-DD HH:MM:SS" or ISO8601
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

                Ok(Some(Site {
                    id: Some(id),
                    domain,
                    title,
                    created_at,
                    updated_at,
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn find_by_domain(&self, domain: &str) -> Result<Option<Site>> {
        let result = sqlx::query_as::<_, (i64, String, String, String, String)>(
            r#"
            SELECT id, domain, title, created_at, updated_at
            FROM sites
            WHERE domain = ?
            "#,
        )
        .bind(domain)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to find site by domain")?;

        match result {
            Some((id, domain, title, created_at_str, updated_at_str)) => {
                // SQLite stores datetime as "YYYY-MM-DD HH:MM:SS" or ISO8601
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

                Ok(Some(Site {
                    id: Some(id),
                    domain,
                    title,
                    created_at,
                    updated_at,
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn update(&self, site: &Site) -> Result<()> {
        let id = site.id.context("Cannot update site without ID")?;

        let rows_affected = sqlx::query(
            r#"
            UPDATE sites
            SET domain = ?, title = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&site.domain)
        .bind(&site.title)
        .bind(site.updated_at)
        .bind(id)
        .execute(&self.pool)
        .await
        .context("Failed to update site")?
        .rows_affected();

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Site with id {} not found", id));
        }

        Ok(())
    }

    pub async fn delete(&self, id: i64) -> Result<()> {
        let rows_affected = sqlx::query(
            r#"
            DELETE FROM sites
            WHERE id = ?
            "#,
        )
        .bind(id)
        .execute(&self.pool)
        .await
        .context("Failed to delete site")?
        .rows_affected();

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Site with id {} not found", id));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test]
    async fn test_new_creates_repository() -> Result<(), sqlx::Error> {
        // The pool is provided by sqlx::test attribute
        let pool = SqlitePool::connect(":memory:").await?;

        let repo = SiteRepository::new(pool.clone());

        // Verify we can access the pool (it's stored correctly)
        // We'll do a simple query to ensure the connection works
        // Test that pool is accessible
        let _result = sqlx::query("SELECT 1").fetch_one(&repo.pool).await?;

        Ok(())
    }

    async fn setup_test_db(pool: &SqlitePool) -> Result<()> {
        // Run migration to create tables
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

        // Create pages table since site creation now creates a root page
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
                meta_robots TEXT NOT NULL DEFAULT 'index,follow',
                canonical_url TEXT,
                og_image_url TEXT,
                structured_data_type TEXT NOT NULL DEFAULT 'WebPage',
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

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_site_success() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteRepository::new(pool);
        let site = Site::new("example.com".to_string(), "Example Site".to_string());

        let id = repo.create(&site).await?;

        // Verify ID is valid
        assert!(id > 0);

        // Verify the site was actually inserted
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sites")
            .fetch_one(&repo.pool)
            .await?;
        assert_eq!(row.0, 1);

        // Verify the root page was created
        let page_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pages WHERE site_id = ?")
            .bind(id)
            .fetch_one(&repo.pool)
            .await?;
        assert_eq!(page_count.0, 1);

        // Verify the root page has correct properties
        let root_page: (Option<i64>, String, String) =
            sqlx::query_as("SELECT parent_page_id, slug, title FROM pages WHERE site_id = ?")
                .bind(id)
                .fetch_one(&repo.pool)
                .await?;

        assert_eq!(root_page.0, None); // parent_page_id should be NULL
        assert_eq!(root_page.1, "");
        assert_eq!(root_page.2, "Home");

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_site_with_all_fields() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteRepository::new(pool);
        let site = Site::new("test.org".to_string(), "Test Organization".to_string());

        let id = repo.create(&site).await?;

        // Verify the data was inserted correctly
        let row: (String, String, String, String) = sqlx::query_as(
            r#"
            SELECT domain, title, created_at, updated_at 
            FROM sites 
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_one(&repo.pool)
        .await?;

        assert_eq!(row.0, site.domain);
        assert_eq!(row.1, site.title);
        assert!(!row.2.is_empty());
        assert!(!row.3.is_empty());

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_site_duplicate_domain_fails() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteRepository::new(pool);
        let site1 = Site::new("duplicate.com".to_string(), "First Site".to_string());
        let site2 = Site::new("duplicate.com".to_string(), "Second Site".to_string());

        // First insert should succeed
        let id1 = repo.create(&site1).await?;
        assert!(id1 > 0);

        // Second insert with same domain should fail
        let result = repo.create(&site2).await;
        assert!(result.is_err());

        // Verify error message contains context
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Failed to create site"));

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_multiple_sites() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteRepository::new(pool);

        let sites = vec![
            Site::new("site1.com".to_string(), "Site One".to_string()),
            Site::new("site2.com".to_string(), "Site Two".to_string()),
            Site::new("site3.com".to_string(), "Site Three".to_string()),
        ];

        let mut ids = Vec::new();
        for site in &sites {
            let id = repo.create(site).await?;
            ids.push(id);
        }

        // Verify all IDs are unique and valid
        assert_eq!(ids.len(), 3);
        for (i, id) in ids.iter().enumerate() {
            assert!(*id > 0);
            if i > 0 {
                assert!(*id > ids[i - 1]);
            }
        }

        // Verify count
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sites")
            .fetch_one(&repo.pool)
            .await?;
        assert_eq!(row.0, 3);

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_id_existing_site() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteRepository::new(pool);
        let site = Site::new("findme.com".to_string(), "Find Me Site".to_string());

        // Create the site first
        let id = repo.create(&site).await?;

        // Find it by ID
        let found = repo.find_by_id(id).await?;

        assert!(found.is_some());
        let found_site = found.unwrap();
        assert_eq!(found_site.id, Some(id));
        assert_eq!(found_site.domain, site.domain);
        assert_eq!(found_site.title, site.title);

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_id_non_existing_site() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteRepository::new(pool);

        // Try to find a non-existing site
        let found = repo.find_by_id(999).await?;

        assert!(found.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_id_with_timestamps() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteRepository::new(pool);
        let site = Site::new("timestamp.com".to_string(), "Timestamp Site".to_string());

        // Store original timestamps
        let original_created = site.created_at;
        let original_updated = site.updated_at;

        // Create the site
        let id = repo.create(&site).await?;

        // Find it by ID
        let found = repo.find_by_id(id).await?;

        assert!(found.is_some());
        let found_site = found.unwrap();

        // Timestamps should be close to the originals (within 1 second)
        let created_diff = (found_site.created_at - original_created)
            .num_seconds()
            .abs();
        let updated_diff = (found_site.updated_at - original_updated)
            .num_seconds()
            .abs();

        assert!(created_diff <= 1);
        assert!(updated_diff <= 1);

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_id_multiple_sites() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteRepository::new(pool);

        // Create multiple sites
        let sites = vec![
            Site::new("first.com".to_string(), "First Site".to_string()),
            Site::new("second.com".to_string(), "Second Site".to_string()),
            Site::new("third.com".to_string(), "Third Site".to_string()),
        ];

        let mut ids = Vec::new();
        for site in &sites {
            ids.push(repo.create(site).await?);
        }

        // Find each site by its ID
        for (i, id) in ids.iter().enumerate() {
            let found = repo.find_by_id(*id).await?;
            assert!(found.is_some());

            let found_site = found.unwrap();
            assert_eq!(found_site.id, Some(*id));
            assert_eq!(found_site.domain, sites[i].domain);
            assert_eq!(found_site.title, sites[i].title);
        }

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_id_zero_and_negative() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteRepository::new(pool);

        // Test with ID 0
        let found = repo.find_by_id(0).await?;
        assert!(found.is_none());

        // Test with negative ID
        let found = repo.find_by_id(-1).await?;
        assert!(found.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_domain_existing_site() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteRepository::new(pool);
        let site = Site::new("searchme.com".to_string(), "Search Me Site".to_string());

        // Create the site first
        let id = repo.create(&site).await?;

        // Find it by domain
        let found = repo.find_by_domain("searchme.com").await?;

        assert!(found.is_some());
        let found_site = found.unwrap();
        assert_eq!(found_site.id, Some(id));
        assert_eq!(found_site.domain, site.domain);
        assert_eq!(found_site.title, site.title);

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_domain_non_existing_site() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteRepository::new(pool);

        // Try to find a non-existing domain
        let found = repo.find_by_domain("nonexistent.com").await?;

        assert!(found.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_domain_case_sensitive() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteRepository::new(pool);
        let site = Site::new(
            "CaseSensitive.com".to_string(),
            "Case Sensitive Site".to_string(),
        );

        // Create the site
        repo.create(&site).await?;

        // Try to find with exact case - should work
        let found = repo.find_by_domain("CaseSensitive.com").await?;
        assert!(found.is_some());

        // Try to find with different case - behavior depends on SQLite collation
        // Default SQLite is case-sensitive for text comparisons
        let found_lower = repo.find_by_domain("casesensitive.com").await?;
        assert!(found_lower.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_domain_with_special_characters() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteRepository::new(pool);

        // Test with various special characters in domain
        let test_domains = vec![
            "sub-domain.com",
            "under_score.com",
            "123numbers.com",
            "日本語.jp",
        ];

        for domain in test_domains {
            let site = Site::new(domain.to_string(), format!("Site for {}", domain));
            repo.create(&site).await?;

            let found = repo.find_by_domain(domain).await?;
            assert!(found.is_some(), "Should find domain: {}", domain);
            assert_eq!(found.unwrap().domain, domain);
        }

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_domain_empty_string() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteRepository::new(pool);

        // Try to find with empty domain
        let found = repo.find_by_domain("").await?;
        assert!(found.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_domain_multiple_sites_different_domains() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteRepository::new(pool);

        // Create multiple sites with different domains
        let sites = vec![
            Site::new("first.com".to_string(), "First Site".to_string()),
            Site::new("second.com".to_string(), "Second Site".to_string()),
            Site::new("third.com".to_string(), "Third Site".to_string()),
        ];

        for site in &sites {
            repo.create(site).await?;
        }

        // Find each site by its domain
        for site in &sites {
            let found = repo.find_by_domain(&site.domain).await?;
            assert!(found.is_some());

            let found_site = found.unwrap();
            assert_eq!(found_site.domain, site.domain);
            assert_eq!(found_site.title, site.title);
        }

        Ok(())
    }

    #[sqlx::test]
    async fn test_update_existing_site() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteRepository::new(pool);
        let mut site = Site::new("original.com".to_string(), "Original Title".to_string());

        // Create the site first
        let id = repo.create(&site).await?;
        site.id = Some(id);

        // Update the site
        site.domain = "updated.com".to_string();
        site.title = "Updated Title".to_string();
        site.updated_at = chrono::Utc::now();

        repo.update(&site).await?;

        // Verify the update
        let found = repo.find_by_id(id).await?;
        assert!(found.is_some());

        let updated_site = found.unwrap();
        assert_eq!(updated_site.domain, "updated.com");
        assert_eq!(updated_site.title, "Updated Title");
        assert!(updated_site.updated_at > site.created_at);

        Ok(())
    }

    #[sqlx::test]
    async fn test_update_non_existing_site() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteRepository::new(pool);
        let mut site = Site::new("test.com".to_string(), "Test".to_string());
        site.id = Some(999); // Non-existent ID

        let result = repo.update(&site).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Site with id 999 not found"));

        Ok(())
    }

    #[sqlx::test]
    async fn test_update_site_without_id() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteRepository::new(pool);
        let site = Site::new("test.com".to_string(), "Test".to_string());
        // site.id is None

        let result = repo.update(&site).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot update site without ID"));

        Ok(())
    }

    #[sqlx::test]
    async fn test_update_preserves_created_at() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteRepository::new(pool);
        let mut site = Site::new("preserve.com".to_string(), "Original".to_string());
        let original_created_at = site.created_at;

        // Create the site
        let id = repo.create(&site).await?;
        site.id = Some(id);

        // Wait a moment to ensure updated_at will be different
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Update the site
        site.title = "Updated".to_string();
        site.updated_at = chrono::Utc::now();
        repo.update(&site).await?;

        // Verify created_at is preserved
        let found = repo.find_by_id(id).await?.unwrap();
        let created_diff = (found.created_at - original_created_at).num_seconds().abs();
        assert!(created_diff <= 1); // Should be the same (within 1 second tolerance)
        assert!(found.updated_at > found.created_at);

        Ok(())
    }

    #[sqlx::test]
    async fn test_update_with_duplicate_domain_fails() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteRepository::new(pool);

        // Create two sites
        let site1 = Site::new("site1.com".to_string(), "Site 1".to_string());
        let site2 = Site::new("site2.com".to_string(), "Site 2".to_string());

        let _id1 = repo.create(&site1).await?;
        let id2 = repo.create(&site2).await?;

        // Try to update site2 with site1's domain
        let mut site2_update = site2.clone();
        site2_update.id = Some(id2);
        site2_update.domain = "site1.com".to_string(); // Duplicate!

        let result = repo.update(&site2_update).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to update site"));

        // Verify site2 wasn't changed
        let unchanged = repo.find_by_id(id2).await?.unwrap();
        assert_eq!(unchanged.domain, "site2.com");

        Ok(())
    }

    #[sqlx::test]
    async fn test_update_multiple_times() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteRepository::new(pool);
        let mut site = Site::new("multi.com".to_string(), "Version 1".to_string());

        // Create the site
        let id = repo.create(&site).await?;
        site.id = Some(id);

        // Update multiple times
        for i in 2..=5 {
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
            site.title = format!("Version {}", i);
            site.updated_at = chrono::Utc::now();
            repo.update(&site).await?;

            // Verify each update
            let found = repo.find_by_id(id).await?.unwrap();
            assert_eq!(found.title, format!("Version {}", i));
        }

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_existing_site() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteRepository::new(pool);
        let site = Site::new("deleteme.com".to_string(), "Delete Me Site".to_string());

        // Create the site first
        let id = repo.create(&site).await?;

        // Verify it exists
        let found = repo.find_by_id(id).await?;
        assert!(found.is_some());

        // Delete it
        repo.delete(id).await?;

        // Verify it's gone
        let not_found = repo.find_by_id(id).await?;
        assert!(not_found.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_non_existing_site() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteRepository::new(pool);

        // Try to delete a non-existing site
        let result = repo.delete(999).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Site with id 999 not found"));

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_site_cascades_to_pages() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteRepository::new(pool.clone());
        let site = Site::new("cascade.com".to_string(), "Cascade Site".to_string());

        // Create site (which automatically creates a root page)
        let site_id = repo.create(&site).await?;

        // Create an additional page for this site
        sqlx::query("INSERT INTO pages (site_id, parent_page_id, slug, title) VALUES (?, ?, ?, ?)")
            .bind(site_id)
            .bind(1) // Assuming root page has id 1
            .bind("test-page")
            .bind("Test Page")
            .execute(&pool)
            .await?;

        // Verify pages exist (1 root + 1 additional)
        let page_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pages WHERE site_id = ?")
            .bind(site_id)
            .fetch_one(&pool)
            .await?;
        assert_eq!(page_count.0, 2);

        // Delete the site
        repo.delete(site_id).await?;

        // Verify pages are also deleted (cascade)
        let page_count_after: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM pages WHERE site_id = ?")
                .bind(site_id)
                .fetch_one(&pool)
                .await?;
        assert_eq!(page_count_after.0, 0);

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_multiple_sites() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteRepository::new(pool);

        // Create multiple sites
        let sites = vec![
            Site::new("delete1.com".to_string(), "Delete 1".to_string()),
            Site::new("delete2.com".to_string(), "Delete 2".to_string()),
            Site::new("delete3.com".to_string(), "Delete 3".to_string()),
        ];

        let mut ids = Vec::new();
        for site in &sites {
            ids.push(repo.create(site).await?);
        }

        // Delete sites one by one
        for id in &ids {
            repo.delete(*id).await?;
        }

        // Verify all are deleted
        for id in &ids {
            let found = repo.find_by_id(*id).await?;
            assert!(found.is_none());
        }

        // Verify table is empty
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sites")
            .fetch_one(&repo.pool)
            .await?;
        assert_eq!(count.0, 0);

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_already_deleted_site() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteRepository::new(pool);
        let site = Site::new("double-delete.com".to_string(), "Double Delete".to_string());

        // Create and delete the site
        let id = repo.create(&site).await?;
        repo.delete(id).await?;

        // Try to delete again
        let result = repo.delete(id).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains(&format!("Site with id {} not found", id)));

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_zero_and_negative_ids() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteRepository::new(pool);

        // Try to delete with ID 0
        let result = repo.delete(0).await;
        assert!(result.is_err());

        // Try to delete with negative ID
        let result = repo.delete(-1).await;
        assert!(result.is_err());

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_site_with_root_page_transaction() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        // Create a site that will succeed
        let repo = SiteRepository::new(pool.clone());
        let site1 = Site::new(
            "transaction-test.com".to_string(),
            "Transaction Test".to_string(),
        );
        let site_id = repo.create(&site1).await?;

        // Verify both site and root page were created
        let site_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sites")
            .fetch_one(&pool)
            .await?;
        assert_eq!(site_count.0, 1);

        let page_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pages")
            .fetch_one(&pool)
            .await?;
        assert_eq!(page_count.0, 1);

        // Verify the root page has the correct site_id
        let root_page_site_id: (i64,) =
            sqlx::query_as("SELECT site_id FROM pages WHERE parent_page_id IS NULL")
                .fetch_one(&pool)
                .await?;
        assert_eq!(root_page_site_id.0, site_id);

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_multiple_sites_each_with_root_page() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteRepository::new(pool.clone());

        // Create multiple sites
        let sites = vec![
            Site::new("site1.com".to_string(), "Site One".to_string()),
            Site::new("site2.com".to_string(), "Site Two".to_string()),
            Site::new("site3.com".to_string(), "Site Three".to_string()),
        ];

        let mut site_ids = Vec::new();
        for site in &sites {
            let id = repo.create(site).await?;
            site_ids.push(id);
        }

        // Verify each site has exactly one root page
        for site_id in &site_ids {
            let page_count: (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM pages WHERE site_id = ? AND parent_page_id IS NULL",
            )
            .bind(site_id)
            .fetch_one(&pool)
            .await?;
            assert_eq!(
                page_count.0, 1,
                "Site {} should have exactly one root page",
                site_id
            );
        }

        // Verify total counts
        let total_sites: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM sites")
            .fetch_one(&pool)
            .await?;
        assert_eq!(total_sites.0, 3);

        let total_pages: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pages")
            .fetch_one(&pool)
            .await?;
        assert_eq!(total_pages.0, 3); // One root page per site

        Ok(())
    }
}
