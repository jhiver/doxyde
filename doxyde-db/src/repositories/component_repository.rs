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
use doxyde_core::models::component::Component;
use sqlx::SqlitePool;

pub struct ComponentRepository {
    pool: SqlitePool,
}

impl ComponentRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, component: &Component) -> Result<i64> {
        if component.is_valid().is_err() {
            return Err(anyhow::anyhow!(
                "Invalid component: {:?}",
                component.is_valid().err()
            ));
        }

        let content_json = serde_json::to_string(&component.content)
            .context("Failed to serialize component content")?;

        let result = sqlx::query(
            r#"
            INSERT INTO components (page_version_id, component_type, position, content, title, template, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(component.page_version_id)
        .bind(&component.component_type)
        .bind(component.position)
        .bind(&content_json)
        .bind(&component.title)
        .bind(&component.template)
        .bind(component.created_at)
        .bind(component.updated_at)
        .execute(&self.pool)
        .await
        .context("Failed to create component")?;

        Ok(result.last_insert_rowid())
    }

    pub async fn find_by_id(&self, id: i64) -> Result<Option<Component>> {
        let row = sqlx::query_as::<
            _,
            (
                i64,
                i64,
                String,
                i32,
                String,
                Option<String>,
                String,
                String,
                String,
            ),
        >(
            r#"
            SELECT 
                id,
                page_version_id,
                component_type,
                position,
                content,
                title,
                template,
                created_at,
                updated_at
            FROM components
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to find component by id")?;

        match row {
            Some((
                id,
                page_version_id,
                component_type,
                position,
                content_str,
                title,
                template,
                created_at_str,
                updated_at_str,
            )) => {
                let content = serde_json::from_str(&content_str)
                    .context("Failed to deserialize component content")?;

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

                Ok(Some(Component {
                    id: Some(id),
                    page_version_id,
                    component_type,
                    position,
                    content,
                    title,
                    template,
                    created_at,
                    updated_at,
                }))
            }
            None => Ok(None),
        }
    }

    pub async fn list_by_page_version(&self, page_version_id: i64) -> Result<Vec<Component>> {
        let rows = sqlx::query_as::<
            _,
            (
                i64,
                i64,
                String,
                i32,
                String,
                Option<String>,
                String,
                String,
                String,
            ),
        >(
            r#"
            SELECT 
                id,
                page_version_id,
                component_type,
                position,
                content,
                title,
                template,
                created_at,
                updated_at
            FROM components
            WHERE page_version_id = ?
            ORDER BY position ASC, id ASC
            "#,
        )
        .bind(page_version_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list components by page version")?;

        let mut components = Vec::new();

        for (
            id,
            page_version_id,
            component_type,
            position,
            content_str,
            title,
            template,
            created_at_str,
            updated_at_str,
        ) in rows
        {
            let content = serde_json::from_str(&content_str)
                .context("Failed to deserialize component content")?;

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

            components.push(Component {
                id: Some(id),
                page_version_id,
                component_type,
                position,
                content,
                title,
                template,
                created_at,
                updated_at,
            });
        }

        Ok(components)
    }

    pub async fn update(&self, component: &Component) -> Result<()> {
        if component.id.is_none() {
            return Err(anyhow::anyhow!("Cannot update component without id"));
        }

        if component.is_valid().is_err() {
            return Err(anyhow::anyhow!(
                "Invalid component: {:?}",
                component.is_valid().err()
            ));
        }

        let content_json = serde_json::to_string(&component.content)
            .context("Failed to serialize component content")?;

        let rows_affected = sqlx::query(
            r#"
            UPDATE components
            SET page_version_id = ?,
                component_type = ?,
                position = ?,
                content = ?,
                title = ?,
                template = ?,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(component.page_version_id)
        .bind(&component.component_type)
        .bind(component.position)
        .bind(&content_json)
        .bind(&component.title)
        .bind(&component.template)
        .bind(component.updated_at)
        .bind(component.id.unwrap())
        .execute(&self.pool)
        .await
        .context("Failed to update component")?
        .rows_affected();

        if rows_affected == 0 {
            return Err(anyhow::anyhow!("Component not found"));
        }

        Ok(())
    }

    pub async fn delete(&self, id: i64) -> Result<()> {
        // Get the component's page_version_id before deleting
        let component = self
            .find_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Component not found"))?;

        let page_version_id = component.page_version_id;

        sqlx::query("DELETE FROM components WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete component")?;

        // Normalize positions after deletion
        self.normalize_positions(page_version_id).await?;

        Ok(())
    }

    pub async fn reorder(&self, page_version_id: i64, positions: Vec<(i64, i32)>) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        for (component_id, new_position) in positions {
            sqlx::query("UPDATE components SET position = ? WHERE id = ? AND page_version_id = ?")
                .bind(new_position)
                .bind(component_id)
                .bind(page_version_id)
                .execute(&mut *tx)
                .await
                .context("Failed to update component position")?;
        }

        tx.commit()
            .await
            .context("Failed to commit reorder transaction")?;
        Ok(())
    }

    /// Update a component's content, title, and template
    pub async fn update_content(
        &self,
        id: i64,
        content: serde_json::Value,
        title: Option<String>,
        template: String,
    ) -> Result<()> {
        tracing::info!(
            "ComponentRepository::update_content called for component {}",
            id
        );
        tracing::info!("  - title: {:?}", title);
        tracing::info!("  - template: {}", template);

        let content_json =
            serde_json::to_string(&content).context("Failed to serialize component content")?;

        tracing::info!("  - SQL UPDATE query parameters:");
        tracing::info!("    - content: {} chars", content_json.len());
        tracing::info!("    - title: {:?}", title);
        tracing::info!("    - template: {}", template);
        tracing::info!("    - id: {}", id);

        let result = sqlx::query(
            r#"
            UPDATE components 
            SET content = ?, title = ?, template = ?, updated_at = datetime('now')
            WHERE id = ?
            "#,
        )
        .bind(&content_json)
        .bind(&title)
        .bind(&template)
        .bind(id)
        .execute(&self.pool)
        .await
        .context("Failed to update component")?;

        tracing::info!(
            "  - UPDATE result: {} rows affected",
            result.rows_affected()
        );

        Ok(())
    }

    /// Move a component up in the position order
    pub async fn move_up(&self, id: i64) -> Result<()> {
        // First, get the component and its current position
        let component = self
            .find_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Component not found"))?;

        if component.position == 0 {
            return Ok(()); // Already at the top
        }

        let mut tx = self.pool.begin().await?;

        // Find the component above
        let above = sqlx::query_as::<_, (i64, i32)>(
            "SELECT id, position FROM components WHERE page_version_id = ? AND position = ? LIMIT 1"
        )
        .bind(component.page_version_id)
        .bind(component.position - 1)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some((above_id, above_position)) = above {
            // Swap positions
            sqlx::query("UPDATE components SET position = ? WHERE id = ?")
                .bind(component.position)
                .bind(above_id)
                .execute(&mut *tx)
                .await?;

            sqlx::query("UPDATE components SET position = ? WHERE id = ?")
                .bind(above_position)
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;

        // Normalize positions after move
        self.normalize_positions(component.page_version_id).await?;

        Ok(())
    }

    /// Move a component down in the position order
    pub async fn move_down(&self, id: i64) -> Result<()> {
        // First, get the component and its current position
        let component = self
            .find_by_id(id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Component not found"))?;

        let mut tx = self.pool.begin().await?;

        // Find the component below
        let below = sqlx::query_as::<_, (i64, i32)>(
            "SELECT id, position FROM components WHERE page_version_id = ? AND position = ? LIMIT 1"
        )
        .bind(component.page_version_id)
        .bind(component.position + 1)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some((below_id, below_position)) = below {
            // Swap positions
            sqlx::query("UPDATE components SET position = ? WHERE id = ?")
                .bind(component.position)
                .bind(below_id)
                .execute(&mut *tx)
                .await?;

            sqlx::query("UPDATE components SET position = ? WHERE id = ?")
                .bind(below_position)
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;

        // Normalize positions after move
        self.normalize_positions(component.page_version_id).await?;

        Ok(())
    }

    /// Copy all components from one version to another
    pub async fn copy_all(&self, from_version_id: i64, to_version_id: i64) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO components (page_version_id, component_type, position, content, title, template, created_at, updated_at)
            SELECT ?, component_type, position, content, title, template, datetime('now'), datetime('now')
            FROM components
            WHERE page_version_id = ?
            ORDER BY position ASC
            "#,
        )
        .bind(to_version_id)
        .bind(from_version_id)
        .execute(&self.pool)
        .await
        .context("Failed to copy components")?;

        // Normalize positions in the new version
        self.normalize_positions(to_version_id).await?;

        Ok(())
    }

    /// Normalize component positions to ensure they are sequential (0, 1, 2, ...)
    pub async fn normalize_positions(&self, page_version_id: i64) -> Result<()> {
        // Get all components for this version ordered by current position
        let components = sqlx::query!(
            r#"
            SELECT id as "id: i64", position as "position: i32"
            FROM components 
            WHERE page_version_id = ?
            ORDER BY position, id
            "#,
            page_version_id
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch components for normalization")?;

        // Update positions to be sequential
        let mut tx = self.pool.begin().await?;

        for (new_position, component) in components.iter().enumerate() {
            let new_pos = new_position as i32;
            if component.position != new_pos {
                sqlx::query!(
                    "UPDATE components SET position = ? WHERE id = ?",
                    new_pos,
                    component.id
                )
                .execute(&mut *tx)
                .await
                .context("Failed to update component position during normalization")?;
            }
        }

        tx.commit()
            .await
            .context("Failed to commit position normalization")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use serde_json::json;

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

        // Create components table
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

        Ok(())
    }

    #[sqlx::test]
    async fn test_new_creates_repository() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;

        let repo = ComponentRepository::new(pool.clone());

        // Verify we can access the pool by doing a simple query
        let _result = sqlx::query("SELECT 1").fetch_one(&repo.pool).await?;

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_component_success() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        // Create test data hierarchy
        let site_id = sqlx::query("INSERT INTO sites (domain, title) VALUES (?, ?)")
            .bind("test.com")
            .bind("Test Site")
            .execute(&pool)
            .await?
            .last_insert_rowid();

        let page_id = sqlx::query("INSERT INTO pages (site_id, slug, title) VALUES (?, ?, ?)")
            .bind(site_id)
            .bind("test-page")
            .bind("Test Page")
            .execute(&pool)
            .await?
            .last_insert_rowid();

        let version_id = sqlx::query(
            "INSERT INTO page_versions (page_id, version_number, created_by) VALUES (?, ?, ?)",
        )
        .bind(page_id)
        .bind(1)
        .bind("test@example.com")
        .execute(&pool)
        .await?
        .last_insert_rowid();

        // Create component
        let repo = ComponentRepository::new(pool.clone());
        let component = Component::new(
            version_id,
            "text".to_string(),
            0,
            serde_json::json!({"text": "Hello, world!"}),
        );

        let id = repo.create(&component).await?;
        assert!(id > 0);

        // Verify it was created
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM components WHERE id = ?")
            .bind(id)
            .fetch_one(&pool)
            .await?;
        assert_eq!(count.0, 1);

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_component_with_different_types() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        // Setup test data
        sqlx::query("INSERT INTO sites (domain, title) VALUES (?, ?)")
            .bind("test.com")
            .bind("Test")
            .execute(&pool)
            .await?;

        sqlx::query("INSERT INTO pages (site_id, slug, title) VALUES (?, ?, ?)")
            .bind(1)
            .bind("page")
            .bind("Page")
            .execute(&pool)
            .await?;

        let version_id =
            sqlx::query("INSERT INTO page_versions (page_id, version_number) VALUES (?, ?)")
                .bind(1)
                .bind(1)
                .execute(&pool)
                .await?
                .last_insert_rowid();

        let repo = ComponentRepository::new(pool.clone());

        // Test different component types
        let test_cases = vec![
            ("text", serde_json::json!({"text": "Hello"})),
            (
                "image",
                serde_json::json!({"src": "/img.jpg", "alt": "Image"}),
            ),
            (
                "code",
                serde_json::json!({"code": "println!()", "language": "rust"}),
            ),
            ("custom", serde_json::json!({"any": "data"})),
        ];

        for (i, (comp_type, content)) in test_cases.iter().enumerate() {
            let component =
                Component::new(version_id, comp_type.to_string(), i as i32, content.clone());

            let id = repo.create(&component).await?;
            assert!(id > 0);

            // Verify stored data
            let row: (String, String, i32) = sqlx::query_as(
                "SELECT component_type, content, position FROM components WHERE id = ?",
            )
            .bind(id)
            .fetch_one(&pool)
            .await?;

            assert_eq!(row.0, *comp_type);
            assert_eq!(row.1, content.to_string());
            assert_eq!(row.2, i as i32);
        }

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_component_invalid_fails() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = ComponentRepository::new(pool);

        // Test with invalid page_version_id
        let invalid_component = Component::new(
            0, // Invalid ID
            "text".to_string(),
            0,
            serde_json::json!({"text": "Hello"}),
        );

        let result = repo.create(&invalid_component).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid component"));

        Ok(())
    }

    #[sqlx::test]
    async fn test_create_component_foreign_key_constraint() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = ComponentRepository::new(pool);

        // Try to create with non-existent page_version_id
        let component = Component::new(
            999, // Non-existent version
            "text".to_string(),
            0,
            serde_json::json!({"text": "Hello"}),
        );

        let result = repo.create(&component).await;
        assert!(result.is_err());

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_id_existing() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        // Setup test data
        sqlx::query("INSERT INTO sites (domain, title) VALUES (?, ?)")
            .bind("test.com")
            .bind("Test")
            .execute(&pool)
            .await?;

        sqlx::query("INSERT INTO pages (site_id, slug, title) VALUES (?, ?, ?)")
            .bind(1)
            .bind("page")
            .bind("Page")
            .execute(&pool)
            .await?;

        let version_id =
            sqlx::query("INSERT INTO page_versions (page_id, version_number) VALUES (?, ?)")
                .bind(1)
                .bind(1)
                .execute(&pool)
                .await?
                .last_insert_rowid();

        let repo = ComponentRepository::new(pool);

        // Create a component
        let component = Component::new(
            version_id,
            "text".to_string(),
            0,
            json!({"text": "Hello, world!"}),
        );
        let id = repo.create(&component).await?;

        // Find it
        let found = repo.find_by_id(id).await?;
        assert!(found.is_some());

        let found = found.unwrap();
        assert_eq!(found.id, Some(id));
        assert_eq!(found.page_version_id, version_id);
        assert_eq!(found.component_type, "text");
        assert_eq!(found.position, 0);
        assert_eq!(found.content, json!({"text": "Hello, world!"}));

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_id_non_existing() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = ComponentRepository::new(pool);

        let found = repo.find_by_id(999).await?;
        assert!(found.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_id_zero_and_negative() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = ComponentRepository::new(pool);

        let found = repo.find_by_id(0).await?;
        assert!(found.is_none());

        let found = repo.find_by_id(-1).await?;
        assert!(found.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_id_preserves_content_structure() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        // Setup test data
        sqlx::query("INSERT INTO sites (domain, title) VALUES (?, ?)")
            .bind("test.com")
            .bind("Test")
            .execute(&pool)
            .await?;

        sqlx::query("INSERT INTO pages (site_id, slug, title) VALUES (?, ?, ?)")
            .bind(1)
            .bind("page")
            .bind("Page")
            .execute(&pool)
            .await?;

        let version_id =
            sqlx::query("INSERT INTO page_versions (page_id, version_number) VALUES (?, ?)")
                .bind(1)
                .bind(1)
                .execute(&pool)
                .await?
                .last_insert_rowid();

        let repo = ComponentRepository::new(pool);

        // Test with complex JSON content
        let complex_content = json!({
            "title": "Complex Component",
            "items": [1, 2, 3],
            "nested": {
                "key": "value",
                "array": ["a", "b", "c"]
            },
            "boolean": true,
            "null_value": null
        });

        let component =
            Component::new(version_id, "custom".to_string(), 0, complex_content.clone());
        let id = repo.create(&component).await?;

        // Find and verify content structure is preserved
        let found = repo.find_by_id(id).await?.unwrap();
        assert_eq!(found.content, complex_content);

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_by_page_version_empty() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = ComponentRepository::new(pool);

        let components = repo.list_by_page_version(999).await?;
        assert!(components.is_empty());

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_by_page_version_single() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        // Setup test data
        sqlx::query("INSERT INTO sites (domain, title) VALUES (?, ?)")
            .bind("test.com")
            .bind("Test")
            .execute(&pool)
            .await?;

        sqlx::query("INSERT INTO pages (site_id, slug, title) VALUES (?, ?, ?)")
            .bind(1)
            .bind("page")
            .bind("Page")
            .execute(&pool)
            .await?;

        let version_id =
            sqlx::query("INSERT INTO page_versions (page_id, version_number) VALUES (?, ?)")
                .bind(1)
                .bind(1)
                .execute(&pool)
                .await?
                .last_insert_rowid();

        let repo = ComponentRepository::new(pool);

        // Create a component
        let component = Component::new(version_id, "text".to_string(), 0, json!({"text": "Hello"}));
        repo.create(&component).await?;

        // List components
        let components = repo.list_by_page_version(version_id).await?;
        assert_eq!(components.len(), 1);
        assert_eq!(components[0].component_type, "text");
        assert_eq!(components[0].position, 0);

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_by_page_version_multiple_ordered() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        // Setup test data
        sqlx::query("INSERT INTO sites (domain, title) VALUES (?, ?)")
            .bind("test.com")
            .bind("Test")
            .execute(&pool)
            .await?;

        sqlx::query("INSERT INTO pages (site_id, slug, title) VALUES (?, ?, ?)")
            .bind(1)
            .bind("page")
            .bind("Page")
            .execute(&pool)
            .await?;

        let version_id =
            sqlx::query("INSERT INTO page_versions (page_id, version_number) VALUES (?, ?)")
                .bind(1)
                .bind(1)
                .execute(&pool)
                .await?
                .last_insert_rowid();

        let repo = ComponentRepository::new(pool);

        // Create components in random order
        let components_data = vec![
            (2, "image", json!({"src": "/img2.jpg"})),
            (0, "text", json!({"text": "First"})),
            (1, "code", json!({"code": "println!()"})),
        ];

        for (pos, comp_type, content) in components_data {
            let component = Component::new(version_id, comp_type.to_string(), pos, content);
            repo.create(&component).await?;
        }

        // List should be ordered by position
        let components = repo.list_by_page_version(version_id).await?;
        assert_eq!(components.len(), 3);
        assert_eq!(components[0].position, 0);
        assert_eq!(components[0].component_type, "text");
        assert_eq!(components[1].position, 1);
        assert_eq!(components[1].component_type, "code");
        assert_eq!(components[2].position, 2);
        assert_eq!(components[2].component_type, "image");

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_by_page_version_different_versions() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        // Setup test data
        sqlx::query("INSERT INTO sites (domain, title) VALUES (?, ?)")
            .bind("test.com")
            .bind("Test")
            .execute(&pool)
            .await?;

        sqlx::query("INSERT INTO pages (site_id, slug, title) VALUES (?, ?, ?)")
            .bind(1)
            .bind("page")
            .bind("Page")
            .execute(&pool)
            .await?;

        // Create two versions
        let version1_id =
            sqlx::query("INSERT INTO page_versions (page_id, version_number) VALUES (?, ?)")
                .bind(1)
                .bind(1)
                .execute(&pool)
                .await?
                .last_insert_rowid();

        let version2_id =
            sqlx::query("INSERT INTO page_versions (page_id, version_number) VALUES (?, ?)")
                .bind(1)
                .bind(2)
                .execute(&pool)
                .await?
                .last_insert_rowid();

        let repo = ComponentRepository::new(pool);

        // Create components for different versions
        let comp1 = Component::new(version1_id, "text".to_string(), 0, json!({"text": "v1"}));
        let comp2 = Component::new(version2_id, "text".to_string(), 0, json!({"text": "v2"}));
        let comp3 = Component::new(
            version2_id,
            "image".to_string(),
            1,
            json!({"src": "/img.jpg"}),
        );

        repo.create(&comp1).await?;
        repo.create(&comp2).await?;
        repo.create(&comp3).await?;

        // List for version 1
        let v1_components = repo.list_by_page_version(version1_id).await?;
        assert_eq!(v1_components.len(), 1);
        assert_eq!(v1_components[0].content["text"], "v1");

        // List for version 2
        let v2_components = repo.list_by_page_version(version2_id).await?;
        assert_eq!(v2_components.len(), 2);
        assert_eq!(v2_components[0].content["text"], "v2");
        assert_eq!(v2_components[1].component_type, "image");

        Ok(())
    }

    #[sqlx::test]
    async fn test_update_component_success() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        // Setup test data
        sqlx::query("INSERT INTO sites (domain, title) VALUES ('test.com', 'Test')")
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO pages (site_id, slug, title) VALUES (1, 'page', 'Page')")
            .execute(&pool)
            .await?;
        let version_id =
            sqlx::query("INSERT INTO page_versions (page_id, version_number) VALUES (1, 1)")
                .execute(&pool)
                .await?
                .last_insert_rowid();

        let repo = ComponentRepository::new(pool.clone());

        // Create a component
        let mut component = Component::new(
            version_id,
            "text".to_string(),
            0,
            json!({"text": "Original"}),
        );
        let id = repo.create(&component).await?;
        component.id = Some(id);

        // Update it
        component.component_type = "code".to_string();
        component.position = 5;
        component.content = json!({"code": "Updated"});
        component.updated_at = chrono::Utc::now();

        repo.update(&component).await?;

        // Verify update
        let found = repo.find_by_id(id).await?.unwrap();
        assert_eq!(found.component_type, "code");
        assert_eq!(found.position, 5);
        assert_eq!(found.content["code"], "Updated");

        Ok(())
    }

    #[sqlx::test]
    async fn test_update_component_without_id_fails() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = ComponentRepository::new(pool);
        let component = Component::new(1, "text".to_string(), 0, json!({"text": "Test"}));

        let result = repo.update(&component).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("without id"));

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_component_success() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        // Setup test data
        sqlx::query("INSERT INTO sites (domain, title) VALUES ('test.com', 'Test')")
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO pages (site_id, slug, title) VALUES (1, 'page', 'Page')")
            .execute(&pool)
            .await?;
        let version_id =
            sqlx::query("INSERT INTO page_versions (page_id, version_number) VALUES (1, 1)")
                .execute(&pool)
                .await?
                .last_insert_rowid();

        let repo = ComponentRepository::new(pool);

        // Create and delete
        let component = Component::new(
            version_id,
            "text".to_string(),
            0,
            json!({"text": "Delete me"}),
        );
        let id = repo.create(&component).await?;

        repo.delete(id).await?;

        // Verify deleted
        let found = repo.find_by_id(id).await?;
        assert!(found.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_reorder_components() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        // Setup test data
        sqlx::query("INSERT INTO sites (domain, title) VALUES ('test.com', 'Test')")
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO pages (site_id, slug, title) VALUES (1, 'page', 'Page')")
            .execute(&pool)
            .await?;
        let version_id =
            sqlx::query("INSERT INTO page_versions (page_id, version_number) VALUES (1, 1)")
                .execute(&pool)
                .await?
                .last_insert_rowid();

        let repo = ComponentRepository::new(pool);

        // Create components
        let comp1 = Component::new(version_id, "text".to_string(), 0, json!({"text": "First"}));
        let comp2 = Component::new(
            version_id,
            "image".to_string(),
            1,
            json!({"src": "/img.jpg"}),
        );
        let comp3 = Component::new(
            version_id,
            "code".to_string(),
            2,
            json!({"code": "print()"}),
        );

        let id1 = repo.create(&comp1).await?;
        let _id2 = repo.create(&comp2).await?;
        let id3 = repo.create(&comp3).await?;

        // Reorder: swap first and last
        repo.reorder(version_id, vec![(id1, 2), (id3, 0)]).await?;

        // Verify new order
        let components = repo.list_by_page_version(version_id).await?;
        assert_eq!(components.len(), 3);
        assert_eq!(components[0].component_type, "code"); // was position 2, now 0
        assert_eq!(components[1].component_type, "image"); // unchanged
        assert_eq!(components[2].component_type, "text"); // was position 0, now 2

        Ok(())
    }

    #[sqlx::test]
    async fn test_normalize_positions() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        // Setup test data
        sqlx::query("INSERT INTO sites (domain, title) VALUES ('test.com', 'Test')")
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO pages (site_id, slug, title) VALUES (1, 'page', 'Page')")
            .execute(&pool)
            .await?;
        let version_id =
            sqlx::query("INSERT INTO page_versions (page_id, version_number) VALUES (1, 1)")
                .execute(&pool)
                .await?
                .last_insert_rowid();

        // Manually insert components with non-sequential positions
        sqlx::query("INSERT INTO components (page_version_id, component_type, position, content) VALUES (?, 'text', 0, '{}')")
            .bind(version_id)
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO components (page_version_id, component_type, position, content) VALUES (?, 'text', 2, '{}')")
            .bind(version_id)
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO components (page_version_id, component_type, position, content) VALUES (?, 'text', 5, '{}')")
            .bind(version_id)
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO components (page_version_id, component_type, position, content) VALUES (?, 'text', 5, '{}')")
            .bind(version_id)
            .execute(&pool)
            .await?;

        let repo = ComponentRepository::new(pool);

        // Normalize positions
        repo.normalize_positions(version_id).await?;

        // Verify positions are now sequential
        let components = repo.list_by_page_version(version_id).await?;
        assert_eq!(components.len(), 4);

        for (i, component) in components.iter().enumerate() {
            assert_eq!(
                component.position, i as i32,
                "Component at index {} should have position {}",
                i, i
            );
        }

        Ok(())
    }

    #[sqlx::test]
    async fn test_move_up() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        // Setup test data
        sqlx::query("INSERT INTO sites (domain, title) VALUES ('test.com', 'Test')")
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO pages (site_id, slug, title) VALUES (1, 'page', 'Page')")
            .execute(&pool)
            .await?;
        let version_id =
            sqlx::query("INSERT INTO page_versions (page_id, version_number) VALUES (1, 1)")
                .execute(&pool)
                .await?
                .last_insert_rowid();

        let repo = ComponentRepository::new(pool);

        // Create three components
        let comp1 = Component::new(version_id, "text".to_string(), 0, json!({"text": "First"}));
        let comp2 = Component::new(version_id, "text".to_string(), 1, json!({"text": "Second"}));
        let comp3 = Component::new(version_id, "text".to_string(), 2, json!({"text": "Third"}));

        let _id1 = repo.create(&comp1).await?;
        let id2 = repo.create(&comp2).await?;
        let _id3 = repo.create(&comp3).await?;

        // Move the second component up
        repo.move_up(id2).await?;

        // Verify positions after move
        let components = repo.list_by_page_version(version_id).await?;
        assert_eq!(components.len(), 3);

        // Find components by their content to verify positions
        let first_comp = components
            .iter()
            .find(|c| {
                if let Ok(content) = serde_json::from_value::<serde_json::Value>(c.content.clone())
                {
                    content.get("text").and_then(|t| t.as_str()) == Some("First")
                } else {
                    false
                }
            })
            .unwrap();

        let second_comp = components
            .iter()
            .find(|c| {
                if let Ok(content) = serde_json::from_value::<serde_json::Value>(c.content.clone())
                {
                    content.get("text").and_then(|t| t.as_str()) == Some("Second")
                } else {
                    false
                }
            })
            .unwrap();

        assert_eq!(
            second_comp.position, 0,
            "Second component should now be at position 0"
        );
        assert_eq!(
            first_comp.position, 1,
            "First component should now be at position 1"
        );

        // Try to move the first component (now at position 0) up - should do nothing
        repo.move_up(id2).await?;
        let components_after = repo.list_by_page_version(version_id).await?;
        assert_eq!(
            components_after[0].id,
            Some(id2),
            "Component at position 0 should still be id2"
        );

        Ok(())
    }

    #[sqlx::test]
    async fn test_move_down() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        // Setup test data
        sqlx::query("INSERT INTO sites (domain, title) VALUES ('test.com', 'Test')")
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO pages (site_id, slug, title) VALUES (1, 'page', 'Page')")
            .execute(&pool)
            .await?;
        let version_id =
            sqlx::query("INSERT INTO page_versions (page_id, version_number) VALUES (1, 1)")
                .execute(&pool)
                .await?
                .last_insert_rowid();

        let repo = ComponentRepository::new(pool);

        // Create three components
        let comp1 = Component::new(version_id, "text".to_string(), 0, json!({"text": "First"}));
        let comp2 = Component::new(version_id, "text".to_string(), 1, json!({"text": "Second"}));
        let comp3 = Component::new(version_id, "text".to_string(), 2, json!({"text": "Third"}));

        let _id1 = repo.create(&comp1).await?;
        let id2 = repo.create(&comp2).await?;
        let _id3 = repo.create(&comp3).await?;

        // Move the second component down
        repo.move_down(id2).await?;

        // Verify positions after move
        let components = repo.list_by_page_version(version_id).await?;
        assert_eq!(components.len(), 3);

        // Find components by their content to verify positions
        let second_comp = components
            .iter()
            .find(|c| {
                if let Ok(content) = serde_json::from_value::<serde_json::Value>(c.content.clone())
                {
                    content.get("text").and_then(|t| t.as_str()) == Some("Second")
                } else {
                    false
                }
            })
            .unwrap();

        let third_comp = components
            .iter()
            .find(|c| {
                if let Ok(content) = serde_json::from_value::<serde_json::Value>(c.content.clone())
                {
                    content.get("text").and_then(|t| t.as_str()) == Some("Third")
                } else {
                    false
                }
            })
            .unwrap();

        assert_eq!(
            second_comp.position, 2,
            "Second component should now be at position 2"
        );
        assert_eq!(
            third_comp.position, 1,
            "Third component should now be at position 1"
        );

        // Try to move the last component down - should do nothing
        repo.move_down(id2).await?;
        let components_after = repo.list_by_page_version(version_id).await?;
        let last_comp = components_after.iter().find(|c| c.position == 2).unwrap();
        assert_eq!(
            last_comp.id,
            Some(id2),
            "Component at position 2 should still be id2"
        );

        Ok(())
    }

