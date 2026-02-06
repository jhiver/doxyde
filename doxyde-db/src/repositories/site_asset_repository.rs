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
use doxyde_core::models::site_asset::{SiteAsset, SiteAssetMeta};
use sqlx::SqlitePool;

pub struct SiteAssetRepository {
    pool: SqlitePool,
}

impl SiteAssetRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, asset: &SiteAsset) -> Result<i64> {
        let result = sqlx::query(
            r#"
            INSERT INTO site_assets (path, content, mime_type, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&asset.path)
        .bind(&asset.content)
        .bind(&asset.mime_type)
        .bind(asset.created_at)
        .bind(asset.updated_at)
        .execute(&self.pool)
        .await
        .context("Failed to create site asset")?;

        Ok(result.last_insert_rowid())
    }

    pub async fn find_by_id(&self, id: i64) -> Result<Option<SiteAsset>> {
        let row = sqlx::query_as::<_, (i64, String, Vec<u8>, String, String, String)>(
            r#"
            SELECT id, path, content, mime_type, created_at, updated_at
            FROM site_assets
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to find site asset by ID")?;

        row.map(|r| row_to_asset(r)).transpose()
    }

    pub async fn find_by_path(&self, path: &str) -> Result<Option<SiteAsset>> {
        let row = sqlx::query_as::<_, (i64, String, Vec<u8>, String, String, String)>(
            r#"
            SELECT id, path, content, mime_type, created_at, updated_at
            FROM site_assets
            WHERE path = ?
            "#,
        )
        .bind(path)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to find site asset by path")?;

        row.map(|r| row_to_asset(r)).transpose()
    }

    pub async fn list_all(&self) -> Result<Vec<SiteAssetMeta>> {
        let rows = sqlx::query_as::<_, (i64, String, String, i64, String, String)>(
            r#"
            SELECT id, path, mime_type, length(content) as content_length,
                   created_at, updated_at
            FROM site_assets
            ORDER BY path
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to list site assets")?;

        rows.into_iter().map(|r| row_to_asset_meta(r)).collect()
    }

    pub async fn update(&self, asset: &SiteAsset) -> Result<()> {
        let id = asset.id.ok_or_else(|| anyhow::anyhow!("Asset has no ID"))?;

        let rows = sqlx::query(
            r#"
            UPDATE site_assets
            SET path = ?, content = ?, mime_type = ?, updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&asset.path)
        .bind(&asset.content)
        .bind(&asset.mime_type)
        .bind(chrono::Utc::now())
        .bind(id)
        .execute(&self.pool)
        .await
        .context("Failed to update site asset")?
        .rows_affected();

        if rows == 0 {
            return Err(anyhow::anyhow!("Site asset not found"));
        }
        Ok(())
    }

    pub async fn delete(&self, id: i64) -> Result<()> {
        let rows = sqlx::query("DELETE FROM site_assets WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete site asset")?
            .rows_affected();

        if rows == 0 {
            return Err(anyhow::anyhow!("Site asset not found"));
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

fn row_to_asset(row: (i64, String, Vec<u8>, String, String, String)) -> Result<SiteAsset> {
    let (id, path, content, mime_type, created_at_str, updated_at_str) = row;
    Ok(SiteAsset {
        id: Some(id),
        path,
        content,
        mime_type,
        created_at: parse_datetime(&created_at_str)?,
        updated_at: parse_datetime(&updated_at_str)?,
    })
}

fn row_to_asset_meta(row: (i64, String, String, i64, String, String)) -> Result<SiteAssetMeta> {
    let (id, path, mime_type, content_length, created_at_str, updated_at_str) = row;
    Ok(SiteAssetMeta {
        id,
        path,
        mime_type,
        content_length,
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
            CREATE TABLE IF NOT EXISTS site_assets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL UNIQUE,
                content BLOB NOT NULL,
                mime_type TEXT NOT NULL,
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

        let repo = SiteAssetRepository::new(pool);
        let a = SiteAsset::new(
            "js/custom.js".to_string(),
            b"console.log('hi');".to_vec(),
            "text/javascript".to_string(),
        );
        let id = repo.create(&a).await?;
        assert!(id > 0);

        let found = repo.find_by_id(id).await?;
        assert!(found.is_some());
        let found = found.unwrap();
        assert_eq!(found.path, "js/custom.js");
        assert_eq!(found.content, b"console.log('hi');");
        assert_eq!(found.mime_type, "text/javascript");

        Ok(())
    }

    #[sqlx::test]
    async fn test_find_by_path() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteAssetRepository::new(pool);
        let a = SiteAsset::new(
            "fonts/brand.woff2".to_string(),
            vec![0, 1, 2, 3],
            "font/woff2".to_string(),
        );
        repo.create(&a).await?;

        assert!(repo.find_by_path("fonts/brand.woff2").await?.is_some());
        assert!(repo.find_by_path("nonexistent.js").await?.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_list_all_returns_meta() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteAssetRepository::new(pool);
        let a = SiteAsset::new(
            "js/app.js".to_string(),
            b"var x = 1;".to_vec(),
            "text/javascript".to_string(),
        );
        repo.create(&a).await?;

        let all = repo.list_all().await?;
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].path, "js/app.js");
        assert_eq!(all[0].content_length, 10); // "var x = 1;" is 10 bytes

        Ok(())
    }

    #[sqlx::test]
    async fn test_update() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteAssetRepository::new(pool);
        let a = SiteAsset::new(
            "js/app.js".to_string(),
            b"old".to_vec(),
            "text/javascript".to_string(),
        );
        let id = repo.create(&a).await?;

        let mut found = repo.find_by_id(id).await?.unwrap();
        found.content = b"new content".to_vec();
        repo.update(&found).await?;

        let updated = repo.find_by_id(id).await?.unwrap();
        assert_eq!(updated.content, b"new content");

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteAssetRepository::new(pool);
        let a = SiteAsset::new(
            "js/app.js".to_string(),
            b"content".to_vec(),
            "text/javascript".to_string(),
        );
        let id = repo.create(&a).await?;

        repo.delete(id).await?;
        assert!(repo.find_by_id(id).await?.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_delete_nonexistent_fails() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        setup_test_db(&pool).await?;

        let repo = SiteAssetRepository::new(pool);
        assert!(repo.delete(999).await.is_err());

        Ok(())
    }
}
