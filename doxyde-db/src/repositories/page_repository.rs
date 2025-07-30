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
use doxyde_core::{utils::slug::generate_slug_from_title, Page};
use sqlx::SqlitePool;

pub struct PageRepository {
    pool: SqlitePool,
}

impl PageRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, page: &Page) -> Result<i64> {
        // Root pages are created automatically with sites and cannot be created manually
        if page.parent_page_id.is_none() {
            return Err(anyhow::anyhow!(
                "Root pages are created automatically with sites and cannot be created manually"
            ));
        }

        let result = sqlx::query(
            r#"
            INSERT INTO pages (site_id, parent_page_id, slug, title, description, keywords, template, meta_robots, canonical_url, og_image_url, structured_data_type, position, sort_mode, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(page.site_id)
        .bind(page.parent_page_id)
        .bind(&page.slug)
        .bind(&page.title)
        .bind(&page.description)
        .bind(&page.keywords)
        .bind(&page.template)
        .bind(&page.meta_robots)
        .bind(&page.canonical_url)
        .bind(&page.og_image_url)
        .bind(&page.structured_data_type)
        .bind(page.position)
        .bind(&page.sort_mode)
        .bind(page.created_at)
        .bind(page.updated_at)
        .execute(&self.pool)
        .await
        .context("Failed to create page")?;

        Ok(result.last_insert_rowid())
    }

    pub async fn find_by_id(&self, id: i64) -> Result<Option<Page>> {
        let result =
            sqlx::query_as::<_, (i64, i64, Option<i64>, String, String, Option<String>, Option<String>, String, String, Option<String>, Option<String>, String, i32, String, String, String)>(
                r#"
            SELECT id, site_id, parent_page_id, slug, title, description, keywords, template, meta_robots, canonical_url, og_image_url, structured_data_type, position, sort_mode, created_at, updated_at
            FROM pages
            WHERE id = ?
            "#,
            )
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .context("Failed to find page by id")?;

        match result {
            Some((
                id,
                site_id,
                parent_page_id,
                slug,
                title,
                description,
                keywords,
                template,
                meta_robots,
                canonical_url,
                og_image_url,
                structured_data_type,
                position,
                sort_mode,
                created_at_str,
                updated_at_str,
            )) => {
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

                Ok(Some(Page {
                    id: Some(id),
                    site_id,
                    parent_page_id,
                    slug,
                    title,
                    description,
                    keywords,
                    template,
                    meta_robots,
                    canonical_url,
                    og_image_url,
                    structured_data_type,
                    position,
                    sort_mode,
                    created_at,
                    updated_at,
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn find_by_slug_and_site_id(&self, slug: &str, site_id: i64) -> Result<Option<Page>> {
        let result =
            sqlx::query_as::<_, (i64, i64, Option<i64>, String, String, Option<String>, Option<String>, String, String, Option<String>, Option<String>, String, i32, String, String, String)>(
                r#"
            SELECT id, site_id, parent_page_id, slug, title, description, keywords, template, meta_robots, canonical_url, og_image_url, structured_data_type, position, sort_mode, created_at, updated_at
            FROM pages
            WHERE slug = ? AND site_id = ?
            "#,
            )
            .bind(slug)
            .bind(site_id)
            .fetch_optional(&self.pool)
            .await
            .context("Failed to find page by slug and site_id")?;

        match result {
            Some((
                id,
                site_id,
                parent_page_id,
                slug,
                title,
                description,
                keywords,
                template,
                meta_robots,
                canonical_url,
                og_image_url,
                structured_data_type,
                position,
                sort_mode,
                created_at_str,
                updated_at_str,
            )) => {
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

                Ok(Some(Page {
                    id: Some(id),
                    site_id,
                    parent_page_id,
                    slug,
                    title,
                    description,
                    keywords,
                    template,
                    meta_robots,
                    canonical_url,
                    og_image_url,
                    structured_data_type,
                    position,
                    sort_mode,
                    created_at,
                    updated_at,
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn list_by_site_id(&self, site_id: i64) -> Result<Vec<Page>> {
        let results =
            sqlx::query_as::<_, (i64, i64, Option<i64>, String, String, Option<String>, Option<String>, String, String, Option<String>, Option<String>, String, i32, String, String, String)>(
                r#"
            SELECT id, site_id, parent_page_id, slug, title, description, keywords, template, meta_robots, canonical_url, og_image_url, structured_data_type, position, sort_mode, created_at, updated_at
            FROM pages
            WHERE site_id = ?
            ORDER BY parent_page_id, position, slug
            "#,
            )
            .bind(site_id)
            .fetch_all(&self.pool)
            .await
            .context("Failed to list pages by site_id")?;

        let mut pages = Vec::new();
        for (
            id,
            site_id,
            parent_page_id,
            slug,
            title,
            description,
            keywords,
            template,
            meta_robots,
            canonical_url,
            og_image_url,
            structured_data_type,
            position,
            sort_mode,
            created_at_str,
            updated_at_str,
        ) in results
        {
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

            pages.push(Page {
                id: Some(id),
                site_id,
                parent_page_id,
                slug,
                title,
                description,
                keywords,
                template,
                meta_robots,
                canonical_url,
                og_image_url,
                structured_data_type,
                position,
                sort_mode,
                created_at,
                updated_at,
            });
        }

        Ok(pages)
    }

    pub async fn update(&self, page: &Page) -> Result<()> {
        let id = page.id.context("Cannot update page without ID")?;

        let rows_affected = sqlx::query(
            r#"
            UPDATE pages
            SET site_id = ?, parent_page_id = ?, slug = ?, title = ?, description = ?, keywords = ?, template = ?, meta_robots = ?, canonical_url = ?, og_image_url = ?, structured_data_type = ?, position = ?, sort_mode = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(page.site_id)
        .bind(page.parent_page_id)
        .bind(&page.slug)
        .bind(&page.title)
        .bind(&page.description)
        .bind(&page.keywords)
        .bind(&page.template)
        .bind(&page.meta_robots)
        .bind(&page.canonical_url)
        .bind(&page.og_image_url)
        .bind(&page.structured_data_type)
        .bind(page.position)
        .bind(&page.sort_mode)
        .bind(page.updated_at)
        .bind(id)
        .execute(&self.pool)
        .await
        .context("Failed to update page")?
        .rows_affected();

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Page with id {} not found", id));
        }

        Ok(())
    }

    pub async fn delete(&self, id: i64) -> Result<()> {
        // First check if the page exists and get its info
        let page = self
            .find_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page with id {} not found", id))?;

        // Check if this is a root page
        if page.parent_page_id.is_none() {
            return Err(anyhow::anyhow!("Cannot delete root page"));
        }

        // Check if the page has children
        let children = self.list_children(id).await?;
        if !children.is_empty() {
            return Err(anyhow::anyhow!(
                "Cannot delete page with id {} because it has {} child page(s)",
                id,
                children.len()
            ));
        }

        // Start a transaction to ensure all deletions happen together
        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to start transaction")?;

        // Delete all components for all versions of this page
        sqlx::query!(
            r#"
            DELETE FROM components 
            WHERE page_version_id IN (
                SELECT id FROM page_versions WHERE page_id = ?
            )
            "#,
            id
        )
        .execute(&mut *tx)
        .await
        .context("Failed to delete page components")?;

        // Delete all page versions
        sqlx::query!(r#"DELETE FROM page_versions WHERE page_id = ?"#, id)
            .execute(&mut *tx)
            .await
            .context("Failed to delete page versions")?;

        // Delete the page itself
        sqlx::query!(r#"DELETE FROM pages WHERE id = ?"#, id)
            .execute(&mut *tx)
            .await
            .context("Failed to delete page")?;

        // Commit the transaction
        tx.commit().await.context("Failed to commit transaction")?;

        Ok(())
    }

    /// List children pages sorted according to the parent's sort_mode
    pub async fn list_children_sorted(&self, parent_page_id: i64) -> Result<Vec<Page>> {
        // First get the parent page to check its sort_mode
        let parent = self
            .find_by_id(parent_page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Parent page not found"))?;

        // Build the ORDER BY clause based on sort_mode
        let order_by = match parent.sort_mode.as_str() {
            "created_at_asc" => "created_at ASC",
            "created_at_desc" => "created_at DESC",
            "title_asc" => "title ASC",
            "title_desc" => "title DESC",
            "manual" => "position ASC, slug ASC",
            _ => "position ASC, slug ASC", // fallback to manual sorting
        };

        let query = format!(
            r#"
            SELECT id, site_id, parent_page_id, slug, title, description, keywords, template, meta_robots, canonical_url, og_image_url, structured_data_type, position, sort_mode, created_at, updated_at
            FROM pages
            WHERE parent_page_id = ?
            ORDER BY {}
            "#,
            order_by
        );

        let results = sqlx::query_as::<
            _,
            (
                i64,
                i64,
                Option<i64>,
                String,
                String,
                Option<String>,
                Option<String>,
                String,
                String,
                Option<String>,
                Option<String>,
                String,
                i32,
                String,
                String,
                String,
            ),
        >(&query)
        .bind(parent_page_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list children pages")?;

        let mut pages = Vec::new();
        for (
            id,
            site_id,
            parent_page_id,
            slug,
            title,
            description,
            keywords,
            template,
            meta_robots,
            canonical_url,
            og_image_url,
            structured_data_type,
            position,
            sort_mode,
            created_at_str,
            updated_at_str,
        ) in results
        {
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

            pages.push(Page {
                id: Some(id),
                site_id,
                parent_page_id,
                slug,
                title,
                description,
                keywords,
                template,
                meta_robots,
                canonical_url,
                og_image_url,
                structured_data_type,
                position,
                sort_mode,
                created_at,
                updated_at,
            });
        }

        Ok(pages)
    }

    pub async fn list_children(&self, parent_page_id: i64) -> Result<Vec<Page>> {
        let results =
            sqlx::query_as::<_, (i64, i64, Option<i64>, String, String, Option<String>, Option<String>, String, String, Option<String>, Option<String>, String, i32, String, String, String)>(
                r#"
            SELECT id, site_id, parent_page_id, slug, title, description, keywords, template, meta_robots, canonical_url, og_image_url, structured_data_type, position, sort_mode, created_at, updated_at
            FROM pages
            WHERE parent_page_id = ?
            ORDER BY position, slug
            "#,
            )
            .bind(parent_page_id)
            .fetch_all(&self.pool)
            .await
            .context("Failed to list children pages")?;

        let mut pages = Vec::new();
        for (
            id,
            site_id,
            parent_page_id,
            slug,
            title,
            description,
            keywords,
            template,
            meta_robots,
            canonical_url,
            og_image_url,
            structured_data_type,
            position,
            sort_mode,
            created_at_str,
            updated_at_str,
        ) in results
        {
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

            pages.push(Page {
                id: Some(id),
                site_id,
                parent_page_id,
                slug,
                title,
                description,
                keywords,
                template,
                meta_robots,
                canonical_url,
                og_image_url,
                structured_data_type,
                position,
                sort_mode,
                created_at,
                updated_at,
            });
        }

        Ok(pages)
    }

    /// List all root-level pages for a site (pages with no parent)
    pub async fn list_root_pages(&self, site_id: i64) -> Result<Vec<Page>> {
        let results =
            sqlx::query_as::<_, (i64, i64, Option<i64>, String, String, Option<String>, Option<String>, String, String, Option<String>, Option<String>, String, i32, String, String, String)>(
                r#"
            SELECT id, site_id, parent_page_id, slug, title, description, keywords, template, meta_robots, canonical_url, og_image_url, structured_data_type, position, sort_mode, created_at, updated_at
            FROM pages
            WHERE site_id = ? AND parent_page_id IS NULL
            ORDER BY position, slug
            "#,
            )
            .bind(site_id)
            .fetch_all(&self.pool)
            .await
            .context("Failed to list root pages")?;

        let mut pages = Vec::new();
        for (
            id,
            site_id,
            parent_page_id,
            slug,
            title,
            description,
            keywords,
            template,
            meta_robots,
            canonical_url,
            og_image_url,
            structured_data_type,
            position,
            sort_mode,
            created_at_str,
            updated_at_str,
        ) in results
        {
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

            pages.push(Page {
                id: Some(id),
                site_id,
                parent_page_id,
                slug,
                title,
                description,
                keywords,
                template,
                meta_robots,
                canonical_url,
                og_image_url,
                structured_data_type,
                position,
                sort_mode,
                created_at,
                updated_at,
            });
        }

        Ok(pages)
    }

    pub async fn get_breadcrumb_trail(&self, page_id: i64) -> Result<Vec<Page>> {
        let mut trail = Vec::new();
        let mut current_id = Some(page_id);

        // Walk up the tree from the given page to the root
        while let Some(id) = current_id {
            match self.find_by_id(id).await? {
                Some(page) => {
                    current_id = page.parent_page_id;
                    trail.push(page);
                }
                None => break,
            }
        }

        // Reverse to get root-to-page order
        trail.reverse();
        Ok(trail)
    }

    pub async fn is_descendant_of(&self, page_id: i64, potential_ancestor_id: i64) -> Result<bool> {
        // A page cannot be a descendant of itself
        if page_id == potential_ancestor_id {
            return Ok(false);
        }

        // Get the page to check
        let page = match self.find_by_id(page_id).await? {
            Some(p) => p,
            None => return Ok(false), // Non-existent page is not a descendant
        };

        // If the page has no parent, it's not a descendant of anything
        let mut current_parent_id = match page.parent_page_id {
            Some(id) => id,
            None => return Ok(false),
        };

        // Walk up the tree looking for the potential ancestor
        while current_parent_id != potential_ancestor_id {
            match self.find_by_id(current_parent_id).await? {
                Some(parent) => {
                    match parent.parent_page_id {
                        Some(id) => current_parent_id = id,
                        None => return Ok(false), // Reached root without finding ancestor
                    }
                }
                None => return Ok(false), // Broken chain
            }
        }

        Ok(true)
    }

    // Transaction version of is_descendant_of for use within move_page
    async fn is_descendant_of_tx(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        page_id: i64,
        potential_ancestor_id: i64,
    ) -> Result<bool> {
        // A page cannot be a descendant of itself
        if page_id == potential_ancestor_id {
            return Ok(false);
        }

        // Get the page to check within transaction
        let page_parent =
            sqlx::query_scalar::<_, Option<i64>>("SELECT parent_page_id FROM pages WHERE id = ?")
                .bind(page_id)
                .fetch_optional(&mut **tx)
                .await
                .context("Failed to fetch page parent")?;

        // If page doesn't exist or has no parent, it's not a descendant of anything
        let mut current_parent_id = match page_parent {
            Some(Some(id)) => id,
            _ => return Ok(false),
        };

        // Walk up the tree looking for the potential ancestor
        while current_parent_id != potential_ancestor_id {
            let next_parent = sqlx::query_scalar::<_, Option<i64>>(
                "SELECT parent_page_id FROM pages WHERE id = ?",
            )
            .bind(current_parent_id)
            .fetch_optional(&mut **tx)
            .await
            .context("Failed to fetch parent")?;

            match next_parent {
                Some(Some(parent_id)) => current_parent_id = parent_id,
                _ => return Ok(false), // Reached root or broken chain
            }
        }

        Ok(true)
    }

    pub async fn get_all_descendants(&self, page_id: i64) -> Result<Vec<Page>> {
        let mut descendants = Vec::new();
        let mut pages_to_process = vec![page_id];

        while let Some(current_id) = pages_to_process.pop() {
            // Get all direct children of the current page
            let children = self.list_children(current_id).await?;

            for child in children {
                if let Some(child_id) = child.id {
                    pages_to_process.push(child_id);
                    descendants.push(child);
                }
            }
        }

        // Sort by position within each level (preserves hierarchical order)
        descendants.sort_by(|a, b| {
            // First compare by parent_page_id to group siblings together
            match (a.parent_page_id, b.parent_page_id) {
                (Some(a_parent), Some(b_parent)) => {
                    match a_parent.cmp(&b_parent) {
                        std::cmp::Ordering::Equal => {
                            // Same parent, sort by position
                            a.position.cmp(&b.position)
                        }
                        other => other,
                    }
                }
                (None, Some(_)) => std::cmp::Ordering::Less,
                (Some(_), None) => std::cmp::Ordering::Greater,
                (None, None) => a.position.cmp(&b.position),
            }
        });

        Ok(descendants)
    }

    pub async fn reorder_siblings(
        &self,
        parent_id: Option<i64>,
        positions: Vec<(i64, i32)>,
    ) -> Result<()> {
        // Start a transaction to ensure all updates are atomic
        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to begin transaction")?;

        for (page_id, new_position) in positions {
            // Verify the page exists and has the expected parent
            let verify_result: Option<(Option<i64>,)> = sqlx::query_as(
                r#"
                SELECT parent_page_id
                FROM pages
                WHERE id = ?
                "#,
            )
            .bind(page_id)
            .fetch_optional(&mut *tx)
            .await
            .context("Failed to verify page")?;

            match verify_result {
                Some((page_parent_id,)) => {
                    // Check if the parent matches
                    if page_parent_id != parent_id {
                        return Err(anyhow::anyhow!(
                            "Page {} does not have parent_page_id {:?}",
                            page_id,
                            parent_id
                        ));
                    }
                }
                None => {
                    return Err(anyhow::anyhow!("Page with id {} not found", page_id));
                }
            }

            // Update the position
            sqlx::query(
                r#"
                UPDATE pages
                SET position = ?, updated_at = datetime('now')
                WHERE id = ?
                "#,
            )
            .bind(new_position)
            .bind(page_id)
            .execute(&mut *tx)
            .await
            .context("Failed to update page position")?;
        }

        // Commit the transaction
        tx.commit().await.context("Failed to commit transaction")?;

        Ok(())
    }

    /// Update positions for a batch of pages (used for manual reordering)
    pub async fn update_positions(&self, positions: &[(i64, i32)]) -> Result<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to start transaction")?;

        for (page_id, position) in positions {
            sqlx::query!(
                r#"UPDATE pages SET position = ? WHERE id = ?"#,
                position,
                page_id
            )
            .execute(&mut *tx)
            .await
            .context("Failed to update page position")?;
        }

        tx.commit().await.context("Failed to commit transaction")?;
        Ok(())
    }

    pub async fn move_page(&self, page_id: i64, new_parent_id: i64) -> Result<()> {
        // Start a transaction
        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to start transaction")?;

        // Get the page to be moved (within transaction)
        let page = sqlx::query_as::<_, (i64, i64, Option<i64>, String, String, Option<String>, Option<String>, String, String, Option<String>, Option<String>, String, i32, String, String, String)>(
            r#"
            SELECT id, site_id, parent_page_id, slug, title, description, keywords, template, meta_robots, canonical_url, og_image_url, structured_data_type, position, sort_mode, created_at, updated_at
            FROM pages
            WHERE id = ?
            "#
        )
        .bind(page_id)
        .fetch_optional(&mut *tx)
        .await
        .context("Failed to fetch page")?
        .ok_or_else(|| anyhow::anyhow!("Page with id {} not found", page_id))?;

        let page = Page {
            id: Some(page.0),
            site_id: page.1,
            parent_page_id: page.2,
            slug: page.3,
            title: page.4,
            description: page.5,
            keywords: page.6,
            template: page.7,
            meta_robots: page.8,
            canonical_url: page.9,
            og_image_url: page.10,
            structured_data_type: page.11,
            position: page.12,
            sort_mode: page.13,
            created_at: chrono::DateTime::parse_from_rfc3339(&page.14)
                .unwrap_or_else(|_| {
                    chrono::NaiveDateTime::parse_from_str(&page.14, "%Y-%m-%d %H:%M:%S")
                        .unwrap_or_else(|_| chrono::Utc::now().naive_utc())
                        .and_utc()
                        .fixed_offset()
                })
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339(&page.15)
                .unwrap_or_else(|_| {
                    chrono::NaiveDateTime::parse_from_str(&page.15, "%Y-%m-%d %H:%M:%S")
                        .unwrap_or_else(|_| chrono::Utc::now().naive_utc())
                        .and_utc()
                        .fixed_offset()
                })
                .with_timezone(&chrono::Utc),
        };

        // Cannot move root page
        if page.parent_page_id.is_none() {
            return Err(anyhow::anyhow!("Cannot move root page"));
        }

        // Get the new parent (within transaction)
        let new_parent_site =
            sqlx::query_scalar::<_, i64>("SELECT site_id FROM pages WHERE id = ?")
                .bind(new_parent_id)
                .fetch_optional(&mut *tx)
                .await
                .context("Failed to check new parent")?;

        match new_parent_site {
            None => {
                return Err(anyhow::anyhow!(
                    "New parent page with id {} not found",
                    new_parent_id
                ))
            }
            Some(parent_site_id) if parent_site_id != page.site_id => {
                return Err(anyhow::anyhow!("Cannot move page to a different site"));
            }
            Some(_) => {} // Same site, continue
        }

        // Check if already at this parent (no-op)
        if page.parent_page_id == Some(new_parent_id) {
            return Ok(());
        }

        // Cannot move to itself
        if page_id == new_parent_id {
            return Err(anyhow::anyhow!("Cannot move page to itself"));
        }

        // Cannot move to a descendant - check within transaction
        let is_descendant = self
            .is_descendant_of_tx(&mut tx, new_parent_id, page_id)
            .await?;
        if is_descendant {
            return Err(anyhow::anyhow!(
                "Cannot move page to one of its descendants"
            ));
        }

        // Check for slug conflict at destination
        let conflict = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT id 
            FROM pages 
            WHERE site_id = ? AND parent_page_id = ? AND slug = ? AND id != ?
            "#,
        )
        .bind(page.site_id)
        .bind(new_parent_id)
        .bind(&page.slug)
        .bind(page_id)
        .fetch_optional(&mut *tx)
        .await
        .context("Failed to check for slug conflict")?;

        if conflict.is_some() {
            return Err(anyhow::anyhow!(
                "A page with slug '{}' already exists under the new parent",
                page.slug
            ));
        }

        // Get the max position at destination
        let max_position = sqlx::query_scalar::<_, Option<i32>>(
            "SELECT MAX(position) FROM pages WHERE parent_page_id = ?",
        )
        .bind(new_parent_id)
        .fetch_one(&mut *tx)
        .await
        .context("Failed to get max position")?;

        let new_position = match max_position {
            Some(max) => max + 1,
            None => 0,
        };

        // Update the page
        sqlx::query(
            r#"
            UPDATE pages 
            SET parent_page_id = ?, position = ?, updated_at = datetime('now')
            WHERE id = ?
            "#,
        )
        .bind(new_parent_id)
        .bind(new_position)
        .bind(page_id)
        .execute(&mut *tx)
        .await
        .context("Failed to move page")?;

        // Commit the transaction
        tx.commit().await.context("Failed to commit transaction")?;

        Ok(())
    }

    pub async fn get_root_page(&self, site_id: i64) -> Result<Option<Page>> {
        let result =
            sqlx::query_as::<_, (i64, i64, Option<i64>, String, String, Option<String>, Option<String>, String, String, Option<String>, Option<String>, String, i32, String, String, String)>(
                r#"
            SELECT id, site_id, parent_page_id, slug, title, description, keywords, template, meta_robots, canonical_url, og_image_url, structured_data_type, position, sort_mode, created_at, updated_at
            FROM pages
            WHERE site_id = ? AND parent_page_id IS NULL
            "#,
            )
            .bind(site_id)
            .fetch_optional(&self.pool)
            .await
            .context("Failed to get root page")?;

        match result {
            Some((
                id,
                site_id,
                parent_page_id,
                slug,
                title,
                description,
                keywords,
                template,
                meta_robots,
                canonical_url,
                og_image_url,
                structured_data_type,
                position,
                sort_mode,
                created_at_str,
                updated_at_str,
            )) => {
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

                Ok(Some(Page {
                    id: Some(id),
                    site_id,
                    parent_page_id,
                    slug,
                    title,
                    description,
                    keywords,
                    template,
                    meta_robots,
                    canonical_url,
                    og_image_url,
                    structured_data_type,
                    position,
                    sort_mode,
                    created_at,
                    updated_at,
                }))
            }
            None => Ok(None),
        }
    }

    /// Get all pages that are valid move targets for the given page
    /// A page cannot be moved to itself or any of its descendants
    pub async fn get_valid_move_targets(&self, page_id: i64) -> Result<Vec<Page>> {
        // Get the page to check its site and current parent
        let page = self
            .find_by_id(page_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Page with id {} not found", page_id))?;

        // Get all pages in the same site
        let all_pages = self.list_by_site_id(page.site_id).await?;

        // Get all descendants of the current page
        let descendants = self.get_all_descendants(page_id).await?;
        let descendant_ids: std::collections::HashSet<i64> =
            descendants.iter().filter_map(|p| p.id).collect();

        // Filter out:
        // 1. The page itself
        // 2. All its descendants
        // 3. Its current parent (no-op move)
        let valid_targets: Vec<Page> = all_pages
            .into_iter()
            .filter(|p| {
                if let Some(id) = p.id {
                    // Exclude the page itself
                    id != page_id
                    // Exclude all descendants
                    && !descendant_ids.contains(&id)
                    // Exclude the current parent (if it exists)
                    && Some(id) != page.parent_page_id
                } else {
                    false
                }
            })
            .collect();

        Ok(valid_targets)
    }

    pub async fn has_children(&self, page_id: i64) -> Result<bool> {
        let row = sqlx::query!(
            r#"SELECT COUNT(*) as "count: i64" FROM pages WHERE parent_page_id = ?"#,
            page_id
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to check for child pages")?;

        Ok(row.count > 0)
    }

    /// Generate a unique slug for a page under the given parent
    pub async fn generate_unique_slug(
        &self,
        site_id: i64,
        parent_page_id: Option<i64>,
        base_slug: &str,
    ) -> Result<String> {
        let mut slug = base_slug.to_string();
        let mut suffix = 1;

        loop {
            // Check if this slug already exists under the same parent
            let exists = sqlx::query!(
                r#"
                SELECT COUNT(*) as "count: i64" 
                FROM pages 
                WHERE site_id = ? AND parent_page_id IS ? AND slug = ?
                "#,
                site_id,
                parent_page_id,
                slug
            )
            .fetch_one(&self.pool)
            .await
            .context("Failed to check slug existence")?;

            if exists.count == 0 {
                return Ok(slug);
            }

            // Generate new slug with suffix
            suffix += 1;
            slug = format!("{}-{}", base_slug, suffix);
        }
    }

    /// Create a page with auto-generated slug if needed
    pub async fn create_with_auto_slug(&self, page: &mut Page) -> Result<i64> {
        // If slug is empty, generate from title
        if page.slug.is_empty() {
            page.slug = generate_slug_from_title(&page.title);
        }

        // Ensure slug is unique
        page.slug = self
            .generate_unique_slug(page.site_id, page.parent_page_id, &page.slug)
            .await?;

        // Create the page with the unique slug
        self.create(page).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repositories::site_repository::SiteRepository;
    use doxyde_core::models::site::Site;

    #[sqlx::test]
    async fn test_new_creates_repository() -> Result<(), sqlx::Error> {
        // The pool is provided by sqlx::test attribute
        let pool = SqlitePool::connect(":memory:").await?;

        let repo = PageRepository::new(pool.clone());

        // Verify we can access the pool (it's stored correctly)
        // We'll do a simple query to ensure the connection works
        let _result = sqlx::query("SELECT 1").fetch_one(&repo.pool).await?;

        Ok(())
    }

    // Helper function to get the root page ID for a site
    async fn get_root_page_id(pool: &SqlitePool, site_id: i64) -> Result<i64> {
        let repo = PageRepository::new(pool.clone());
        let root_page = repo
            .get_root_page(site_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found for site {}", site_id))?;
        root_page
            .id
            .ok_or_else(|| anyhow::anyhow!("Root page has no ID"))
    }

    async fn setup_test_db(pool: &SqlitePool) -> Result<i64> {
        // Enable foreign key constraints in SQLite
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(pool)
            .await?;

        // Create sites table first
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

        // Create pages table with foreign key to sites
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
                sort_mode TEXT NOT NULL DEFAULT 'created_at_asc',
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

        // Create components table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS components (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                page_version_id INTEGER NOT NULL,
                component_type TEXT NOT NULL,
                position INTEGER NOT NULL,
                title TEXT,
                template TEXT DEFAULT 'default',
                content TEXT NOT NULL,
                style_options TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                FOREIGN KEY (page_version_id) REFERENCES page_versions(id) ON DELETE CASCADE
            )
            "#,
        )
        .execute(pool)
        .await?;

        // Create a test site using SiteRepository to ensure root page is created
        let site_repo = SiteRepository::new(pool.clone());
        let site = Site::new("test-site.com".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;

        Ok(site_id)
    }

    #[sqlx::test]
    async fn test_create_page_success() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        let repo = PageRepository::new(pool);

        // Get the root page that was created automatically
        let root_page = repo.get_root_page(site_id).await?.unwrap();

        // Create a child page under the root
        let page = Page::new_with_parent(
            site_id,
            root_page.id.unwrap(),
            "about".to_string(),
            "About Us".to_string(),
        );

        let id = repo.create(&page).await?;

        // Verify ID is valid
        assert!(id > 0);

        // Verify the page was actually inserted (1 root + 1 new page)
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pages")
            .fetch_one(&repo.pool)
            .await?;
        assert_eq!(row.0, 2);

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_page_with_all_fields() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        let repo = PageRepository::new(pool);

        // Get the root page that was created automatically
        let root_page = repo.get_root_page(site_id).await?.unwrap();

        // Create a child page under the root
        let page = Page::new_with_parent(
            site_id,
            root_page.id.unwrap(),
            "services".to_string(),
            "Our Services".to_string(),
        );

        let id = repo.create(&page).await?;

        // Verify the data was inserted correctly
        let row: (i64, Option<i64>, String, String, i32, String, String) = sqlx::query_as(
            r#"
            SELECT site_id, parent_page_id, slug, title, position, created_at, updated_at 
            FROM pages 
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_one(&repo.pool)
        .await?;

        assert_eq!(row.0, page.site_id);
        assert_eq!(row.1, page.parent_page_id);
        assert_eq!(row.2, page.slug);
        assert_eq!(row.3, page.title);
        assert_eq!(row.4, page.position);
        assert!(!row.5.is_empty());
        assert!(!row.6.is_empty());

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_page_with_parent() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        let repo = PageRepository::new(pool.clone());

        // Get the root page that was created automatically
        let root_page_id = get_root_page_id(&pool, site_id).await?;

        // Create parent page as child of root
        let parent_page = Page::new_with_parent(
            site_id,
            root_page_id,
            "parent".to_string(),
            "Parent Page".to_string(),
        );
        let parent_id = repo.create(&parent_page).await?;

        // Create child page
        let mut child_page = Page::new_with_parent(
            site_id,
            parent_id,
            "child".to_string(),
            "Child Page".to_string(),
        );
        child_page.position = 1;
        let child_id = repo.create(&child_page).await?;

        // Verify the child page data
        let row: (i64, Option<i64>, String, String, i32) = sqlx::query_as(
            r#"
            SELECT site_id, parent_page_id, slug, title, position
            FROM pages 
            WHERE id = ?
            "#,
        )
        .bind(child_id)
        .fetch_one(&repo.pool)
        .await?;

        assert_eq!(row.0, site_id);
        assert_eq!(row.1, Some(parent_id));
        assert_eq!(row.2, "child");
        assert_eq!(row.3, "Child Page");
        assert_eq!(row.4, 1);

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_page_duplicate_slug_same_site_fails() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        let repo = PageRepository::new(pool.clone());

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        // Create two pages with same slug under same parent
        let page1 = Page::new_with_parent(
            site_id,
            root_id,
            "duplicate".to_string(),
            "First Page".to_string(),
        );
        let page2 = Page::new_with_parent(
            site_id,
            root_id,
            "duplicate".to_string(),
            "Second Page".to_string(),
        );

        // First insert should succeed
        let id1 = repo.create(&page1).await?;
        assert!(id1 > 0);

        // Second insert with same slug and parent should fail
        let result = repo.create(&page2).await;
        assert!(result.is_err());

        // Verify error message contains context
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Failed to create page"));

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_page_same_slug_different_sites_succeeds() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id1 = setup_test_db(&pool).await?;

        // Create a second site using SiteRepository to ensure root page is created
        let site_repo = SiteRepository::new(pool.clone());
        let site2 = Site::new("test-site-2.com".to_string(), "Test Site 2".to_string());
        let site_id2 = site_repo.create(&site2).await?;

        // Get root pages for both sites
        let root_id1 = get_root_page_id(&pool, site_id1).await?;
        let root_id2 = get_root_page_id(&pool, site_id2).await?;

        let repo = PageRepository::new(pool.clone());
        let page1 = Page::new_with_parent(
            site_id1,
            root_id1,
            "about".to_string(),
            "About Site 1".to_string(),
        );
        let page2 = Page::new_with_parent(
            site_id2,
            root_id2,
            "about".to_string(),
            "About Site 2".to_string(),
        );

        // Both inserts should succeed
        let id1 = repo.create(&page1).await?;
        let id2 = repo.create(&page2).await?;

        assert!(id1 > 0);
        assert!(id2 > 0);
        assert_ne!(id1, id2);

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_page_with_non_existent_site_fails() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());
        // Try to create a page with non-existent site but valid parent
        let page = Page::new_with_parent(999, root_id, "test".to_string(), "Test Page".to_string());

        // Should fail due to foreign key constraint
        let result = repo.create(&page).await;
        assert!(result.is_err());

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_multiple_pages() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        let repo = PageRepository::new(pool.clone());

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        // Create child pages
        let pages = vec![
            Page::new_with_parent(
                site_id,
                root_id,
                "page1".to_string(),
                "Page One".to_string(),
            ),
            Page::new_with_parent(
                site_id,
                root_id,
                "page2".to_string(),
                "Page Two".to_string(),
            ),
            Page::new_with_parent(
                site_id,
                root_id,
                "page3".to_string(),
                "Page Three".to_string(),
            ),
        ];

        let mut ids = Vec::new();
        for page in &pages {
            let id = repo.create(page).await?;
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

        // Verify count (including root page)
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pages")
            .fetch_one(&repo.pool)
            .await?;
        assert_eq!(row.0, 4); // 1 root + 3 child pages

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_id_existing_page() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());
        let page = Page::new_with_parent(
            site_id,
            root_id,
            "findme".to_string(),
            "Find Me Page".to_string(),
        );

        // Create the page first
        let id = repo.create(&page).await?;

        // Find it by ID
        let found = repo.find_by_id(id).await?;

        assert!(found.is_some());
        let found_page = found.unwrap();
        assert_eq!(found_page.id, Some(id));
        assert_eq!(found_page.site_id, page.site_id);
        assert_eq!(found_page.slug, page.slug);
        assert_eq!(found_page.title, page.title);

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_id_non_existing_page() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = PageRepository::new(pool);

        // Try to find a non-existing page
        let found = repo.find_by_id(999).await?;

        assert!(found.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_id_with_timestamps() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());
        let page = Page::new_with_parent(
            site_id,
            root_id,
            "timestamp".to_string(),
            "Timestamp Page".to_string(),
        );

        // Store original timestamps
        let original_created = page.created_at;
        let original_updated = page.updated_at;

        // Create the page
        let id = repo.create(&page).await?;

        // Find it by ID
        let found = repo.find_by_id(id).await?;

        assert!(found.is_some());
        let found_page = found.unwrap();

        // Timestamps should be close to the originals (within 1 second)
        let created_diff = (found_page.created_at - original_created)
            .num_seconds()
            .abs();
        let updated_diff = (found_page.updated_at - original_updated)
            .num_seconds()
            .abs();

        assert!(created_diff <= 1);
        assert!(updated_diff <= 1);

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_id_multiple_pages() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());

        // Create multiple child pages
        let pages = vec![
            Page::new_with_parent(
                site_id,
                root_id,
                "first".to_string(),
                "First Page".to_string(),
            ),
            Page::new_with_parent(
                site_id,
                root_id,
                "second".to_string(),
                "Second Page".to_string(),
            ),
            Page::new_with_parent(
                site_id,
                root_id,
                "third".to_string(),
                "Third Page".to_string(),
            ),
        ];

        let mut ids = Vec::new();
        for page in &pages {
            ids.push(repo.create(page).await?);
        }

        // Find each page by its ID
        for (i, id) in ids.iter().enumerate() {
            let found = repo.find_by_id(*id).await?;
            assert!(found.is_some());

            let found_page = found.unwrap();
            assert_eq!(found_page.id, Some(*id));
            assert_eq!(found_page.site_id, pages[i].site_id);
            assert_eq!(found_page.slug, pages[i].slug);
            assert_eq!(found_page.title, pages[i].title);
        }

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_id_zero_and_negative() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = PageRepository::new(pool);

        // Test with ID 0
        let found = repo.find_by_id(0).await?;
        assert!(found.is_none());

        // Test with negative ID
        let found = repo.find_by_id(-1).await?;
        assert!(found.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_id_after_delete() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());
        let page = Page::new_with_parent(
            site_id,
            root_id,
            "deleteme".to_string(),
            "Delete Me Page".to_string(),
        );

        // Create the page
        let id = repo.create(&page).await?;

        // Verify it exists
        let found = repo.find_by_id(id).await?;
        assert!(found.is_some());

        // Delete it using raw SQL (since delete() isn't implemented yet)
        sqlx::query("DELETE FROM pages WHERE id = ?")
            .bind(id)
            .execute(&pool)
            .await?;

        // Try to find it again
        let not_found = repo.find_by_id(id).await?;
        assert!(not_found.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_slug_and_site_id_existing_page() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());
        let page = Page::new_with_parent(
            site_id,
            root_id,
            "about-us".to_string(),
            "About Us Page".to_string(),
        );

        // Create the page first
        let id = repo.create(&page).await?;

        // Find it by slug and site_id
        let found = repo.find_by_slug_and_site_id("about-us", site_id).await?;

        assert!(found.is_some());
        let found_page = found.unwrap();
        assert_eq!(found_page.id, Some(id));
        assert_eq!(found_page.site_id, page.site_id);
        assert_eq!(found_page.slug, page.slug);
        assert_eq!(found_page.title, page.title);

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_slug_and_site_id_non_existing_page() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        let repo = PageRepository::new(pool);

        // Try to find a non-existing page
        let found = repo
            .find_by_slug_and_site_id("nonexistent", site_id)
            .await?;

        assert!(found.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_slug_and_site_id_wrong_site() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id1 = setup_test_db(&pool).await?;

        // Create a second site using SiteRepository to ensure root page is created
        let site_repo = SiteRepository::new(pool.clone());
        let site2 = Site::new("test-site-2.com".to_string(), "Test Site 2".to_string());
        let site_id2 = site_repo.create(&site2).await?;

        // Get the root page for site1
        let root_id1 = get_root_page_id(&pool, site_id1).await?;

        let repo = PageRepository::new(pool.clone());
        let page = Page::new_with_parent(
            site_id1,
            root_id1,
            "test-page".to_string(),
            "Test Page".to_string(),
        );

        // Create the page for site1
        repo.create(&page).await?;

        // Try to find it with site2's ID
        let found = repo.find_by_slug_and_site_id("test-page", site_id2).await?;

        assert!(found.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_slug_and_site_id_same_slug_different_sites() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id1 = setup_test_db(&pool).await?;

        // Create a second site using SiteRepository to ensure root page is created
        let site_repo = SiteRepository::new(pool.clone());
        let site2 = Site::new("test-site-2.com".to_string(), "Test Site 2".to_string());
        let site_id2 = site_repo.create(&site2).await?;

        // Get root pages for both sites
        let root_id1 = get_root_page_id(&pool, site_id1).await?;
        let root_id2 = get_root_page_id(&pool, site_id2).await?;

        let repo = PageRepository::new(pool.clone());
        let page1 = Page::new_with_parent(
            site_id1,
            root_id1,
            "about".to_string(),
            "About Site 1".to_string(),
        );
        let page2 = Page::new_with_parent(
            site_id2,
            root_id2,
            "about".to_string(),
            "About Site 2".to_string(),
        );

        // Create both pages with same slug but different sites
        let id1 = repo.create(&page1).await?;
        let id2 = repo.create(&page2).await?;

        // Find first page
        let found1 = repo.find_by_slug_and_site_id("about", site_id1).await?;
        assert!(found1.is_some());
        assert_eq!(found1.unwrap().id, Some(id1));

        // Find second page
        let found2 = repo.find_by_slug_and_site_id("about", site_id2).await?;
        assert!(found2.is_some());
        assert_eq!(found2.unwrap().id, Some(id2));

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_slug_and_site_id_case_sensitive() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());
        let page = Page::new_with_parent(
            site_id,
            root_id,
            "CaseSensitive".to_string(),
            "Case Sensitive Page".to_string(),
        );

        // Create the page
        repo.create(&page).await?;

        // Try to find with exact case - should work
        let found = repo
            .find_by_slug_and_site_id("CaseSensitive", site_id)
            .await?;
        assert!(found.is_some());

        // Try to find with different case - SQLite is case-sensitive by default
        let found_lower = repo
            .find_by_slug_and_site_id("casesensitive", site_id)
            .await?;
        assert!(found_lower.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_slug_and_site_id_with_special_characters() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());

        // Test with various special characters in slug
        let test_slugs = vec![
            "sub-page",
            "under_score",
            "page.html",
            "dir/nested",
            "123numbers",
        ];

        for slug in test_slugs {
            let page = Page::new_with_parent(
                site_id,
                root_id,
                slug.to_string(),
                format!("Page for {}", slug),
            );
            repo.create(&page).await?;

            let found = repo.find_by_slug_and_site_id(slug, site_id).await?;
            assert!(found.is_some(), "Should find slug: {}", slug);
            assert_eq!(found.unwrap().slug, slug);
        }

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_slug_and_site_id_empty_slug() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        let repo = PageRepository::new(pool);

        // Empty slug should find the root page
        let found = repo.find_by_slug_and_site_id("", site_id).await?;
        assert!(found.is_some());
        let page = found.unwrap();
        assert_eq!(page.slug, "");
        assert!(page.parent_page_id.is_none()); // It's the root page

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_slug_and_site_id_non_existent_site() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = PageRepository::new(pool);

        // Try to find with non-existent site ID
        let found = repo.find_by_slug_and_site_id("any-slug", 999).await?;
        assert!(found.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_by_site_id_no_pages() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        let repo = PageRepository::new(pool.clone());

        // List pages for a site (should have 1 root page)
        let pages = repo.list_by_site_id(site_id).await?;

        assert_eq!(pages.len(), 1); // Root page exists

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_by_site_id_single_page() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());
        let page = Page::new_with_parent(
            site_id,
            root_id,
            "about".to_string(),
            "About Page".to_string(),
        );

        // Create a page
        let id = repo.create(&page).await?;

        // List pages (should have root + new page)
        let pages = repo.list_by_site_id(site_id).await?;

        assert_eq!(pages.len(), 2); // root + about
                                    // Find the about page in the list
        let about_page = pages.iter().find(|p| p.slug == "about").unwrap();
        assert_eq!(about_page.id, Some(id));
        assert_eq!(about_page.title, "About Page");

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_by_site_id_multiple_pages_ordered() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());

        // Create pages in non-alphabetical order
        let pages_data = vec![
            ("contact", "Contact Us"),
            ("about", "About Us"),
            ("services", "Our Services"),
            ("blog", "Blog"),
        ];

        for (slug, title) in &pages_data {
            let page = Page::new_with_parent(site_id, root_id, slug.to_string(), title.to_string());
            repo.create(&page).await?;
        }

        // List pages
        let pages = repo.list_by_site_id(site_id).await?;

        // Verify they are ordered by slug (including root)
        assert_eq!(pages.len(), 5);
        // Root page slug is empty string
        assert_eq!(pages[0].slug, ""); // root page
        assert_eq!(pages[1].slug, "about");
        assert_eq!(pages[2].slug, "blog");
        assert_eq!(pages[3].slug, "contact");
        assert_eq!(pages[4].slug, "services");

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_by_site_id_multiple_sites() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id1 = setup_test_db(&pool).await?;

        // Create a second site using SiteRepository to ensure root page is created
        let site_repo = SiteRepository::new(pool.clone());
        let site2 = Site::new("test-site-2.com".to_string(), "Test Site 2".to_string());
        let site_id2 = site_repo.create(&site2).await?;

        // Get root pages for both sites
        let root1_id = get_root_page_id(&pool, site_id1).await?;
        let root2_id = get_root_page_id(&pool, site_id2).await?;

        let repo = PageRepository::new(pool.clone());

        // Create child pages for site1
        let site1_pages = vec![
            Page::new_with_parent(
                site_id1,
                root1_id,
                "page1".to_string(),
                "Site 1 Page 1".to_string(),
            ),
            Page::new_with_parent(
                site_id1,
                root1_id,
                "page2".to_string(),
                "Site 1 Page 2".to_string(),
            ),
        ];

        // Create child pages for site2
        let site2_pages = vec![
            Page::new_with_parent(
                site_id2,
                root2_id,
                "page1".to_string(),
                "Site 2 Page 1".to_string(),
            ),
            Page::new_with_parent(
                site_id2,
                root2_id,
                "page2".to_string(),
                "Site 2 Page 2".to_string(),
            ),
            Page::new_with_parent(
                site_id2,
                root2_id,
                "page3".to_string(),
                "Site 2 Page 3".to_string(),
            ),
        ];

        for page in &site1_pages {
            repo.create(page).await?;
        }
        for page in &site2_pages {
            repo.create(page).await?;
        }

        // List pages for site1 (including root)
        let pages1 = repo.list_by_site_id(site_id1).await?;
        assert_eq!(pages1.len(), 3); // 1 root + 2 child pages
        for page in &pages1 {
            assert_eq!(page.site_id, site_id1);
        }

        // List pages for site2 (including root)
        let pages2 = repo.list_by_site_id(site_id2).await?;
        assert_eq!(pages2.len(), 4); // 1 root + 3 child pages
        for page in &pages2 {
            assert_eq!(page.site_id, site_id2);
        }

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_by_site_id_non_existent_site() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = PageRepository::new(pool);

        // List pages for non-existent site
        let pages = repo.list_by_site_id(999).await?;

        assert!(pages.is_empty());

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_by_site_id_with_special_characters() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());

        // Create pages with special characters
        let pages_data = vec![
            ("sub-page", "Sub Page"),
            ("under_score", "Under Score"),
            ("page.html", "HTML Page"),
            ("dir/nested", "Nested Page"),
            ("123numbers", "Numbers Page"),
        ];

        for (slug, title) in &pages_data {
            let page = Page::new_with_parent(site_id, root_id, slug.to_string(), title.to_string());
            repo.create(&page).await?;
        }

        // List pages
        let pages = repo.list_by_site_id(site_id).await?;

        assert_eq!(pages.len(), 6); // 1 root + 5 child pages
                                    // Verify ordering
        assert_eq!(pages[0].slug, ""); // root page
        assert_eq!(pages[1].slug, "123numbers");
        assert_eq!(pages[2].slug, "dir/nested");
        assert_eq!(pages[3].slug, "page.html");
        assert_eq!(pages[4].slug, "sub-page");
        assert_eq!(pages[5].slug, "under_score");

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_by_site_id_after_delete() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());

        // Create three child pages
        let pages_data = vec![
            ("page1", "Page 1"),
            ("page2", "Page 2"),
            ("page3", "Page 3"),
        ];

        let mut ids = Vec::new();
        for (slug, title) in &pages_data {
            let page = Page::new_with_parent(site_id, root_id, slug.to_string(), title.to_string());
            ids.push(repo.create(&page).await?);
        }

        // Delete the middle page
        sqlx::query("DELETE FROM pages WHERE id = ?")
            .bind(ids[1])
            .execute(&pool)
            .await?;

        // List pages
        let pages = repo.list_by_site_id(site_id).await?;

        assert_eq!(pages.len(), 3); // 1 root + 2 remaining child pages
        assert_eq!(pages[0].slug, ""); // root page
        assert_eq!(pages[1].slug, "page1");
        assert_eq!(pages[2].slug, "page3");

        Ok(())
    }

    #[sqlx::test]
    async fn test_update_existing_page() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());
        let mut page = Page::new_with_parent(
            site_id,
            root_id,
            "original".to_string(),
            "Original Title".to_string(),
        );

        // Create the page first
        let id = repo.create(&page).await?;
        page.id = Some(id);

        // Update the page
        page.slug = "updated".to_string();
        page.title = "Updated Title".to_string();
        page.position = 5;
        page.updated_at = chrono::Utc::now();

        repo.update(&page).await?;

        // Verify the update
        let found = repo.find_by_id(id).await?;
        assert!(found.is_some());

        let updated_page = found.unwrap();
        assert_eq!(updated_page.slug, "updated");
        assert_eq!(updated_page.title, "Updated Title");
        assert_eq!(updated_page.position, 5);
        assert!(updated_page.updated_at > page.created_at);

        Ok(())
    }

    #[sqlx::test]
    async fn test_update_page_with_parent() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());

        // Create parent page as child of root
        let parent_page = Page::new_with_parent(
            site_id,
            root_id,
            "parent".to_string(),
            "Parent Page".to_string(),
        );
        let parent_id = repo.create(&parent_page).await?;

        // Create another page as child of root
        let mut page = Page::new_with_parent(
            site_id,
            root_id,
            "child".to_string(),
            "Child Page".to_string(),
        );
        let id = repo.create(&page).await?;
        page.id = Some(id);

        // Update to add parent
        page.parent_page_id = Some(parent_id);
        page.updated_at = chrono::Utc::now();
        repo.update(&page).await?;

        // Verify the update
        let found = repo.find_by_id(id).await?;
        assert!(found.is_some());
        assert_eq!(found.unwrap().parent_page_id, Some(parent_id));

        Ok(())
    }

    #[sqlx::test]
    async fn test_update_non_existing_page() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        let repo = PageRepository::new(pool);
        let mut page = Page::new(site_id, "test".to_string(), "Test".to_string());
        page.id = Some(999); // Non-existent ID

        let result = repo.update(&page).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Page with id 999 not found"));

        Ok(())
    }

    #[sqlx::test]
    async fn test_update_page_without_id() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        let repo = PageRepository::new(pool);
        let page = Page::new(site_id, "test".to_string(), "Test".to_string());
        // page.id is None

        let result = repo.update(&page).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot update page without ID"));

        Ok(())
    }

    #[sqlx::test]
    async fn test_update_with_duplicate_slug_fails() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());

        // Create two child pages
        let page1 =
            Page::new_with_parent(site_id, root_id, "page1".to_string(), "Page 1".to_string());
        let page2 =
            Page::new_with_parent(site_id, root_id, "page2".to_string(), "Page 2".to_string());

        let _id1 = repo.create(&page1).await?;
        let id2 = repo.create(&page2).await?;

        // Try to update page2 with page1's slug
        let mut page2_update = page2.clone();
        page2_update.id = Some(id2);
        page2_update.slug = "page1".to_string(); // Duplicate!

        let result = repo.update(&page2_update).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to update page"));

        // Verify page2 wasn't changed
        let unchanged = repo.find_by_id(id2).await?.unwrap();
        assert_eq!(unchanged.slug, "page2");

        Ok(())
    }

    #[sqlx::test]
    async fn test_update_preserves_created_at() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());
        let mut page = Page::new_with_parent(
            site_id,
            root_id,
            "preserve".to_string(),
            "Original".to_string(),
        );
        let original_created_at = page.created_at;

        // Create the page
        let id = repo.create(&page).await?;
        page.id = Some(id);

        // Wait a moment to ensure updated_at will be different
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // Update the page
        page.title = "Updated".to_string();
        page.updated_at = chrono::Utc::now();
        repo.update(&page).await?;

        // Verify created_at is preserved
        let found = repo.find_by_id(id).await?.unwrap();
        let created_diff = (found.created_at - original_created_at).num_seconds().abs();
        assert!(created_diff <= 1); // Should be the same (within 1 second tolerance)
        assert!(found.updated_at > found.created_at);

        Ok(())
    }

    #[sqlx::test]
    async fn test_update_parent_to_different_page() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());

        // Create three child pages
        let page1 =
            Page::new_with_parent(site_id, root_id, "page1".to_string(), "Page 1".to_string());
        let page2 =
            Page::new_with_parent(site_id, root_id, "page2".to_string(), "Page 2".to_string());
        let mut page3 =
            Page::new_with_parent(site_id, root_id, "page3".to_string(), "Page 3".to_string());

        let id1 = repo.create(&page1).await?;
        let id2 = repo.create(&page2).await?;
        let id3 = repo.create(&page3).await?;
        page3.id = Some(id3);

        // First set page3's parent to page1
        page3.parent_page_id = Some(id1);
        page3.updated_at = chrono::Utc::now();
        repo.update(&page3).await?;

        // Verify parent is page1
        let found = repo.find_by_id(id3).await?.unwrap();
        assert_eq!(found.parent_page_id, Some(id1));

        // Now change parent to page2
        page3.parent_page_id = Some(id2);
        page3.updated_at = chrono::Utc::now();
        repo.update(&page3).await?;

        // Verify parent is now page2
        let found = repo.find_by_id(id3).await?.unwrap();
        assert_eq!(found.parent_page_id, Some(id2));

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_existing_page() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());

        // Create a child page to delete
        let page = Page::new_with_parent(
            site_id,
            root_id,
            "deleteme".to_string(),
            "Delete Me Page".to_string(),
        );

        // Create the page first
        let id = repo.create(&page).await?;

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
    async fn test_delete_non_existing_page() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = PageRepository::new(pool);

        // Try to delete a non-existing page
        let result = repo.delete(999).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Page with id 999 not found"));

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_page_cascades_to_children() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());

        // Create parent page as a child of root
        let parent = Page::new_with_parent(
            site_id,
            root_id,
            "parent".to_string(),
            "Parent Page".to_string(),
        );
        let parent_id = repo.create(&parent).await?;

        // Create child pages
        let child1 = Page::new_with_parent(
            site_id,
            parent_id,
            "child1".to_string(),
            "Child 1".to_string(),
        );
        let child2 = Page::new_with_parent(
            site_id,
            parent_id,
            "child2".to_string(),
            "Child 2".to_string(),
        );

        let child1_id = repo.create(&child1).await?;
        let child2_id = repo.create(&child2).await?;

        // Verify all exist
        assert!(repo.find_by_id(parent_id).await?.is_some());
        assert!(repo.find_by_id(child1_id).await?.is_some());
        assert!(repo.find_by_id(child2_id).await?.is_some());

        // Try to delete the parent - should fail because it has children
        let result = repo.delete(parent_id).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot delete page"));

        // Delete children first
        repo.delete(child1_id).await?;
        repo.delete(child2_id).await?;

        // Now we can delete the parent
        repo.delete(parent_id).await?;

        // Verify parent is deleted
        assert!(repo.find_by_id(parent_id).await?.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_child_page_keeps_parent() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());

        // Create parent as child of root
        let parent = Page::new_with_parent(
            site_id,
            root_id,
            "parent".to_string(),
            "Parent Page".to_string(),
        );
        let parent_id = repo.create(&parent).await?;

        let child = Page::new_with_parent(
            site_id,
            parent_id,
            "child".to_string(),
            "Child Page".to_string(),
        );
        let child_id = repo.create(&child).await?;

        // Delete the child
        repo.delete(child_id).await?;

        // Verify child is gone but parent remains
        assert!(repo.find_by_id(child_id).await?.is_none());
        assert!(repo.find_by_id(parent_id).await?.is_some());

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_multiple_pages() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());

