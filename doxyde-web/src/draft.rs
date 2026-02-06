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

use anyhow::Result;
use doxyde_core::models::version::PageVersion;
use doxyde_db::repositories::{ComponentRepository, PageVersionRepository};
use sqlx::SqlitePool;

/// Get or create a draft version for a page
pub async fn get_or_create_draft(
    pool: &SqlitePool,
    page_id: i64,
    created_by: Option<String>,
) -> Result<PageVersion> {
    let version_repo = PageVersionRepository::new(pool.clone());
    let component_repo = ComponentRepository::new(pool.clone());

    // Check if there's already a draft
    if let Some(draft) = version_repo.get_draft(page_id).await? {
        return Ok(draft);
    }

    // No draft exists, create one by cloning the latest published version
    let published_version = version_repo.get_published(page_id).await?;

    // Get the next version number
    let next_version_number = version_repo.get_next_version_number(page_id).await?;

    // Create new draft version
    let draft = PageVersion::new(page_id, next_version_number, created_by);
    let draft_id = version_repo.create(&draft).await?;

    // If there was a published version, copy its components
    if let Some(published) = published_version {
        if let Some(published_id) = published.id {
            component_repo.copy_all(published_id, draft_id).await?;
        }
    }

    // Return the created draft with its ID
    Ok(PageVersion {
        id: Some(draft_id),
        ..draft
    })
}

/// Delete a draft version if it exists
pub async fn delete_draft_if_exists(pool: &SqlitePool, page_id: i64) -> Result<()> {
    let version_repo = PageVersionRepository::new(pool.clone());

    if let Some(draft) = version_repo.get_draft(page_id).await? {
        if let Some(draft_id) = draft.id {
            version_repo.delete_draft(draft_id).await?;
        }
    }

    Ok(())
}

/// Publish a draft version, clean up old versions and orphaned files
pub async fn publish_draft(pool: &SqlitePool, page_id: i64) -> Result<()> {
    doxyde_mcp::cleanup::publish_and_cleanup(pool, page_id).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_test_data(pool: &SqlitePool) -> Result<i64> {
        // Create tables
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

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS components (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                page_version_id INTEGER NOT NULL,
                component_type TEXT NOT NULL,
                position INTEGER NOT NULL,
                content TEXT NOT NULL,
                title TEXT,
                template TEXT NOT NULL DEFAULT 'default',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (page_version_id) REFERENCES page_versions(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(pool)
        .await?;

        // Create test site
        let site_id = sqlx::query("INSERT INTO sites (domain, title) VALUES (?, ?)")
            .bind("test.com")
            .bind("Test Site")
            .execute(pool)
            .await?
            .last_insert_rowid();

        // Create root page (sites automatically have a root page in real usage, but in tests we create it manually)
        let page_id = sqlx::query("INSERT INTO pages (site_id, slug, title) VALUES (?, ?, ?)")
            .bind(site_id)
            .bind("home")
            .bind("Home")
            .execute(pool)
            .await?
            .last_insert_rowid();

        Ok(page_id)
    }

    #[sqlx::test]
    async fn test_get_or_create_draft_no_existing_version() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let page_id = setup_test_data(&pool).await?;

        // Create draft for page with no versions
        let draft =
            get_or_create_draft(&pool, page_id, Some("test@example.com".to_string())).await?;

        assert_eq!(draft.page_id, page_id);
        assert_eq!(draft.version_number, 1);
        assert!(!draft.is_published);
        assert_eq!(draft.created_by, Some("test@example.com".to_string()));

        Ok(())
    }

    #[sqlx::test]
    async fn test_get_or_create_draft_with_existing_draft() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let page_id = setup_test_data(&pool).await?;

        // Create first draft
        let draft1 =
            get_or_create_draft(&pool, page_id, Some("user1@example.com".to_string())).await?;
        let draft1_id = draft1.id.unwrap();

        // Try to create another draft - should return the existing one
        let draft2 =
            get_or_create_draft(&pool, page_id, Some("user2@example.com".to_string())).await?;

        assert_eq!(draft2.id.unwrap(), draft1_id);
        assert_eq!(draft2.created_by, Some("user1@example.com".to_string()));

        Ok(())
    }

    #[sqlx::test]
    async fn test_publish_draft() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let page_id = setup_test_data(&pool).await?;

        // Create and publish a draft
        let draft =
            get_or_create_draft(&pool, page_id, Some("test@example.com".to_string())).await?;
        publish_draft(&pool, page_id).await?;

        // Verify it's published
        let version_repo = PageVersionRepository::new(pool.clone());
        let published = version_repo.get_published(page_id).await?;

        assert!(published.is_some());
        let published = published.unwrap();
        assert_eq!(published.id, draft.id);
        assert!(published.is_published);

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_draft() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let page_id = setup_test_data(&pool).await?;

        // Create a draft
        let _draft =
            get_or_create_draft(&pool, page_id, Some("test@example.com".to_string())).await?;

        // Delete it
        delete_draft_if_exists(&pool, page_id).await?;

        // Verify it's gone
        let version_repo = PageVersionRepository::new(pool.clone());
        let draft = version_repo.get_draft(page_id).await?;
        assert!(draft.is_none());

        Ok(())
    }
}