    #[sqlx::test]
    async fn test_move_up_with_two_components() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        // Setup test data
        sqlx::query("INSERT INTO sites (domain, title) VALUES ('test.com', 'Test')")
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO pages (site_id, slug, title) VALUES (1, 'page', 'Page')")
            .execute(&pool)
            .await?;
        let version_id =
            sqlx::query("INSERT INTO page_versions (page_id, version_number) VALUES (1, 1)")
                .execute(&pool)
                .await?
                .last_insert_rowid();

        let repo = ComponentRepository::new(pool);

        // Create only two components (matching the reported issue)
        let comp1 = Component::new(version_id, "text".to_string(), 0, json!({"text": "First"}));
        let comp2 = Component::new(version_id, "text".to_string(), 1, json!({"text": "Second"}));

        let id1 = repo.create(&comp1).await?;
        let id2 = repo.create(&comp2).await?;

        // Initial state verification
        let components_before = repo.list_by_page_version(version_id).await?;
        assert_eq!(components_before.len(), 2);
        assert_eq!(components_before[0].position, 0);
        assert_eq!(components_before[1].position, 1);
        assert_eq!(components_before[0].id, Some(id1));
        assert_eq!(components_before[1].id, Some(id2));

        // Move the second component up
        repo.move_up(id2).await?;

