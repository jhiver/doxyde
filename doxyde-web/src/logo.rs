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
use doxyde_db::repositories::{ComponentRepository, PageRepository, PageVersionRepository};
use sqlx::SqlitePool;

/// Logo data returned by get_logo_data
pub type LogoData = (String, Option<String>, Option<String>);

/// Get logo data for a site if a logo component exists
/// Returns (logo_url, logo_width, logo_height) or None if no logo
pub async fn get_logo_data(db: &SqlitePool, site_id: i64) -> Result<Option<LogoData>> {
    let page_repo = PageRepository::new(db.clone());
    let version_repo = PageVersionRepository::new(db.clone());
    let component_repo = ComponentRepository::new(db.clone());

    // Get the root page
    let root_page = page_repo
        .get_root_page(site_id)
        .await
        .context("Failed to get root page")?;

    let root_page = match root_page {
        Some(page) => page,
        None => return Ok(None), // No root page, no logo
    };

    // Get the published version of the root page
    let root_version = version_repo
        .get_published(root_page.id.unwrap())
        .await
        .context("Failed to get published version")?;

    let root_version = match root_version {
        Some(version) => version,
        None => return Ok(None), // No published version, no logo
    };

    // Get components for root page
    let components = component_repo
        .list_by_page_version(root_version.id.unwrap())
        .await
        .context("Failed to list components")?;

    // Look for logo image component
    for component in components {
        if component.component_type != "image" {
            continue;
        }

        // Check if this is logo
        if let Some(slug) = component.content.get("slug").and_then(|s| s.as_str()) {
            if slug == "logo" {
                // Get the format (default to png)
                let format = component
                    .content
                    .get("format")
                    .and_then(|f| f.as_str())
                    .unwrap_or("png");

                let logo_url = format!("/{}.{}", slug, format);

                // Get display dimensions if set
                let logo_width = component
                    .content
                    .get("display_width")
                    .and_then(|w| w.as_str())
                    .filter(|w| !w.is_empty())
                    .map(|w| w.to_string());

                let logo_height = component
                    .content
                    .get("display_height")
                    .and_then(|h| h.as_str())
                    .filter(|h| !h.is_empty())
                    .map(|h| h.to_string());

                return Ok(Some((logo_url, logo_width, logo_height)));
            }
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_schema(pool: &SqlitePool) -> Result<()> {
        // Create minimal schema for testing
        sqlx::query(
            r#"
            CREATE TABLE sites (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                domain TEXT NOT NULL UNIQUE,
                title TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );
            
            CREATE TABLE pages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                site_id INTEGER NOT NULL,
                parent_page_id INTEGER,
                slug TEXT NOT NULL,
                title TEXT NOT NULL,
                position INTEGER NOT NULL DEFAULT 0,
                sort_mode TEXT NOT NULL DEFAULT 'position',
                description TEXT,
                keywords TEXT,
                template TEXT NOT NULL DEFAULT 'default',
                meta_robots TEXT NOT NULL DEFAULT 'index,follow',
                canonical_url TEXT,
                og_image_url TEXT,
                structured_data_type TEXT NOT NULL DEFAULT 'WebPage',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (site_id) REFERENCES sites(id),
                FOREIGN KEY (parent_page_id) REFERENCES pages(id)
            );
            
            CREATE TABLE page_versions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                page_id INTEGER NOT NULL,
                version_number INTEGER NOT NULL,
                created_by INTEGER,
                comment TEXT,
                is_published BOOLEAN NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (page_id) REFERENCES pages(id) ON DELETE CASCADE
            );
            
            CREATE TABLE components (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                page_version_id INTEGER NOT NULL,
                component_type TEXT NOT NULL,
                title TEXT,
                content TEXT NOT NULL,
                template TEXT NOT NULL DEFAULT 'default',
                position INTEGER NOT NULL,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (page_version_id) REFERENCES page_versions(id) ON DELETE CASCADE
            );
            "#,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    #[sqlx::test]
    async fn test_get_logo_data_no_site(pool: SqlitePool) -> Result<()> {
        create_test_schema(&pool).await?;
        let result = get_logo_data(&pool, 999).await?;
        assert!(result.is_none());
        Ok(())
    }
}
