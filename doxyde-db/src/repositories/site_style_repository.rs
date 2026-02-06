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
use doxyde_core::models::site_style::SiteStyle;
use sqlx::SqlitePool;

pub struct SiteStyleRepository {
    pool: SqlitePool,
}

impl SiteStyleRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, style: &SiteStyle) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO site_styles (name, css_content, is_active, priority, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&style.name)
        .bind(&style.css_content)
        .bind(style.is_active)
        .bind(style.priority)
        .bind(style.created_at)
        .bind(style.updated_at)
        .execute(&self.pool)
        .await
        .context("Failed to create site style")?;

        Ok(result.last_insert_rowid())
    }

    pub async fn find_by_id(&self, id: i64) -> Result<Option<SiteStyle>> {
        let row = sqlx::query_as::<_, (i64, String, String, bool, i32, String, String)>(
            r#"
            SELECT id, name, css_content, is_active, priority, created_at, updated_at
            FROM site_styles
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to find site style by ID")?;

        row.map(|r| row_to_style(r)).transpose()
    }

    pub async fn find_by_name(&self, name: &str) -> Result<Option<SiteStyle>> {
        let row = sqlx::query_as::<_, (i64, String, String, bool, i32, String, String)>(
            r#"
            SELECT id, name, css_content, is_active, priority, created_at, updated_at
            FROM site_styles
            WHERE name = ?
            "#,
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to find site style by name")?;

        row.map(|r| row_to_style(r)).transpose()
    }

    pub async fn list_all(&self) -> Result<Vec<SiteStyle>> {
        let rows = sqlx::query_as::<_, (i64, String, String, bool, i32, String, String)>(
            r#"
            SELECT id, name, css_content, is_active, priority, created_at, updated_at
            FROM site_styles
            ORDER BY priority ASC, name ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to list site styles")?;

        rows.into_iter().map(|r| row_to_style(r)).collect()
    }

    pub async fn list_active_ordered(&self) -> Result<Vec<SiteStyle>> {
        let rows = sqlx::query_as::<_, (i64, String, String, bool, i32, String, String)>(
            r#"
            SELECT id, name, css_content, is_active, priority, created_at, updated_at
            FROM site_styles
            WHERE is_active = 1
            ORDER BY priority ASC, name ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to list active site styles")?;

        rows.into_iter().map(|r| row_to_style(r)).collect()
    }

    pub async fn update(&self, style: &SiteStyle) -> Result<()> {
        let id = style.id.ok_or_else(|| anyhow::anyhow!("Style has no ID"))?;

        let rows = sqlx::query(
            r#"
            UPDATE site_styles
            SET name = ?, css_content = ?, is_active = ?, priority = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&style.name)
        .bind(&style.css_content)
        .bind(style.is_active)
        .bind(style.priority)
        .bind(chrono::Utc::now())
        .bind(id)
        .execute(&self.pool)
        .await
        .context("Failed to update site style")?
        .rows_affected();

        if rows == 0 {
            return Err(anyhow::anyhow!("Site style not found"));
        }
        Ok(())
    }

    pub async fn delete(&self, id: i64) -> Result<()> {
        let rows = sqlx::query("DELETE FROM site_styles WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete site style")?
            .rows_affected();

        if rows == 0 {
            return Err(anyhow::anyhow!("Site style not found"));
        }
        Ok(())
    }

    pub async fn get_combined_css(&self) -> Result<String> {
        let styles = self.list_active_ordered().await?;
        let combined = styles
            .iter()
            .map(|s| {
                format!(
                    "/* {} (priority: {}) */\n{}",
                    s.name, s.priority, s.css_content
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        Ok(combined)
    }
}

fn parse_datetime(s: &str) -> Result<chrono::DateTime<chrono::Utc>> {
    if s.contains('T') {
        chrono::DateTime::parse_from_rfc3339(s)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .context("Failed to parse datetime as RFC3339")
    } else {
        chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
            .map(|dt| dt.and_utc())
            .context("Failed to parse datetime as SQLite format")
    }
}

fn row_to_style(row: (i64, String, String, bool, i32, String, String)) -> Result<SiteStyle> {
    let (id, name, css_content, is_active, priority, created_at_str, updated_at_str) = row;
    Ok(SiteStyle {
        id: Some(id),
        name,
        css_content,
        is_active,
        priority,
        created_at: parse_datetime(&created_at_str)?,
        updated_at: parse_datetime(&updated_at_str)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_test_db(pool: &SqlitePool) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS site_styles (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                css_content TEXT NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1,
                priority INTEGER NOT NULL DEFAULT 0,
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
    async fn test_create_and_find_by_id() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteStyleRepository::new(pool);
        let s = SiteStyle::new("main".to_string(), "body { color: red; }".to_string());
        let id = repo.create(&s).await?;
        assert!(id > 0);

        let found = repo.find_by_id(id).await?;
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.name, "main");
        assert_eq!(found.css_content, "body { color: red; }");

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_name() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteStyleRepository::new(pool);
        let s = SiteStyle::new("theme".to_string(), ".theme {}".to_string());
        repo.create(&s).await?;

        assert!(repo.find_by_name("theme").await?.is_some());
        assert!(repo.find_by_name("nope").await?.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_active_ordered() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteStyleRepository::new(pool);

        let mut s1 = SiteStyle::new("reset".to_string(), "* { margin: 0; }".to_string());
        s1.priority = 0;
        repo.create(&s1).await?;

        let mut s2 = SiteStyle::new(
            "theme".to_string(),
            "body { background: blue; }".to_string(),
        );
        s2.priority = 10;
        repo.create(&s2).await?;

        let mut s3 = SiteStyle::new("inactive".to_string(), "unused".to_string());
        s3.is_active = false;
        repo.create(&s3).await?;

        let active = repo.list_active_ordered().await?;
        assert_eq!(active.len(), 2);
        assert_eq!(active[0].name, "reset");
        assert_eq!(active[1].name, "theme");

        Ok(())
    }

    #[sqlx::test]
    async fn test_get_combined_css() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteStyleRepository::new(pool);

        let mut s1 = SiteStyle::new("reset".to_string(), "* { margin: 0; }".to_string());
        s1.priority = 0;
        repo.create(&s1).await?;

        let mut s2 = SiteStyle::new(
            "theme".to_string(),
            "body { background: blue; }".to_string(),
        );
        s2.priority = 10;
        repo.create(&s2).await?;

        let css = repo.get_combined_css().await?;
        assert!(css.contains("* { margin: 0; }"));
        assert!(css.contains("body { background: blue; }"));
        // reset should come before theme due to priority
        let reset_pos = css.find("* { margin: 0; }").unwrap();
        let theme_pos = css.find("body { background: blue; }").unwrap();
        assert!(reset_pos < theme_pos);

        Ok(())
    }

    #[sqlx::test]
    async fn test_get_combined_css_empty() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteStyleRepository::new(pool);
        let css = repo.get_combined_css().await?;
        assert!(css.is_empty());

        Ok(())
    }

    #[sqlx::test]
    async fn test_update() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteStyleRepository::new(pool);
        let s = SiteStyle::new("main".to_string(), "old".to_string());
        let id = repo.create(&s).await?;

        let mut found = repo.find_by_id(id).await?.unwrap();
        found.css_content = "new".to_string();
        found.priority = 5;
        repo.update(&found).await?;

        let updated = repo.find_by_id(id).await?.unwrap();
        assert_eq!(updated.css_content, "new");
        assert_eq!(updated.priority, 5);

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteStyleRepository::new(pool);
        let s = SiteStyle::new("main".to_string(), "content".to_string());
        let id = repo.create(&s).await?;

        repo.delete(id).await?;
        assert!(repo.find_by_id(id).await?.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_nonexistent_fails() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteStyleRepository::new(pool);
        assert!(repo.delete(999).await.is_err());

        Ok(())
    }
}