        // Verify positions after move
        let components_after = repo.list_by_page_version(version_id).await?;
        assert_eq!(components_after.len(), 2);

        // The second component should now be first
        assert_eq!(
            components_after[0].id,
            Some(id2),
            "Component 2 should be at position 0"
        );
        assert_eq!(components_after[0].position, 0);

        // The first component should now be second
        assert_eq!(
            components_after[1].id,
            Some(id1),
            "Component 1 should be at position 1"
        );
        assert_eq!(components_after[1].position, 1);

        Ok(())
    }

    #[sqlx::test]
    async fn test_copy_all() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        // Setup test data
        sqlx::query("INSERT INTO sites (domain, title) VALUES ('test.com', 'Test')")
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO pages (site_id, slug, title) VALUES (1, 'page', 'Page')")
            .execute(&pool)
            .await?;

        let version1_id =
            sqlx::query("INSERT INTO page_versions (page_id, version_number) VALUES (1, 1)")
                .execute(&pool)
                .await?
                .last_insert_rowid();

        let version2_id =
            sqlx::query("INSERT INTO page_versions (page_id, version_number) VALUES (1, 2)")
                .execute(&pool)
                .await?
                .last_insert_rowid();

        // Insert components directly using SQL
        sqlx::query(
            "INSERT INTO components (page_version_id, component_type, position, content, title, template)
             VALUES (?, 'text', 0, '{\"text\": \"Component 1\"}', 'Title 1', 'hero')"
        )
        .bind(version1_id)
        .execute(&pool)
        .await?;

        sqlx::query(
            "INSERT INTO components (page_version_id, component_type, position, content, title, template)
             VALUES (?, 'markdown', 1, '{\"text\": \"# Component 2\"}', 'Title 2', 'card')"
        )
        .bind(version1_id)
        .execute(&pool)
        .await?;

        let repo = ComponentRepository::new(pool);

        // Copy components to version 2
        repo.copy_all(version1_id, version2_id).await?;

        // Verify components were copied
        let copied_components = repo.list_by_page_version(version2_id).await?;
        assert_eq!(copied_components.len(), 2);

        // Verify basic properties were preserved
        assert_eq!(copied_components[0].component_type, "text");
        assert_eq!(copied_components[0].title, Some("Title 1".to_string()));
        assert_eq!(copied_components[0].template, "hero");

        assert_eq!(copied_components[1].component_type, "markdown");
        assert_eq!(copied_components[1].title, Some("Title 2".to_string()));
        assert_eq!(copied_components[1].template, "card");

        Ok(())
    }

    #[sqlx::test]
    async fn test_update_content_method(pool: SqlitePool) -> Result<()> {
        setup_test_db(&pool).await?;

        // Setup test data
        sqlx::query("INSERT INTO sites (domain, title) VALUES ('test.com', 'Test')")
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO pages (site_id, slug, title) VALUES (1, 'page', 'Page')")
            .execute(&pool)
            .await?;
        sqlx::query("INSERT INTO page_versions (page_id, version_number) VALUES (1, 1)")
            .execute(&pool)
            .await?;

        let repo = ComponentRepository::new(pool);

        // Create a component
        let comp = Component::new(1, "text".to_string(), 0, json!({"text": "Original text"}));
        let comp_id = repo.create(&comp).await?;

        // Verify initial state
        let original = repo.find_by_id(comp_id).await?.unwrap();
        assert_eq!(original.title, None);
        assert_eq!(original.template, "default");

        // Update content, title and template
        repo.update_content(
            comp_id,
            json!({"text": "Updated text"}),
            Some("Updated Title".to_string()),
            "card".to_string(),
        )
        .await?;

        // Verify update
        let updated = repo.find_by_id(comp_id).await?.unwrap();
        assert_eq!(updated.title, Some("Updated Title".to_string()));
        assert_eq!(updated.template, "card");
        assert_eq!(updated.content["text"], "Updated text");

        // Update again with cleared title
        repo.update_content(
            comp_id,
            json!({"text": "Final text"}),
            None,
            "default".to_string(),
        )
        .await?;

        // Verify changes
        let final_comp = repo.find_by_id(comp_id).await?.unwrap();
        assert!(final_comp.title.is_none());
        assert_eq!(final_comp.template, "default");
        assert_eq!(final_comp.content["text"], "Final text");

        Ok(())
    }
}
