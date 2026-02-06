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
use doxyde_core::models::site_template::SiteTemplate;
use sqlx::SqlitePool;

pub struct SiteTemplateRepository {
    pool: SqlitePool,
}

impl SiteTemplateRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, template: &SiteTemplate) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO site_templates (template_name, content, is_active, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&template.template_name)
        .bind(&template.content)
        .bind(template.is_active)
        .bind(template.created_at)
        .bind(template.updated_at)
        .execute(&self.pool)
        .await
        .context("Failed to create site template")?;

        Ok(result.last_insert_rowid())
    }

    pub async fn find_by_id(&self, id: i64) -> Result<Option<SiteTemplate>> {
        let row = sqlx::query_as::<_, (i64, String, String, bool, String, String)>(
            r#"
            SELECT id, template_name, content, is_active, created_at, updated_at
            FROM site_templates
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to find site template by ID")?;

        row.map(|r| row_to_template(r)).transpose()
    }

    pub async fn find_by_name(&self, name: &str) -> Result<Option<SiteTemplate>> {
        let row = sqlx::query_as::<_, (i64, String, String, bool, String, String)>(
            r#"
            SELECT id, template_name, content, is_active, created_at, updated_at
            FROM site_templates
            WHERE template_name = ?
            "#,
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to find site template by name")?;

        row.map(|r| row_to_template(r)).transpose()
    }

    pub async fn find_active_by_name(&self, name: &str) -> Result<Option<SiteTemplate>> {
        let row = sqlx::query_as::<_, (i64, String, String, bool, String, String)>(
            r#"
            SELECT id, template_name, content, is_active, created_at, updated_at
            FROM site_templates
            WHERE template_name = ? AND is_active = 1
            "#,
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to find active site template by name")?;

        row.map(|r| row_to_template(r)).transpose()
    }

    pub async fn list_all(&self) -> Result<Vec<SiteTemplate>> {
        let rows = sqlx::query_as::<_, (i64, String, String, bool, String, String)>(
            r#"
            SELECT id, template_name, content, is_active, created_at, updated_at
            FROM site_templates
            ORDER BY template_name
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to list site templates")?;

        rows.into_iter().map(|r| row_to_template(r)).collect()
    }

    pub async fn update(&self, template: &SiteTemplate) -> Result<()> {
        let id = template
            .id
            .ok_or_else(|| anyhow::anyhow!("Template has no ID"))?;

        let rows = sqlx::query(
            r#"
            UPDATE site_templates
            SET template_name = ?, content = ?, is_active = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&template.template_name)
        .bind(&template.content)
        .bind(template.is_active)
        .bind(chrono::Utc::now())
        .bind(id)
        .execute(&self.pool)
        .await
        .context("Failed to update site template")?
        .rows_affected();

        if rows == 0 {
            return Err(anyhow::anyhow!("Site template not found"));
        }
        Ok(())
    }

    pub async fn delete(&self, id: i64) -> Result<()> {
        let rows = sqlx::query("DELETE FROM site_templates WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete site template")?
            .rows_affected();

        if rows == 0 {
            return Err(anyhow::anyhow!("Site template not found"));
        }
        Ok(())
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

fn row_to_template(row: (i64, String, String, bool, String, String)) -> Result<SiteTemplate> {
    let (id, template_name, content, is_active, created_at_str, updated_at_str) = row;
    Ok(SiteTemplate {
        id: Some(id),
        template_name,
        content,
        is_active,
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
            CREATE TABLE IF NOT EXISTS site_templates (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                template_name TEXT NOT NULL UNIQUE,
                content TEXT NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1,
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

        let repo = SiteTemplateRepository::new(pool);
        let t = SiteTemplate::new("base.html".to_string(), "<html></html>".to_string());
        let id = repo.create(&t).await?;
        assert!(id > 0);

        let found = repo.find_by_id(id).await?;
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.template_name, "base.html");
        assert_eq!(found.content, "<html></html>");
        assert!(found.is_active);

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_name() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteTemplateRepository::new(pool);
        let t = SiteTemplate::new(
            "page_templates/blog.html".to_string(),
            "<p>blog</p>".to_string(),
        );
        repo.create(&t).await?;

        let found = repo.find_by_name("page_templates/blog.html").await?;
        assert!(found.is_some());

        let not_found = repo.find_by_name("nonexistent.html").await?;
        assert!(not_found.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_active_by_name() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteTemplateRepository::new(pool);
        let mut t = SiteTemplate::new("base.html".to_string(), "<html></html>".to_string());
        t.is_active = false;
        repo.create(&t).await?;

        let found = repo.find_active_by_name("base.html").await?;
        assert!(found.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_all() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteTemplateRepository::new(pool);
        repo.create(&SiteTemplate::new("a.html".to_string(), "a".to_string()))
            .await?;
        repo.create(&SiteTemplate::new("b.html".to_string(), "b".to_string()))
            .await?;

        let all = repo.list_all().await?;
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].template_name, "a.html");
        assert_eq!(all[1].template_name, "b.html");

        Ok(())
    }

    #[sqlx::test]
    async fn test_update() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteTemplateRepository::new(pool);
        let t = SiteTemplate::new("base.html".to_string(), "old".to_string());
        let id = repo.create(&t).await?;

        let mut found = repo.find_by_id(id).await?.unwrap();
        found.content = "new".to_string();
        repo.update(&found).await?;

        let updated = repo.find_by_id(id).await?.unwrap();
        assert_eq!(updated.content, "new");

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteTemplateRepository::new(pool);
        let t = SiteTemplate::new("base.html".to_string(), "content".to_string());
        let id = repo.create(&t).await?;

        repo.delete(id).await?;
        let found = repo.find_by_id(id).await?;
        assert!(found.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_nonexistent_fails() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteTemplateRepository::new(pool);
        let result = repo.delete(999).await;
        assert!(result.is_err());

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_duplicate_name_fails() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteTemplateRepository::new(pool);
        let t = SiteTemplate::new("base.html".to_string(), "first".to_string());
        repo.create(&t).await?;

        let t2 = SiteTemplate::new("base.html".to_string(), "second".to_string());
        let result = repo.create(&t2).await;
        assert!(result.is_err());

        Ok(())
    }
}