        // Create multiple child pages
        let pages = vec![
            Page::new_with_parent(
                site_id,
                root_id,
                "delete1".to_string(),
                "Delete 1".to_string(),
            ),
            Page::new_with_parent(
                site_id,
                root_id,
                "delete2".to_string(),
                "Delete 2".to_string(),
            ),
            Page::new_with_parent(
                site_id,
                root_id,
                "delete3".to_string(),
                "Delete 3".to_string(),
            ),
        ];

        let mut ids = Vec::new();
        for page in &pages {
            ids.push(repo.create(page).await?);
        }

        // Delete pages one by one
        for id in &ids {
            repo.delete(*id).await?;
        }

        // Verify all are deleted
        for id in &ids {
            let found = repo.find_by_id(*id).await?;
            assert!(found.is_none());
        }

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_already_deleted_page() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());

        // Create a child page that we'll delete
        let page = Page::new_with_parent(
            site_id,
            root_id,
            "double-delete".to_string(),
            "Double Delete".to_string(),
        );

        // Create and delete the page
        let id = repo.create(&page).await?;
        repo.delete(id).await?;

        // Try to delete again
        let result = repo.delete(id).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains(&format!("Page with id {} not found", id)));

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_zero_and_negative_ids() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = PageRepository::new(pool);

        // Try to delete with ID 0
        let result = repo.delete(0).await;
        assert!(result.is_err());

        // Try to delete with negative ID
        let result = repo.delete(-1).await;
        assert!(result.is_err());

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_children_no_children() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());
        let parent = Page::new_with_parent(
            site_id,
            root_id,
            "parent".to_string(),
            "Parent Page".to_string(),
        );
        let parent_id = repo.create(&parent).await?;

        // List children when there are none
        let children = repo.list_children(parent_id).await?;
        assert!(children.is_empty());

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_children_single_child() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());

        // Create parent as child of root
        let parent = Page::new_with_parent(
            site_id,
            root_id,
            "parent".to_string(),
            "Parent Page".to_string(),
        );
        let parent_id = repo.create(&parent).await?;

        // Create child
        let child = Page::new_with_parent(
            site_id,
            parent_id,
            "child".to_string(),
            "Child Page".to_string(),
        );
        let child_id = repo.create(&child).await?;

        // List children
        let children = repo.list_children(parent_id).await?;

        assert_eq!(children.len(), 1);
        assert_eq!(children[0].id, Some(child_id));
        assert_eq!(children[0].parent_page_id, Some(parent_id));
        assert_eq!(children[0].slug, "child");

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_children_multiple_children_ordered() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());

        // Create parent as child of root
        let parent = Page::new_with_parent(
            site_id,
            root_id,
            "parent".to_string(),
            "Parent Page".to_string(),
        );
        let parent_id = repo.create(&parent).await?;

        // Create children with different positions
        let mut child1 = Page::new_with_parent(
            site_id,
            parent_id,
            "beta".to_string(),
            "Beta Page".to_string(),
        );
        child1.position = 2;

        let mut child2 = Page::new_with_parent(
            site_id,
            parent_id,
            "alpha".to_string(),
            "Alpha Page".to_string(),
        );
        child2.position = 1;

        let mut child3 = Page::new_with_parent(
            site_id,
            parent_id,
            "gamma".to_string(),
            "Gamma Page".to_string(),
        );
        child3.position = 3;

        repo.create(&child1).await?;
        repo.create(&child2).await?;
        repo.create(&child3).await?;

        // List children
        let children = repo.list_children(parent_id).await?;

        // Verify they are ordered by position
        assert_eq!(children.len(), 3);
        assert_eq!(children[0].slug, "alpha"); // position 1
        assert_eq!(children[1].slug, "beta"); // position 2
        assert_eq!(children[2].slug, "gamma"); // position 3

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_children_same_position_ordered_by_slug() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());

        // Create parent as child of root
        let parent = Page::new_with_parent(
            site_id,
            root_id,
            "parent".to_string(),
            "Parent Page".to_string(),
        );
        let parent_id = repo.create(&parent).await?;

        // Create children with same position (0)
        let child1 = Page::new_with_parent(
            site_id,
            parent_id,
            "zebra".to_string(),
            "Zebra Page".to_string(),
        );
        let child2 = Page::new_with_parent(
            site_id,
            parent_id,
            "apple".to_string(),
            "Apple Page".to_string(),
        );
        let child3 = Page::new_with_parent(
            site_id,
            parent_id,
            "mango".to_string(),
            "Mango Page".to_string(),
        );

        repo.create(&child1).await?;
        repo.create(&child2).await?;
        repo.create(&child3).await?;

        // List children
        let children = repo.list_children(parent_id).await?;

        // Verify they are ordered by slug when position is same
        assert_eq!(children.len(), 3);
        assert_eq!(children[0].slug, "apple");
        assert_eq!(children[1].slug, "mango");
        assert_eq!(children[2].slug, "zebra");

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_children_only_direct_children() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());

        // Create hierarchy: root -> parent -> child -> grandchild
        let parent = Page::new_with_parent(
            site_id,
            root_id,
            "parent".to_string(),
            "Parent Page".to_string(),
        );
        let parent_id = repo.create(&parent).await?;

        let child = Page::new_with_parent(
            site_id,
            parent_id,
            "child".to_string(),
            "Child Page".to_string(),
        );
        let child_id = repo.create(&child).await?;

        let grandchild = Page::new_with_parent(
            site_id,
            child_id,
            "grandchild".to_string(),
            "Grandchild Page".to_string(),
        );
        repo.create(&grandchild).await?;

        // List children of parent
        let children = repo.list_children(parent_id).await?;

        // Should only get direct child, not grandchild
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].slug, "child");

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_children_non_existent_parent() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = PageRepository::new(pool);

        // List children for non-existent parent
        let children = repo.list_children(999).await?;

        assert!(children.is_empty());

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_children_different_sites() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id1 = setup_test_db(&pool).await?;

        // Create a second site using SiteRepository to ensure root page is created
        let site_repo = SiteRepository::new(pool.clone());
        let site2 = Site::new("test-site-2.com".to_string(), "Test Site 2".to_string());
        let site_id2 = site_repo.create(&site2).await?;

        // Get root pages for both sites
        let root_id1 = get_root_page_id(&pool, site_id1).await?;
        let root_id2 = get_root_page_id(&pool, site_id2).await?;

        let repo = PageRepository::new(pool.clone());

        // Create parent pages on different sites
        let parent1 = Page::new_with_parent(
            site_id1,
            root_id1,
            "parent".to_string(),
            "Parent 1".to_string(),
        );
        let parent_id1 = repo.create(&parent1).await?;

        let parent2 = Page::new_with_parent(
            site_id2,
            root_id2,
            "parent".to_string(),
            "Parent 2".to_string(),
        );
        let parent_id2 = repo.create(&parent2).await?;

        // Create children for each parent
        let child1 = Page::new_with_parent(
            site_id1,
            parent_id1,
            "child1".to_string(),
            "Child of Parent 1".to_string(),
        );
        let child2 = Page::new_with_parent(
            site_id2,
            parent_id2,
            "child2".to_string(),
            "Child of Parent 2".to_string(),
        );

        repo.create(&child1).await?;
        repo.create(&child2).await?;

        // List children for parent1
        let children1 = repo.list_children(parent_id1).await?;
        assert_eq!(children1.len(), 1);
        assert_eq!(children1[0].slug, "child1");
        assert_eq!(children1[0].site_id, site_id1);

        // List children for parent2
        let children2 = repo.list_children(parent_id2).await?;
        assert_eq!(children2.len(), 1);
        assert_eq!(children2[0].slug, "child2");
        assert_eq!(children2[0].site_id, site_id2);

        Ok(())
    }

    #[sqlx::test]
    async fn test_breadcrumb_trail_root_page() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());

        // Get breadcrumb trail for root page
        let trail = repo.get_breadcrumb_trail(root_id).await?;

        // Trail should contain only the page itself
        assert_eq!(trail.len(), 1);
        assert_eq!(trail[0].id, Some(root_id));
        assert_eq!(trail[0].slug, ""); // Root page has empty slug

        Ok(())
    }

    #[sqlx::test]
    async fn test_breadcrumb_trail_one_level_deep() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());

        // Create child of root
        let child = Page::new_with_parent(
            site_id,
            root_id,
            "child".to_string(),
            "Child Page".to_string(),
        );
        let child_id = repo.create(&child).await?;

        // Get breadcrumb trail for child
        let trail = repo.get_breadcrumb_trail(child_id).await?;

        // Trail should be [root, child]
        assert_eq!(trail.len(), 2);
        assert_eq!(trail[0].id, Some(root_id));
        assert_eq!(trail[0].slug, ""); // Root page has empty slug
        assert_eq!(trail[1].id, Some(child_id));
        assert_eq!(trail[1].slug, "child");

        Ok(())
    }

    #[sqlx::test]
    async fn test_breadcrumb_trail_multiple_levels() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());

        // Create hierarchy: root -> products -> electronics -> phones
        let products = Page::new_with_parent(
            site_id,
            root_id,
            "products".to_string(),
            "Products".to_string(),
        );
        let products_id = repo.create(&products).await?;

        let electronics = Page::new_with_parent(
            site_id,
            products_id,
            "electronics".to_string(),
            "Electronics".to_string(),
        );
        let electronics_id = repo.create(&electronics).await?;

        let phones = Page::new_with_parent(
            site_id,
            electronics_id,
            "phones".to_string(),
            "Phones".to_string(),
        );
        let phones_id = repo.create(&phones).await?;

        // Get breadcrumb trail for phones
        let trail = repo.get_breadcrumb_trail(phones_id).await?;

        // Trail should be [root, products, electronics, phones]
        assert_eq!(trail.len(), 4);
        assert_eq!(trail[0].slug, ""); // Root page has empty slug
        assert_eq!(trail[1].slug, "products");
        assert_eq!(trail[2].slug, "electronics");
        assert_eq!(trail[3].slug, "phones");

        Ok(())
    }

    #[sqlx::test]
    async fn test_breadcrumb_trail_non_existent_page() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = PageRepository::new(pool);

        // Get breadcrumb trail for non-existent page
        let trail = repo.get_breadcrumb_trail(999).await?;

        // Trail should be empty
        assert!(trail.is_empty());

        Ok(())
    }

    #[sqlx::test]
    async fn test_breadcrumb_trail_middle_of_hierarchy() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());

        // Create hierarchy
        let middle =
            Page::new_with_parent(site_id, root_id, "middle".to_string(), "Middle".to_string());
        let middle_id = repo.create(&middle).await?;

        let leaf =
            Page::new_with_parent(site_id, middle_id, "leaf".to_string(), "Leaf".to_string());
        repo.create(&leaf).await?;

        // Get breadcrumb trail for middle page
        let trail = repo.get_breadcrumb_trail(middle_id).await?;

        // Trail should be [root, middle]
        assert_eq!(trail.len(), 2);
        assert_eq!(trail[0].slug, ""); // Root page has empty slug
        assert_eq!(trail[1].slug, "middle");

        Ok(())
    }

    #[sqlx::test]
    async fn test_breadcrumb_trail_with_siblings() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());

        // Create hierarchy with siblings
        let parent =
            Page::new_with_parent(site_id, root_id, "parent".to_string(), "Parent".to_string());
        let parent_id = repo.create(&parent).await?;

        let child1 = Page::new_with_parent(
            site_id,
            parent_id,
            "child1".to_string(),
            "Child 1".to_string(),
        );
        let child1_id = repo.create(&child1).await?;

        let child2 = Page::new_with_parent(
            site_id,
            parent_id,
            "child2".to_string(),
            "Child 2".to_string(),
        );
        let child2_id = repo.create(&child2).await?;

        // Get breadcrumb trail for child1
        let trail1 = repo.get_breadcrumb_trail(child1_id).await?;
        assert_eq!(trail1.len(), 3); // root -> parent -> child1
        assert_eq!(trail1[0].slug, ""); // root
        assert_eq!(trail1[1].slug, "parent");
        assert_eq!(trail1[2].slug, "child1");

        // Get breadcrumb trail for child2
        let trail2 = repo.get_breadcrumb_trail(child2_id).await?;
        assert_eq!(trail2.len(), 3); // root -> parent -> child2
        assert_eq!(trail2[0].slug, ""); // root
        assert_eq!(trail2[1].slug, "parent");
        assert_eq!(trail2[2].slug, "child2");

        // Both should have same parent
        assert_eq!(trail1[1].id, trail2[1].id);

        Ok(())
    }

    #[sqlx::test]
    async fn test_only_one_root_page_per_site() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        let repo = PageRepository::new(pool);

        // Site already has a root page created automatically
        let existing_root = repo.get_root_page(site_id).await?;
        assert!(existing_root.is_some());

        // Try to create another root page manually - should fail
        let root2 = Page::new(
            site_id,
            "another-root".to_string(),
            "Another Root".to_string(),
        );
        let result = repo.create(&root2).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Root pages are created automatically"));

        Ok(())
    }

    #[sqlx::test]
    async fn test_cannot_delete_root_page() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());

        // Try to delete root page - should fail
        let result = repo.delete(root_id).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot delete root page"));

        // Verify root page still exists
        let found = repo.find_by_id(root_id).await?;
        assert!(found.is_some());

        Ok(())
    }

    #[sqlx::test]
    async fn test_cannot_delete_page_with_children() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());

        // Create parent page
        let parent = Page::new_with_parent(
            site_id,
            root_id,
            "parent".to_string(),
            "Parent Page".to_string(),
        );
        let parent_id = repo.create(&parent).await?;

        // Create child page
        let child = Page::new_with_parent(
            site_id,
            parent_id,
            "child".to_string(),
            "Child Page".to_string(),
        );
        repo.create(&child).await?;

        // Try to delete parent page - should fail
        let result = repo.delete(parent_id).await;
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Cannot delete page with id"));
        assert!(error_msg.contains("because it has 1 child page(s)"));

        // Verify parent page still exists
        let found = repo.find_by_id(parent_id).await?;
        assert!(found.is_some());

        Ok(())
    }

    #[sqlx::test]
    async fn test_slug_uniqueness_per_parent() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());

        // Create two parent pages
        let parent1 = Page::new_with_parent(
            site_id,
            root_id,
            "products".to_string(),
            "Products".to_string(),
        );
        let parent1_id = repo.create(&parent1).await?;

        let parent2 = Page::new_with_parent(
            site_id,
            root_id,
            "services".to_string(),
            "Services".to_string(),
        );
        let parent2_id = repo.create(&parent2).await?;

        // Create child with same slug under different parents - should succeed
        let child1 = Page::new_with_parent(
            site_id,
            parent1_id,
            "overview".to_string(),
            "Product Overview".to_string(),
        );
        let child1_id = repo.create(&child1).await?;

        let child2 = Page::new_with_parent(
            site_id,
            parent2_id,
            "overview".to_string(),
            "Service Overview".to_string(),
        );
        let child2_id = repo.create(&child2).await?;

        // Both should exist
        assert!(repo.find_by_id(child1_id).await?.is_some());
        assert!(repo.find_by_id(child2_id).await?.is_some());

        // Try to create another child with same slug under parent1 - should fail
        let duplicate = Page::new_with_parent(
            site_id,
            parent1_id,
            "overview".to_string(),
            "Duplicate Overview".to_string(),
        );
        let result = repo.create(&duplicate).await;
        assert!(result.is_err());

        Ok(())
    }

    #[sqlx::test]
    async fn test_multiple_sites_each_with_root_page() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id1 = setup_test_db(&pool).await?;

        // Create second site using SiteRepository to ensure root page is created
        let site_repo = SiteRepository::new(pool.clone());
        let site2 = Site::new("test-site-2.com".to_string(), "Test Site 2".to_string());
        let site_id2 = site_repo.create(&site2).await?;

        // Get root pages for both sites
        let root1_id = get_root_page_id(&pool, site_id1).await?;
        let root2_id = get_root_page_id(&pool, site_id2).await?;

        let repo = PageRepository::new(pool.clone());

        // Both root pages should exist
        assert!(repo.find_by_id(root1_id).await?.is_some());
        assert!(repo.find_by_id(root2_id).await?.is_some());

        // Root pages should be different
        assert_ne!(root1_id, root2_id);

        Ok(())
    }

    #[sqlx::test]
    async fn test_get_root_page() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        let repo = PageRepository::new(pool.clone());

        // Should find root page that was created automatically
        let found_root = repo.get_root_page(site_id).await?;
        assert!(found_root.is_some());
        let found_root = found_root.unwrap();
        assert!(found_root.id.is_some());
        assert_eq!(found_root.slug, ""); // Root page has empty slug
        assert!(found_root.parent_page_id.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_unique_constraint_with_null_parent() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;

        // Get the root page that was created automatically
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());

        // Try to create another root page - should fail due to our check
        let root2 = Page::new(site_id, "".to_string(), "Home Page 2".to_string());
        let result = repo.create(&root2).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Root pages are created automatically"));

        // But pages with same slug under different parents should work
        let child1 =
            Page::new_with_parent(site_id, root_id, "about".to_string(), "About 1".to_string());
        let child1_id = repo.create(&child1).await?;

        let parent2 = Page::new_with_parent(
            site_id,
            root_id,
            "products".to_string(),
            "Products".to_string(),
        );
        let parent2_id = repo.create(&parent2).await?;

        let child2 = Page::new_with_parent(
            site_id,
            parent2_id,
            "about".to_string(),
            "About 2".to_string(),
        );
        let child2_id = repo.create(&child2).await?;

        // Both children should exist with same slug but different parents
        assert!(repo.find_by_id(child1_id).await?.is_some());
        assert!(repo.find_by_id(child2_id).await?.is_some());

        Ok(())
    }

    #[sqlx::test]
    async fn test_is_descendant_of_direct_child() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let repo = PageRepository::new(pool.clone());

        // Get root page
        let root_id = get_root_page_id(&pool, site_id).await?;

        // Create a child page
        let child = Page::new_with_parent(
            site_id,
            root_id,
            "child".to_string(),
            "Child Page".to_string(),
        );
        let child_id = repo.create(&child).await?;

        // Child should be a descendant of root
        assert!(repo.is_descendant_of(child_id, root_id).await?);

        // Root should NOT be a descendant of child
        assert!(!repo.is_descendant_of(root_id, child_id).await?);

        Ok(())
    }

    #[sqlx::test]
    async fn test_is_descendant_of_deep_hierarchy() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let repo = PageRepository::new(pool.clone());

        // Get root page
        let root_id = get_root_page_id(&pool, site_id).await?;

        // Create a deep hierarchy: root -> level1 -> level2 -> level3
        let level1 = Page::new_with_parent(
            site_id,
            root_id,
            "level1".to_string(),
            "Level 1".to_string(),
        );
        let level1_id = repo.create(&level1).await?;

        let level2 = Page::new_with_parent(
            site_id,
            level1_id,
            "level2".to_string(),
            "Level 2".to_string(),
        );
        let level2_id = repo.create(&level2).await?;

        let level3 = Page::new_with_parent(
            site_id,
            level2_id,
            "level3".to_string(),
            "Level 3".to_string(),
        );
        let level3_id = repo.create(&level3).await?;

        // Test various relationships
        assert!(repo.is_descendant_of(level3_id, level2_id).await?); // Direct parent
        assert!(repo.is_descendant_of(level3_id, level1_id).await?); // Grandparent
        assert!(repo.is_descendant_of(level3_id, root_id).await?); // Great-grandparent
        assert!(repo.is_descendant_of(level2_id, root_id).await?); // Level2 is descendant of root
        assert!(repo.is_descendant_of(level1_id, root_id).await?); // Level1 is descendant of root

        // Test non-relationships
        assert!(!repo.is_descendant_of(root_id, level1_id).await?); // Root not descendant of level1
        assert!(!repo.is_descendant_of(level1_id, level2_id).await?); // Parent not descendant of child
        assert!(!repo.is_descendant_of(level2_id, level3_id).await?); // Parent not descendant of child

        Ok(())
    }

    #[sqlx::test]
    async fn test_is_descendant_of_same_page() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let repo = PageRepository::new(pool.clone());

        // Get root page
        let root_id = get_root_page_id(&pool, site_id).await?;

        // Create a page
        let page = Page::new_with_parent(site_id, root_id, "page".to_string(), "Page".to_string());
        let page_id = repo.create(&page).await?;

        // A page is NOT a descendant of itself
        assert!(!repo.is_descendant_of(page_id, page_id).await?);
        assert!(!repo.is_descendant_of(root_id, root_id).await?);

        Ok(())
    }

    #[sqlx::test]
    async fn test_is_descendant_of_non_existent_pages() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let repo = PageRepository::new(pool.clone());

        // Get root page
        let root_id = get_root_page_id(&pool, site_id).await?;

        // Non-existent page is not a descendant of anything
        assert!(!repo.is_descendant_of(9999, root_id).await?);

        // Existing page is not a descendant of non-existent page
        assert!(!repo.is_descendant_of(root_id, 9999).await?);

        // Two non-existent pages
        assert!(!repo.is_descendant_of(9999, 8888).await?);

        Ok(())
    }

    #[sqlx::test]
    async fn test_is_descendant_of_root_page() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let repo = PageRepository::new(pool.clone());

        // Get root page
        let root_id = get_root_page_id(&pool, site_id).await?;

        // Create a child
        let child =
            Page::new_with_parent(site_id, root_id, "child".to_string(), "Child".to_string());
        let child_id = repo.create(&child).await?;

        // Root page is not a descendant of anything (it has no parent)
        assert!(!repo.is_descendant_of(root_id, child_id).await?);
        assert!(!repo.is_descendant_of(root_id, root_id).await?);
        assert!(!repo.is_descendant_of(root_id, 9999).await?);

        Ok(())
    }

    #[sqlx::test]
    async fn test_is_descendant_of_sibling_pages() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let repo = PageRepository::new(pool.clone());

        // Get root page
        let root_id = get_root_page_id(&pool, site_id).await?;

        // Create two siblings under root
        let sibling1 = Page::new_with_parent(
            site_id,
            root_id,
            "sibling1".to_string(),
            "Sibling 1".to_string(),
        );
        let sibling1_id = repo.create(&sibling1).await?;

        let sibling2 = Page::new_with_parent(
            site_id,
            root_id,
            "sibling2".to_string(),
            "Sibling 2".to_string(),
        );
        let sibling2_id = repo.create(&sibling2).await?;

        // Siblings are not descendants of each other
        assert!(!repo.is_descendant_of(sibling1_id, sibling2_id).await?);
        assert!(!repo.is_descendant_of(sibling2_id, sibling1_id).await?);

        // But both are descendants of root
        assert!(repo.is_descendant_of(sibling1_id, root_id).await?);
        assert!(repo.is_descendant_of(sibling2_id, root_id).await?);

        Ok(())
    }

    #[sqlx::test]
    async fn test_is_descendant_of_cross_site() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id1 = setup_test_db(&pool).await?;

        // Create second site
        let site_repo = SiteRepository::new(pool.clone());
        let site2 = Site::new("test-site-2.com".to_string(), "Test Site 2".to_string());
        let site_id2 = site_repo.create(&site2).await?;

        let repo = PageRepository::new(pool.clone());

        // Get root pages
        let root1_id = get_root_page_id(&pool, site_id1).await?;
        let root2_id = get_root_page_id(&pool, site_id2).await?;

        // Create child in site1
        let child1 = Page::new_with_parent(
            site_id1,
            root1_id,
            "child1".to_string(),
            "Child 1".to_string(),
        );
        let child1_id = repo.create(&child1).await?;

        // Create child in site2
        let child2 = Page::new_with_parent(
            site_id2,
            root2_id,
            "child2".to_string(),
            "Child 2".to_string(),
        );
        let child2_id = repo.create(&child2).await?;

        // Pages from different sites are never descendants of each other
        assert!(!repo.is_descendant_of(child1_id, root2_id).await?);
        assert!(!repo.is_descendant_of(child2_id, root1_id).await?);
        assert!(!repo.is_descendant_of(child1_id, child2_id).await?);
        assert!(!repo.is_descendant_of(root1_id, root2_id).await?);

        Ok(())
    }

    #[sqlx::test]
    async fn test_get_all_descendants_no_children() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let repo = PageRepository::new(pool.clone());

        // Get root page
        let root_id = get_root_page_id(&pool, site_id).await?;

        // Create a page with no children
        let page = Page::new_with_parent(site_id, root_id, "page".to_string(), "Page".to_string());
        let page_id = repo.create(&page).await?;

        // Get descendants - should be empty
        let descendants = repo.get_all_descendants(page_id).await?;
        assert!(descendants.is_empty());

        Ok(())
    }

    #[sqlx::test]
    async fn test_get_all_descendants_direct_children() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let repo = PageRepository::new(pool.clone());

        // Get root page
        let root_id = get_root_page_id(&pool, site_id).await?;

        // Create children under root
        let child1 = Page::new_with_parent(
            site_id,
            root_id,
            "child1".to_string(),
            "Child 1".to_string(),
        );
        let child1_id = repo.create(&child1).await?;

        let child2 = Page::new_with_parent(
            site_id,
            root_id,
            "child2".to_string(),
            "Child 2".to_string(),
        );
        let child2_id = repo.create(&child2).await?;

        // Get descendants of root
        let descendants = repo.get_all_descendants(root_id).await?;
        assert_eq!(descendants.len(), 2);

        // Verify both children are included
        let descendant_ids: Vec<i64> = descendants.iter().filter_map(|p| p.id).collect();
        assert!(descendant_ids.contains(&child1_id));
        assert!(descendant_ids.contains(&child2_id));

        Ok(())
    }

    #[sqlx::test]
    async fn test_get_all_descendants_deep_hierarchy() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let repo = PageRepository::new(pool.clone());

        // Get root page
        let root_id = get_root_page_id(&pool, site_id).await?;

        // Create a deep hierarchy
        // root
        //   ├── level1
        //   │   ├── level2a
        //   │   │   └── level3
        //   │   └── level2b
        //   └── sibling

        let level1 = Page::new_with_parent(
            site_id,
            root_id,
            "level1".to_string(),
            "Level 1".to_string(),
        );
        let level1_id = repo.create(&level1).await?;

        let sibling = Page::new_with_parent(
            site_id,
            root_id,
            "sibling".to_string(),
            "Sibling".to_string(),
        );
        let sibling_id = repo.create(&sibling).await?;

        let level2a = Page::new_with_parent(
            site_id,
            level1_id,
            "level2a".to_string(),
            "Level 2A".to_string(),
        );
        let level2a_id = repo.create(&level2a).await?;

        let level2b = Page::new_with_parent(
            site_id,
            level1_id,
            "level2b".to_string(),
            "Level 2B".to_string(),
        );
        let level2b_id = repo.create(&level2b).await?;

        let level3 = Page::new_with_parent(
            site_id,
            level2a_id,
            "level3".to_string(),
            "Level 3".to_string(),
        );
        let level3_id = repo.create(&level3).await?;

        // Get all descendants of root
        let root_descendants = repo.get_all_descendants(root_id).await?;
        assert_eq!(root_descendants.len(), 5);

        let root_descendant_ids: Vec<i64> = root_descendants.iter().filter_map(|p| p.id).collect();
        assert!(root_descendant_ids.contains(&level1_id));
        assert!(root_descendant_ids.contains(&sibling_id));
        assert!(root_descendant_ids.contains(&level2a_id));
        assert!(root_descendant_ids.contains(&level2b_id));
        assert!(root_descendant_ids.contains(&level3_id));

        // Get descendants of level1 (should include level2a, level2b, and level3)
        let level1_descendants = repo.get_all_descendants(level1_id).await?;
        assert_eq!(level1_descendants.len(), 3);

        let level1_descendant_ids: Vec<i64> =
            level1_descendants.iter().filter_map(|p| p.id).collect();
        assert!(level1_descendant_ids.contains(&level2a_id));
        assert!(level1_descendant_ids.contains(&level2b_id));
        assert!(level1_descendant_ids.contains(&level3_id));
        assert!(!level1_descendant_ids.contains(&sibling_id)); // Sibling is not a descendant

        // Get descendants of level2a (should only include level3)
        let level2a_descendants = repo.get_all_descendants(level2a_id).await?;
        assert_eq!(level2a_descendants.len(), 1);
        assert_eq!(level2a_descendants[0].id, Some(level3_id));

        Ok(())
    }

    #[sqlx::test]
    async fn test_get_all_descendants_non_existent_page() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let _site_id = setup_test_db(&pool).await?;
        let repo = PageRepository::new(pool.clone());

        // Get descendants of non-existent page
        let descendants = repo.get_all_descendants(9999).await?;
        assert!(descendants.is_empty());

        Ok(())
    }

    #[sqlx::test]
    async fn test_get_all_descendants_ordering() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let repo = PageRepository::new(pool.clone());

        // Get root page
        let root_id = get_root_page_id(&pool, site_id).await?;

        // Create pages with specific positions
        let mut page1 =
            Page::new_with_parent(site_id, root_id, "page1".to_string(), "Page 1".to_string());
        page1.position = 2;
        let page1_id = repo.create(&page1).await?;

        let mut page2 =
            Page::new_with_parent(site_id, root_id, "page2".to_string(), "Page 2".to_string());
        page2.position = 1;
        let page2_id = repo.create(&page2).await?;

        let mut page3 =
            Page::new_with_parent(site_id, root_id, "page3".to_string(), "Page 3".to_string());
        page3.position = 3;
        repo.create(&page3).await?;

        // Create children under page1
        let mut child1 = Page::new_with_parent(
            site_id,
            page1_id,
            "child1".to_string(),
            "Child 1".to_string(),
        );
        child1.position = 2;
        repo.create(&child1).await?;

        let mut child2 = Page::new_with_parent(
            site_id,
            page1_id,
            "child2".to_string(),
            "Child 2".to_string(),
        );
        child2.position = 1;
        repo.create(&child2).await?;

        // Get all descendants
        let descendants = repo.get_all_descendants(root_id).await?;

        // Verify we have all pages
        assert_eq!(descendants.len(), 5);

        // Verify pages are grouped by parent and sorted by position within each group
        let page2_idx = descendants
            .iter()
            .position(|p| p.id == Some(page2_id))
            .unwrap();
        let page1_idx = descendants
            .iter()
            .position(|p| p.id == Some(page1_id))
            .unwrap();

        // Page2 (position 1) should come before Page1 (position 2)
        assert!(page2_idx < page1_idx);

        Ok(())
    }

    #[sqlx::test]
    async fn test_get_all_descendants_cross_site() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id1 = setup_test_db(&pool).await?;

        // Create second site
        let site_repo = SiteRepository::new(pool.clone());
        let site2 = Site::new("test-site-2.com".to_string(), "Test Site 2".to_string());
        let site_id2 = site_repo.create(&site2).await?;

        let repo = PageRepository::new(pool.clone());

        // Get root pages
        let root1_id = get_root_page_id(&pool, site_id1).await?;
        let root2_id = get_root_page_id(&pool, site_id2).await?;

        // Create children in both sites
        let child1 = Page::new_with_parent(
            site_id1,
            root1_id,
            "child1".to_string(),
            "Child 1".to_string(),
        );
        repo.create(&child1).await?;

        let child2 = Page::new_with_parent(
            site_id2,
            root2_id,
            "child2".to_string(),
            "Child 2".to_string(),
        );
        let child2_id = repo.create(&child2).await?;

        // Get descendants of root1 - should not include pages from site2
        let descendants = repo.get_all_descendants(root1_id).await?;
        assert_eq!(descendants.len(), 1);

        let descendant_ids: Vec<i64> = descendants.iter().filter_map(|p| p.id).collect();
        assert!(!descendant_ids.contains(&child2_id));

        Ok(())
    }

    #[sqlx::test]
    async fn test_reorder_siblings_basic() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let repo = PageRepository::new(pool.clone());

        // Get root page
        let root_id = get_root_page_id(&pool, site_id).await?;

        // Create siblings with initial positions
        let mut page1 =
            Page::new_with_parent(site_id, root_id, "page1".to_string(), "Page 1".to_string());
        page1.position = 1;
        let page1_id = repo.create(&page1).await?;

        let mut page2 =
            Page::new_with_parent(site_id, root_id, "page2".to_string(), "Page 2".to_string());
        page2.position = 2;
        let page2_id = repo.create(&page2).await?;

        let mut page3 =
            Page::new_with_parent(site_id, root_id, "page3".to_string(), "Page 3".to_string());
        page3.position = 3;
        let page3_id = repo.create(&page3).await?;

        // Reorder: page3 -> position 1, page1 -> position 2, page2 -> position 3
        repo.reorder_siblings(
            Some(root_id),
            vec![(page3_id, 1), (page1_id, 2), (page2_id, 3)],
        )
        .await?;

        // Verify new positions
        let children = repo.list_children(root_id).await?;
        assert_eq!(children.len(), 3);
        assert_eq!(children[0].id, Some(page3_id));
        assert_eq!(children[0].position, 1);
        assert_eq!(children[1].id, Some(page1_id));
        assert_eq!(children[1].position, 2);
        assert_eq!(children[2].id, Some(page2_id));
        assert_eq!(children[2].position, 3);

        Ok(())
    }

    #[sqlx::test]
    async fn test_reorder_siblings_root_level() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let repo = PageRepository::new(pool.clone());

        // Get root page
        let root_id = get_root_page_id(&pool, site_id).await?;

        // Create another page at root level (which shouldn't be possible, but for testing)
        // We'll test with pages under root instead
        let page1 =
            Page::new_with_parent(site_id, root_id, "page1".to_string(), "Page 1".to_string());
        let page1_id = repo.create(&page1).await?;

        let page2 =
            Page::new_with_parent(site_id, root_id, "page2".to_string(), "Page 2".to_string());
        let page2_id = repo.create(&page2).await?;

        // Reorder
        repo.reorder_siblings(Some(root_id), vec![(page2_id, 1), (page1_id, 2)])
            .await?;

        // Verify
        let children = repo.list_children(root_id).await?;
        assert_eq!(children[0].id, Some(page2_id));
        assert_eq!(children[1].id, Some(page1_id));

        Ok(())
    }

    #[sqlx::test]
    async fn test_reorder_siblings_wrong_parent() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let repo = PageRepository::new(pool.clone());

        // Get root page
        let root_id = get_root_page_id(&pool, site_id).await?;

        // Create two parent pages
        let parent1 = Page::new_with_parent(
            site_id,
            root_id,
            "parent1".to_string(),
            "Parent 1".to_string(),
        );
        let parent1_id = repo.create(&parent1).await?;

        let parent2 = Page::new_with_parent(
            site_id,
            root_id,
            "parent2".to_string(),
            "Parent 2".to_string(),
        );
        let parent2_id = repo.create(&parent2).await?;

        // Create child under parent1
        let child = Page::new_with_parent(
            site_id,
            parent1_id,
            "child".to_string(),
            "Child".to_string(),
        );
        let child_id = repo.create(&child).await?;

        // Try to reorder as if child belongs to parent2 - should fail
        let result = repo
            .reorder_siblings(Some(parent2_id), vec![(child_id, 1)])
            .await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("does not have parent_page_id"));

        Ok(())
    }

    #[sqlx::test]
    async fn test_reorder_siblings_non_existent_page() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let repo = PageRepository::new(pool.clone());

        // Get root page
        let root_id = get_root_page_id(&pool, site_id).await?;

        // Try to reorder non-existent page
        let result = repo.reorder_siblings(Some(root_id), vec![(9999, 1)]).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Page with id 9999 not found"));

        Ok(())
    }

    #[sqlx::test]
    async fn test_reorder_siblings_empty_list() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let repo = PageRepository::new(pool.clone());

        // Get root page
        let root_id = get_root_page_id(&pool, site_id).await?;

        // Reorder with empty list - should succeed (no-op)
        repo.reorder_siblings(Some(root_id), vec![]).await?;

        Ok(())
    }

    #[sqlx::test]
    async fn test_reorder_siblings_partial_list() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let repo = PageRepository::new(pool.clone());

        // Get root page
        let root_id = get_root_page_id(&pool, site_id).await?;

        // Create three siblings
        let page1 =
            Page::new_with_parent(site_id, root_id, "page1".to_string(), "Page 1".to_string());
        let page1_id = repo.create(&page1).await?;

        let page2 =
            Page::new_with_parent(site_id, root_id, "page2".to_string(), "Page 2".to_string());
        let page2_id = repo.create(&page2).await?;

        let page3 =
            Page::new_with_parent(site_id, root_id, "page3".to_string(), "Page 3".to_string());
        let page3_id = repo.create(&page3).await?;

        // Reorder only two of them
        repo.reorder_siblings(Some(root_id), vec![(page2_id, 10), (page3_id, 20)])
            .await?;

        // Verify: page1 keeps position 0, page2 gets 10, page3 gets 20
        let page1_updated = repo.find_by_id(page1_id).await?.unwrap();
        let page2_updated = repo.find_by_id(page2_id).await?.unwrap();
        let page3_updated = repo.find_by_id(page3_id).await?.unwrap();

        assert_eq!(page1_updated.position, 0); // Unchanged
        assert_eq!(page2_updated.position, 10);
        assert_eq!(page3_updated.position, 20);

        Ok(())
    }

    #[sqlx::test]
    async fn test_reorder_siblings_duplicate_positions() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let repo = PageRepository::new(pool.clone());

        // Get root page
        let root_id = get_root_page_id(&pool, site_id).await?;

        // Create two siblings
        let page1 =
            Page::new_with_parent(site_id, root_id, "page1".to_string(), "Page 1".to_string());
        let page1_id = repo.create(&page1).await?;

        let page2 =
            Page::new_with_parent(site_id, root_id, "page2".to_string(), "Page 2".to_string());
        let page2_id = repo.create(&page2).await?;

        // Set both to the same position (this is allowed, ordering will use slug as tiebreaker)
        repo.reorder_siblings(Some(root_id), vec![(page1_id, 5), (page2_id, 5)])
            .await?;

        // Verify both have position 5
        let page1_updated = repo.find_by_id(page1_id).await?.unwrap();
        let page2_updated = repo.find_by_id(page2_id).await?.unwrap();

        assert_eq!(page1_updated.position, 5);
        assert_eq!(page2_updated.position, 5);

        // When listed, they should be ordered by slug as tiebreaker
        let children = repo.list_children(root_id).await?;
        assert_eq!(children[0].slug, "page1"); // Comes first alphabetically
        assert_eq!(children[1].slug, "page2");

        Ok(())
    }

    #[sqlx::test]
    async fn test_move_page_basic() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let repo = PageRepository::new(pool.clone());

        // Get root page
        let root_id = get_root_page_id(&pool, site_id).await?;

        // Create structure:
        // root
        //   ├── parent1
        //   │   └── child
        //   └── parent2

        let parent1 = Page::new_with_parent(
            site_id,
            root_id,
            "parent1".to_string(),
            "Parent 1".to_string(),
        );
        let parent1_id = repo.create(&parent1).await?;

        let parent2 = Page::new_with_parent(
            site_id,
            root_id,
            "parent2".to_string(),
            "Parent 2".to_string(),
        );
        let parent2_id = repo.create(&parent2).await?;

        let child = Page::new_with_parent(
            site_id,
            parent1_id,
            "child".to_string(),
            "Child".to_string(),
        );
        let child_id = repo.create(&child).await?;

        // Move child from parent1 to parent2
        repo.move_page(child_id, parent2_id).await?;

        // Verify the move
        let moved_child = repo.find_by_id(child_id).await?.unwrap();
        assert_eq!(moved_child.parent_page_id, Some(parent2_id));

        // Verify child is no longer under parent1
        let parent1_children = repo.list_children(parent1_id).await?;
        assert!(parent1_children.is_empty());

        // Verify child is now under parent2
        let parent2_children = repo.list_children(parent2_id).await?;
        assert_eq!(parent2_children.len(), 1);
        assert_eq!(parent2_children[0].id, Some(child_id));

        Ok(())
    }

    #[sqlx::test]
    async fn test_move_page_cannot_move_root() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let repo = PageRepository::new(pool.clone());

        // Get root page
        let root_id = get_root_page_id(&pool, site_id).await?;

        // Create a page
        let page = Page::new_with_parent(site_id, root_id, "page".to_string(), "Page".to_string());
        let page_id = repo.create(&page).await?;

        // Try to move root page - should fail
        let result = repo.move_page(root_id, page_id).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot move root page"));

        Ok(())
    }

    #[sqlx::test]
    async fn test_move_page_to_itself() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let repo = PageRepository::new(pool.clone());

        // Get root page
        let root_id = get_root_page_id(&pool, site_id).await?;

        // Create a page
        let page = Page::new_with_parent(site_id, root_id, "page".to_string(), "Page".to_string());
        let page_id = repo.create(&page).await?;

        // Try to move page to itself - should fail
        let result = repo.move_page(page_id, page_id).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot move page to itself"));

        Ok(())
    }

    #[sqlx::test]
    async fn test_move_page_to_descendant() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let repo = PageRepository::new(pool.clone());

        // Get root page
        let root_id = get_root_page_id(&pool, site_id).await?;

        // Create hierarchy: root -> parent -> child -> grandchild
        let parent =
            Page::new_with_parent(site_id, root_id, "parent".to_string(), "Parent".to_string());
        let parent_id = repo.create(&parent).await?;

        let child =
            Page::new_with_parent(site_id, parent_id, "child".to_string(), "Child".to_string());
        let child_id = repo.create(&child).await?;

        let grandchild = Page::new_with_parent(
            site_id,
            child_id,
            "grandchild".to_string(),
            "Grandchild".to_string(),
        );
        let grandchild_id = repo.create(&grandchild).await?;

        // Try to move parent to grandchild - should fail
        let result = repo.move_page(parent_id, grandchild_id).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot move page to one of its descendants"));

        // Try to move parent to child - should also fail
        let result2 = repo.move_page(parent_id, child_id).await;
        assert!(result2.is_err());
        assert!(result2
            .unwrap_err()
            .to_string()
            .contains("Cannot move page to one of its descendants"));

        Ok(())
    }

    #[sqlx::test]
    async fn test_move_page_same_parent_noop() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let repo = PageRepository::new(pool.clone());

        // Get root page
        let root_id = get_root_page_id(&pool, site_id).await?;

        // Create a page under root
        let page = Page::new_with_parent(site_id, root_id, "page".to_string(), "Page".to_string());
        let page_id = repo.create(&page).await?;

        // Move page to same parent (root) - should be no-op
        repo.move_page(page_id, root_id).await?;

        // Verify nothing changed
        let page_after = repo.find_by_id(page_id).await?.unwrap();
        assert_eq!(page_after.parent_page_id, Some(root_id));

        Ok(())
    }

    #[sqlx::test]
    async fn test_move_page_slug_conflict() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let repo = PageRepository::new(pool.clone());

        // Get root page
        let root_id = get_root_page_id(&pool, site_id).await?;

        // Create two parents
        let parent1 = Page::new_with_parent(
            site_id,
            root_id,
            "parent1".to_string(),
            "Parent 1".to_string(),
        );
        let parent1_id = repo.create(&parent1).await?;

        let parent2 = Page::new_with_parent(
            site_id,
            root_id,
            "parent2".to_string(),
            "Parent 2".to_string(),
        );
        let parent2_id = repo.create(&parent2).await?;

        // Create children with same slug under different parents
        let child1 = Page::new_with_parent(
            site_id,
            parent1_id,
            "sameslug".to_string(),
            "Child 1".to_string(),
        );
        let child1_id = repo.create(&child1).await?;

        let child2 = Page::new_with_parent(
            site_id,
            parent2_id,
            "sameslug".to_string(),
            "Child 2".to_string(),
        );
        let _child2_id = repo.create(&child2).await?;

        // Try to move child1 to parent2 - should fail due to slug conflict
        let result = repo.move_page(child1_id, parent2_id).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("already exists under the new parent"));

        Ok(())
    }

    #[sqlx::test]
    async fn test_move_page_cross_site() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id1 = setup_test_db(&pool).await?;

        // Create second site
        let site_repo = SiteRepository::new(pool.clone());
        let site2 = Site::new("test-site-2.com".to_string(), "Test Site 2".to_string());
        let site_id2 = site_repo.create(&site2).await?;

        let repo = PageRepository::new(pool.clone());

        // Get root pages
        let root1_id = get_root_page_id(&pool, site_id1).await?;
        let root2_id = get_root_page_id(&pool, site_id2).await?;

        // Create page in site1
        let page =
            Page::new_with_parent(site_id1, root1_id, "page".to_string(), "Page".to_string());
        let page_id = repo.create(&page).await?;

        // Try to move to site2 - should fail
        let result = repo.move_page(page_id, root2_id).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot move page to a different site"));

        Ok(())
    }

    #[sqlx::test]
    async fn test_move_page_position_updated() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let repo = PageRepository::new(pool.clone());

        // Get root page
        let root_id = get_root_page_id(&pool, site_id).await?;

        // Create parent with existing children
        let parent =
            Page::new_with_parent(site_id, root_id, "parent".to_string(), "Parent".to_string());
        let parent_id = repo.create(&parent).await?;

        let mut existing1 = Page::new_with_parent(
            site_id,
            parent_id,
            "existing1".to_string(),
            "Existing 1".to_string(),
        );
        existing1.position = 10;
        repo.create(&existing1).await?;

        let mut existing2 = Page::new_with_parent(
            site_id,
            parent_id,
            "existing2".to_string(),
            "Existing 2".to_string(),
        );
        existing2.position = 20;
        repo.create(&existing2).await?;

        // Create page to move
        let page = Page::new_with_parent(
            site_id,
            root_id,
            "moveme".to_string(),
            "Move Me".to_string(),
        );
        let page_id = repo.create(&page).await?;

        // Move page to parent
        repo.move_page(page_id, parent_id).await?;

        // Verify it got position 21 (max + 1)
        let moved_page = repo.find_by_id(page_id).await?.unwrap();
        assert_eq!(moved_page.position, 21);

        Ok(())
    }

    #[sqlx::test]
    async fn test_move_page_non_existent_page() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let repo = PageRepository::new(pool.clone());

        // Get root page
        let root_id = get_root_page_id(&pool, site_id).await?;

        // Try to move non-existent page
        let result = repo.move_page(9999, root_id).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Page with id 9999 not found"));

        Ok(())
    }

    #[sqlx::test]
    async fn test_move_page_non_existent_parent() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let repo = PageRepository::new(pool.clone());

        // Get root page
        let root_id = get_root_page_id(&pool, site_id).await?;

        // Create a page
        let page = Page::new_with_parent(site_id, root_id, "page".to_string(), "Page".to_string());
        let page_id = repo.create(&page).await?;

        // Try to move to non-existent parent
        let result = repo.move_page(page_id, 9999).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("New parent page with id 9999 not found"));

        Ok(())
    }

    #[sqlx::test]
    async fn test_move_page_with_children() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let repo = PageRepository::new(pool.clone());

        // Get root page
        let root_id = get_root_page_id(&pool, site_id).await?;

        // Create structure:
        // root
        //   ├── parent1
        //   │   └── branch (has children)
        //   │       ├── leaf1
        //   │       └── leaf2
        //   └── parent2

        let parent1 = Page::new_with_parent(
            site_id,
            root_id,
            "parent1".to_string(),
            "Parent 1".to_string(),
        );
        let parent1_id = repo.create(&parent1).await?;

        let parent2 = Page::new_with_parent(
            site_id,
            root_id,
            "parent2".to_string(),
            "Parent 2".to_string(),
        );
        let parent2_id = repo.create(&parent2).await?;

        let branch = Page::new_with_parent(
            site_id,
            parent1_id,
            "branch".to_string(),
            "Branch".to_string(),
        );
        let branch_id = repo.create(&branch).await?;

        let leaf1 = Page::new_with_parent(
            site_id,
            branch_id,
            "leaf1".to_string(),
            "Leaf 1".to_string(),
        );
        let leaf1_id = repo.create(&leaf1).await?;

        let leaf2 = Page::new_with_parent(
            site_id,
            branch_id,
            "leaf2".to_string(),
            "Leaf 2".to_string(),
        );
        let leaf2_id = repo.create(&leaf2).await?;

        // Move branch (with children) from parent1 to parent2
        repo.move_page(branch_id, parent2_id).await?;

        // Verify branch moved
        let moved_branch = repo.find_by_id(branch_id).await?.unwrap();
        assert_eq!(moved_branch.parent_page_id, Some(parent2_id));

        // Verify children still under branch (not affected by move)
        let branch_children = repo.list_children(branch_id).await?;
        assert_eq!(branch_children.len(), 2);

        let child_ids: Vec<i64> = branch_children.iter().filter_map(|p| p.id).collect();
        assert!(child_ids.contains(&leaf1_id));
        assert!(child_ids.contains(&leaf2_id));

        Ok(())
    }

    #[sqlx::test]
    async fn test_get_valid_move_targets_basic() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let repo = PageRepository::new(pool.clone());

        // Get root page
        let root_id = get_root_page_id(&pool, site_id).await?;

        // Create test pages
        let page1 =
            Page::new_with_parent(site_id, root_id, "page1".to_string(), "Page 1".to_string());
        let page1_id = repo.create(&page1).await?;

        let page2 =
            Page::new_with_parent(site_id, root_id, "page2".to_string(), "Page 2".to_string());
        let page2_id = repo.create(&page2).await?;

        let page3 =
            Page::new_with_parent(site_id, root_id, "page3".to_string(), "Page 3".to_string());
        let page3_id = repo.create(&page3).await?;

        // Get valid move targets for page1
        let targets = repo.get_valid_move_targets(page1_id).await?;

        // page1 can be moved to page2, or page3 (but not to itself or its current parent which is root)
        assert_eq!(targets.len(), 2);

        let target_ids: Vec<i64> = targets.iter().filter_map(|p| p.id).collect();
        assert!(!target_ids.contains(&root_id)); // Current parent excluded
        assert!(target_ids.contains(&page2_id));
        assert!(target_ids.contains(&page3_id));
        assert!(!target_ids.contains(&page1_id)); // Self excluded

        Ok(())
    }

    #[sqlx::test]
    async fn test_get_valid_move_targets_excludes_descendants() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let repo = PageRepository::new(pool.clone());

        // Get root page
        let root_id = get_root_page_id(&pool, site_id).await?;

        // Create hierarchy: parent -> child -> grandchild
        let parent =
            Page::new_with_parent(site_id, root_id, "parent".to_string(), "Parent".to_string());
        let parent_id = repo.create(&parent).await?;

        let child =
            Page::new_with_parent(site_id, parent_id, "child".to_string(), "Child".to_string());
        let child_id = repo.create(&child).await?;

        let grandchild = Page::new_with_parent(
            site_id,
            child_id,
            "grandchild".to_string(),
            "Grandchild".to_string(),
        );
        let grandchild_id = repo.create(&grandchild).await?;

        // Create a sibling to parent
        let sibling = Page::new_with_parent(
            site_id,
            root_id,
            "sibling".to_string(),
            "Sibling".to_string(),
        );
        let sibling_id = repo.create(&sibling).await?;

        // Get valid move targets for parent
        let targets = repo.get_valid_move_targets(parent_id).await?;

        // parent can only be moved to sibling (not to root which is its current parent, nor to child or grandchild)
        let target_ids: Vec<i64> = targets.iter().filter_map(|p| p.id).collect();
        assert!(!target_ids.contains(&root_id)); // Current parent excluded
        assert!(target_ids.contains(&sibling_id));
        assert!(!target_ids.contains(&parent_id)); // Self excluded
        assert!(!target_ids.contains(&child_id)); // Descendant excluded
        assert!(!target_ids.contains(&grandchild_id)); // Descendant excluded
        assert_eq!(targets.len(), 1); // Only sibling is valid

        Ok(())
    }

    #[sqlx::test]
    async fn test_get_valid_move_targets_for_root_page() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let repo = PageRepository::new(pool.clone());

        // Get root page
        let root_id = get_root_page_id(&pool, site_id).await?;

        // Root page should have no valid move targets
        let targets = repo.get_valid_move_targets(root_id).await?;
        assert_eq!(targets.len(), 0);

        Ok(())
    }

    #[sqlx::test]
    async fn test_get_valid_move_targets_different_sites() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;

        // Setup the database schema first
        let site1_id = setup_test_db(&pool).await?;

        let site_repo = SiteRepository::new(pool.clone());
        let page_repo = PageRepository::new(pool.clone());

        // Create second site
        let site2 = Site::new("site2.com".to_string(), "Site 2".to_string());
        let site2_id = site_repo.create(&site2).await?;

        // Get root pages
        let root1_id = get_root_page_id(&pool, site1_id).await?;
        let root2_id = get_root_page_id(&pool, site2_id).await?;

        // Create pages in different sites
        let page1 = Page::new_with_parent(
            site1_id,
            root1_id,
            "page1".to_string(),
            "Page 1".to_string(),
        );
        let page1_id = page_repo.create(&page1).await?;

        let page2 = Page::new_with_parent(
            site2_id,
            root2_id,
            "page2".to_string(),
            "Page 2".to_string(),
        );
        page_repo.create(&page2).await?;

        // Get valid move targets for page1 (should only include pages from site1)
        let targets = page_repo.get_valid_move_targets(page1_id).await?;

        // Should have no targets (root1 is current parent, no other pages in site1)
        assert_eq!(targets.len(), 0);

        Ok(())
    }

    #[sqlx::test]
    async fn test_get_valid_move_targets_excludes_current_parent() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let repo = PageRepository::new(pool.clone());

        // Get root page
        let root = repo.get_root_page(site_id).await?.unwrap();
        let root_id = root.id.unwrap();

        // Create page1 under root
        let page1 =
            Page::new_with_parent(site_id, root_id, "page1".to_string(), "Page 1".to_string());
        let page1_id = repo.create(&page1).await?;

        // Create page2 under root
        let page2 =
            Page::new_with_parent(site_id, root_id, "page2".to_string(), "Page 2".to_string());
        let page2_id = repo.create(&page2).await?;

        // Get valid move targets for page1 (which is currently under root)
        let targets = repo.get_valid_move_targets(page1_id).await?;

        // page1 should NOT be able to move to:
        // - itself (page1_id)
        // - its current parent (root_id)
        // So it should only have page2 as a valid target
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].id, Some(page2_id));

        // Verify root is not in the targets
        assert!(!targets.iter().any(|p| p.id == Some(root_id)));

        Ok(())
    }

    #[sqlx::test]
    async fn test_has_children() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());

        // Create a parent page
        let parent = Page::new_with_parent(
            site_id,
            root_id,
            "parent".to_string(),
            "Parent Page".to_string(),
        );
        let parent_id = repo.create(&parent).await?;

        // Parent should have no children initially
        assert!(!repo.has_children(parent_id).await?);

        // Create a child page
        let child = Page::new_with_parent(
            site_id,
            parent_id,
            "child".to_string(),
            "Child Page".to_string(),
        );
        let _child_id = repo.create(&child).await?;

        // Now parent should have children
        assert!(repo.has_children(parent_id).await?);

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_page_success() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());

        // Create a page
        let page = Page::new_with_parent(
            site_id,
            root_id,
            "test-page".to_string(),
            "Test Page".to_string(),
        );
        let page_id = repo.create(&page).await?;

        // Verify page exists
        assert!(repo.find_by_id(page_id).await?.is_some());

        // Delete the page
        repo.delete(page_id).await?;

        // Verify page is gone
        assert!(repo.find_by_id(page_id).await?.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_page_with_versions_and_components() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let root_id = get_root_page_id(&pool, site_id).await?;

        let page_repo = PageRepository::new(pool.clone());
        let version_repo =
            crate::repositories::page_version_repository::PageVersionRepository::new(pool.clone());
        let component_repo =
            crate::repositories::component_repository::ComponentRepository::new(pool.clone());

        // Create a page
        let page = Page::new_with_parent(
            site_id,
            root_id,
            "test-page".to_string(),
            "Test Page".to_string(),
        );
        let page_id = page_repo.create(&page).await?;

        // Create a version
        let version =
            doxyde_core::models::version::PageVersion::new(page_id, 1, Some("test".to_string()));
        let version_id = version_repo.create(&version).await?;

        // Create a component
        let component = doxyde_core::models::component::Component::new(
            version_id,
            "text".to_string(),
            0,
            serde_json::json!({ "text": "Test content" }),
        );
        let component_id = component_repo.create(&component).await?;

        // Verify everything exists
        assert!(page_repo.find_by_id(page_id).await?.is_some());
        assert!(version_repo.find_by_id(version_id).await?.is_some());
        assert!(component_repo.find_by_id(component_id).await?.is_some());

        // Delete the page
        page_repo.delete(page_id).await?;

        // Verify everything is gone
        assert!(page_repo.find_by_id(page_id).await?.is_none());
        assert!(version_repo.find_by_id(version_id).await?.is_none());
        assert!(component_repo.find_by_id(component_id).await?.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_generate_unique_slug() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());

        // First slug should remain unchanged
        let slug1 = repo
            .generate_unique_slug(site_id, Some(root_id), "about-us")
            .await?;
        assert_eq!(slug1, "about-us");

        // Create a page with that slug
        let page1 = Page::new_with_parent(site_id, root_id, slug1, "About Us".to_string());
        repo.create(&page1).await?;

        // Next slug with same base should get suffix
        let slug2 = repo
            .generate_unique_slug(site_id, Some(root_id), "about-us")
            .await?;
        assert_eq!(slug2, "about-us-2");

        // Create another page
        let page2 = Page::new_with_parent(site_id, root_id, slug2, "About Us 2".to_string());
        repo.create(&page2).await?;

        // Third slug should get suffix 3
        let slug3 = repo
            .generate_unique_slug(site_id, Some(root_id), "about-us")
            .await?;
        assert_eq!(slug3, "about-us-3");

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_with_auto_slug() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());

        // Create page with explicit slug
        let mut page1 = Page::new_with_parent(
            site_id,
            root_id,
            "custom-slug".to_string(),
            "My Page".to_string(),
        );
        let id1 = repo.create_with_auto_slug(&mut page1).await?;
        assert_eq!(page1.slug, "custom-slug");

        // Create page with empty slug - should auto-generate
        let mut page2 =
            Page::new_with_parent(site_id, root_id, "".to_string(), "My Page".to_string());
        let id2 = repo.create_with_auto_slug(&mut page2).await?;
        assert_eq!(page2.slug, "my-page");

        // Create another page with same title - should get suffix
        let mut page3 =
            Page::new_with_parent(site_id, root_id, "".to_string(), "My Page".to_string());
        let id3 = repo.create_with_auto_slug(&mut page3).await?;
        assert_eq!(page3.slug, "my-page-2");

        // Verify all pages were created
        assert!(repo.find_by_id(id1).await?.is_some());
        assert!(repo.find_by_id(id2).await?.is_some());
        assert!(repo.find_by_id(id3).await?.is_some());

        Ok(())
    }

    #[sqlx::test]
    async fn test_generate_unique_slug_different_parents() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());

        // Create two parent pages
        let parent1 = Page::new_with_parent(
            site_id,
            root_id,
            "parent1".to_string(),
            "Parent 1".to_string(),
        );
        let parent1_id = repo.create(&parent1).await?;

        let parent2 = Page::new_with_parent(
            site_id,
            root_id,
            "parent2".to_string(),
            "Parent 2".to_string(),
        );
        let parent2_id = repo.create(&parent2).await?;

        // Same slug should be allowed under different parents
        let slug1 = repo
            .generate_unique_slug(site_id, Some(parent1_id), "about")
            .await?;
        assert_eq!(slug1, "about");

        let page1 = Page::new_with_parent(site_id, parent1_id, slug1, "About".to_string());
        repo.create(&page1).await?;

        // Same slug under different parent should also work
        let slug2 = repo
            .generate_unique_slug(site_id, Some(parent2_id), "about")
            .await?;
        assert_eq!(slug2, "about");

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_children_sorted() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let site_id = setup_test_db(&pool).await?;
        let root_id = get_root_page_id(&pool, site_id).await?;

        let repo = PageRepository::new(pool.clone());

        // Create parent page with manual sort mode
        let mut parent = Page::new_with_parent(
            site_id,
            root_id,
            "parent".to_string(),
            "Parent Page".to_string(),
        );
        parent.sort_mode = "manual".to_string();
        let parent_id = repo.create(&parent).await?;

        // Create child pages with different positions and creation times
        let mut child1 = Page::new_with_parent(
            site_id,
            parent_id,
            "child1".to_string(),
            "B Child".to_string(),
        );
        child1.position = 2;
        child1.created_at = chrono::Utc::now() - chrono::Duration::days(2);
        let id1 = repo.create(&child1).await?;

        let mut child2 = Page::new_with_parent(
            site_id,
            parent_id,
            "child2".to_string(),
            "A Child".to_string(),
        );
        child2.position = 1;
        child2.created_at = chrono::Utc::now() - chrono::Duration::days(1);
        let id2 = repo.create(&child2).await?;

        let mut child3 = Page::new_with_parent(
            site_id,
            parent_id,
            "child3".to_string(),
            "C Child".to_string(),
        );
        child3.position = 0;
        child3.created_at = chrono::Utc::now();
        let id3 = repo.create(&child3).await?;

        // Test manual sorting (by position)
        let children = repo.list_children_sorted(parent_id).await?;
        assert_eq!(children.len(), 3);
        assert_eq!(children[0].id, Some(id3)); // position 0
        assert_eq!(children[1].id, Some(id2)); // position 1
        assert_eq!(children[2].id, Some(id1)); // position 2

        // Update parent to sort by created_at_asc
        parent.id = Some(parent_id);
        parent.sort_mode = "created_at_asc".to_string();
        repo.update(&parent).await?;

        let children = repo.list_children_sorted(parent_id).await?;
        assert_eq!(children[0].id, Some(id1)); // oldest first
        assert_eq!(children[1].id, Some(id2));
        assert_eq!(children[2].id, Some(id3)); // newest last

        // Update parent to sort by title_desc
        parent.sort_mode = "title_desc".to_string();
        repo.update(&parent).await?;

        let children = repo.list_children_sorted(parent_id).await?;
        assert_eq!(children[0].title, "C Child"); // C first
        assert_eq!(children[1].title, "B Child");
        assert_eq!(children[2].title, "A Child"); // A last

        Ok(())
    }
}
