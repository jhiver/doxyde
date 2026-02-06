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
use doxyde_db::repositories::{ComponentRepository, PageVersionRepository};
use sqlx::SqlitePool;
use std::collections::HashSet;

#[derive(Debug)]
pub struct CleanupResult {
    pub page_id: i64,
    pub published_version_id: i64,
    pub old_versions_deleted: u64,
    pub files_deleted: u64,
}

/// Publish a draft and clean up orphaned files and old versions
pub async fn publish_and_cleanup(
    pool: &SqlitePool,
    page_id: i64,
) -> Result<CleanupResult> {
    let version_repo = PageVersionRepository::new(pool.clone());
    let component_repo = ComponentRepository::new(pool.clone());

    // Get the draft to publish
    let draft = version_repo
        .get_draft(page_id)
        .await?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No draft version exists for this page. Use get_or_create_draft first."
            )
        })?;
    let draft_id = draft
        .id
        .ok_or_else(|| anyhow::anyhow!("Draft has no ID"))?;

    // Step 1: Collect image files from old published version
    let old_files = collect_published_image_paths(
        &version_repo,
        &component_repo,
        page_id,
    )
    .await?;

    // Step 2: Collect image files from the draft
    let new_files = component_repo
        .collect_image_paths_for_version(draft_id)
        .await?;
    let new_file_set: HashSet<&str> = new_files
        .iter()
        .map(|(fp, _)| fp.as_str())
        .collect();

    // Step 3: Compute removed files (in old but not in new)
    let removed_files = compute_removed_files(&old_files, &new_file_set);

    // Step 4: Publish (unpublish old, publish new)
    publish_draft_with_unpublish(&version_repo, page_id, draft_id).await?;

    // Step 5: Delete old versions from DB
    let old_versions_deleted = delete_old_versions(
        &version_repo,
        page_id,
        draft_id,
    )
    .await?;

    // Step 6-7: Delete orphaned files from disk
    let files_deleted = delete_orphaned_files(
        &component_repo,
        &removed_files,
    )
    .await?;

    Ok(CleanupResult {
        page_id,
        published_version_id: draft_id,
        old_versions_deleted,
        files_deleted,
    })
}

/// Collect image file paths from the current published version
async fn collect_published_image_paths(
    version_repo: &PageVersionRepository,
    component_repo: &ComponentRepository,
    page_id: i64,
) -> Result<Vec<(String, Option<String>)>> {
    let published = version_repo.get_published(page_id).await?;
    match published {
        Some(version) => {
            let version_id = version
                .id
                .ok_or_else(|| anyhow::anyhow!("Published version has no ID"))?;
            component_repo
                .collect_image_paths_for_version(version_id)
                .await
        }
        None => Ok(Vec::new()),
    }
}

/// Compute which files were removed (present in old, absent in new)
fn compute_removed_files<'a>(
    old_files: &'a [(String, Option<String>)],
    new_file_set: &HashSet<&str>,
) -> Vec<&'a (String, Option<String>)> {
    old_files
        .iter()
        .filter(|(fp, _)| !new_file_set.contains(fp.as_str()))
        .collect()
}

/// Unpublish the current published version and publish the draft
async fn publish_draft_with_unpublish(
    version_repo: &PageVersionRepository,
    page_id: i64,
    draft_id: i64,
) -> Result<()> {
    // Unpublish current published version if any
    if let Some(current) = version_repo.get_published(page_id).await? {
        let current_id = current
            .id
            .ok_or_else(|| anyhow::anyhow!("Published version has no ID"))?;
        version_repo.unpublish(current_id).await?;
    }

    // Publish the draft
    version_repo.publish(draft_id).await?;

    Ok(())
}

/// Delete all old versions (everything except the newly published one)
async fn delete_old_versions(
    version_repo: &PageVersionRepository,
    page_id: i64,
    keep_version_id: i64,
) -> Result<u64> {
    let old_versions = version_repo
        .list_old_versions(page_id, keep_version_id)
        .await?;

    if old_versions.is_empty() {
        return Ok(0);
    }

    let ids: Vec<i64> = old_versions
        .iter()
        .filter_map(|v| v.id)
        .collect();

    version_repo.delete_versions(&ids).await
}

/// Check each removed file; delete from disk if no longer referenced
async fn delete_orphaned_files(
    component_repo: &ComponentRepository,
    removed_files: &[&(String, Option<String>)],
) -> Result<u64> {
    let mut deleted = 0u64;

    for (file_path, thumb_path) in removed_files.iter().copied() {
        let refs = component_repo
            .count_references_to_file(file_path)
            .await
            .context("Failed to count references")?;

        if refs > 0 {
            tracing::debug!(
                file_path,
                refs,
                "Skipping file still referenced by other components"
            );
            continue;
        }

        // Delete the main file
        if try_delete_file(file_path) {
            deleted += 1;
        }

        // Delete the thumbnail if present
        if let Some(thumb) = thumb_path {
            try_delete_file(thumb);
        }
    }

    Ok(deleted)
}

/// Try to delete a file, logging on failure. Returns true if deleted.
fn try_delete_file(path: &str) -> bool {
    match std::fs::remove_file(path) {
        Ok(()) => {
            tracing::info!(path, "Deleted orphaned file");
            true
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            tracing::debug!(path, "Orphaned file already missing");
            false
        }
        Err(e) => {
            tracing::warn!(path, error = %e, "Failed to delete orphaned file");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use doxyde_core::models::component::Component;
    use doxyde_core::models::version::PageVersion;
    use serde_json::json;
    use std::io::Write;

    async fn setup_test_db(pool: &SqlitePool) -> Result<i64> {
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

        sqlx::query("INSERT INTO sites (domain, title) VALUES ('test.com', 'Test')")
            .execute(pool)
            .await?;

        let page_id =
            sqlx::query("INSERT INTO pages (site_id, slug, title) VALUES (1, 'page', 'Page')")
                .execute(pool)
                .await?
                .last_insert_rowid();

        Ok(page_id)
    }

    #[sqlx::test]
    async fn test_publish_and_cleanup_basic() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let page_id = setup_test_db(&pool).await?;
        let version_repo = PageVersionRepository::new(pool.clone());
        let component_repo = ComponentRepository::new(pool.clone());

        // Create and publish v1 with a text component
        let v1 = PageVersion::new(page_id, 1, None);
        let v1_id = version_repo.create(&v1).await?;
        let text_comp =
            Component::new(v1_id, "text".to_string(), 0, json!({"text": "Hello"}));
        component_repo.create(&text_comp).await?;
        version_repo.publish(v1_id).await?;

        // Create draft v2
        let v2 = PageVersion::new(page_id, 2, None);
        let v2_id = version_repo.create(&v2).await?;
        let text_comp2 =
            Component::new(v2_id, "text".to_string(), 0, json!({"text": "Updated"}));
        component_repo.create(&text_comp2).await?;

        // Publish with cleanup
        let result = publish_and_cleanup(&pool, page_id).await?;

        assert_eq!(result.page_id, page_id);
        assert_eq!(result.published_version_id, v2_id);
        assert_eq!(result.old_versions_deleted, 1); // v1 deleted
        assert_eq!(result.files_deleted, 0); // no images

        // Verify v2 is published
        let published = version_repo.get_published(page_id).await?;
        assert_eq!(published.map(|v| v.id), Some(Some(v2_id)));

        // Verify v1 is gone
        let v1_found = version_repo.find_by_id(v1_id).await?;
        assert!(v1_found.is_none());

        Ok(())
    }

    #[sqlx::test]
    async fn test_publish_and_cleanup_deletes_orphaned_files() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let page_id = setup_test_db(&pool).await?;
        let version_repo = PageVersionRepository::new(pool.clone());
        let component_repo = ComponentRepository::new(pool.clone());

        // Create temp files to simulate uploaded images
        let tmp_dir = tempfile::tempdir()?;
        let img_path = tmp_dir.path().join("image1.jpg");
        let thumb_path = tmp_dir.path().join("image1_thumb.jpg");
        let kept_path = tmp_dir.path().join("image2.jpg");

        let mut f = std::fs::File::create(&img_path)?;
        f.write_all(b"fake image data")?;
        let mut f = std::fs::File::create(&thumb_path)?;
        f.write_all(b"fake thumb data")?;
        let mut f = std::fs::File::create(&kept_path)?;
        f.write_all(b"kept image data")?;

        let img_path_str = img_path.to_string_lossy().to_string();
        let thumb_path_str = thumb_path.to_string_lossy().to_string();
        let kept_path_str = kept_path.to_string_lossy().to_string();

        // Create and publish v1 with two image components
        let v1 = PageVersion::new(page_id, 1, None);
        let v1_id = version_repo.create(&v1).await?;

        let img1 = Component::new(
            v1_id,
            "image".to_string(),
            0,
            json!({
                "slug": "img1",
                "format": "jpg",
                "file_path": img_path_str,
                "thumb_file_path": thumb_path_str
            }),
        );
        component_repo.create(&img1).await?;

        let img2 = Component::new(
            v1_id,
            "image".to_string(),
            1,
            json!({
                "slug": "img2",
                "format": "jpg",
                "file_path": kept_path_str
            }),
        );
        component_repo.create(&img2).await?;
        version_repo.publish(v1_id).await?;

        // Create draft v2 - keep img2 but remove img1
        let v2 = PageVersion::new(page_id, 2, None);
        let v2_id = version_repo.create(&v2).await?;

        let img2_copy = Component::new(
            v2_id,
            "image".to_string(),
            0,
            json!({
                "slug": "img2",
                "format": "jpg",
                "file_path": kept_path_str
            }),
        );
        component_repo.create(&img2_copy).await?;

        // Publish with cleanup
        let result = publish_and_cleanup(&pool, page_id).await?;

        assert_eq!(result.files_deleted, 1); // img1 deleted
        assert_eq!(result.old_versions_deleted, 1); // v1 deleted

        // img1 and its thumbnail should be deleted
        assert!(!img_path.exists());
        assert!(!thumb_path.exists());

        // img2 should still exist
        assert!(kept_path.exists());

        Ok(())
    }

    #[sqlx::test]
    async fn test_publish_and_cleanup_no_prior_published() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let page_id = setup_test_db(&pool).await?;
        let version_repo = PageVersionRepository::new(pool.clone());
        let component_repo = ComponentRepository::new(pool.clone());

        // Create draft v1 directly (no prior published version)
        let v1 = PageVersion::new(page_id, 1, None);
        let v1_id = version_repo.create(&v1).await?;
        let text = Component::new(v1_id, "text".to_string(), 0, json!({"text": "First"}));
        component_repo.create(&text).await?;

        let result = publish_and_cleanup(&pool, page_id).await?;

        assert_eq!(result.old_versions_deleted, 0);
        assert_eq!(result.files_deleted, 0);
        assert_eq!(result.published_version_id, v1_id);

        Ok(())
    }

    #[sqlx::test]
    async fn test_publish_and_cleanup_shared_file_not_deleted() -> Result<()> {
        let pool = SqlitePool::connect(":memory:").await?;
        let page_id = setup_test_db(&pool).await?;
        let version_repo = PageVersionRepository::new(pool.clone());
        let component_repo = ComponentRepository::new(pool.clone());

        let tmp_dir = tempfile::tempdir()?;
        let shared_path = tmp_dir.path().join("shared.jpg");
        let mut f = std::fs::File::create(&shared_path)?;
        f.write_all(b"shared image")?;
        let shared_str = shared_path.to_string_lossy().to_string();

        // Create a second page that also uses this image
        let page2_id = sqlx::query(
            "INSERT INTO pages (site_id, slug, title) VALUES (1, 'page2', 'Page 2')",
        )
        .execute(&pool)
        .await?
        .last_insert_rowid();

        let other_v = PageVersion::new(page2_id, 1, None);
        let other_vid = version_repo.create(&other_v).await?;
        let other_img = Component::new(
            other_vid,
            "image".to_string(),
            0,
            json!({"slug": "shared", "format": "jpg", "file_path": shared_str}),
        );
        component_repo.create(&other_img).await?;
        version_repo.publish(other_vid).await?;

        // page1: publish v1 with the shared image
        let v1 = PageVersion::new(page_id, 1, None);
        let v1_id = version_repo.create(&v1).await?;
        let img = Component::new(
            v1_id,
            "image".to_string(),
            0,
            json!({"slug": "shared", "format": "jpg", "file_path": shared_str}),
        );
        component_repo.create(&img).await?;
        version_repo.publish(v1_id).await?;

        // page1: create draft v2 WITHOUT the image
        let v2 = PageVersion::new(page_id, 2, None);
        let v2_id = version_repo.create(&v2).await?;
        let text = Component::new(v2_id, "text".to_string(), 0, json!({"text": "No image"}));
        component_repo.create(&text).await?;

        let result = publish_and_cleanup(&pool, page_id).await?;

        // File should NOT be deleted because page2 still references it
        assert_eq!(result.files_deleted, 0);
        assert!(shared_path.exists());

        Ok(())
    }

    #[test]
    fn test_compute_removed_files() {
        let old_files = vec![
            ("/a.jpg".to_string(), None),
            ("/b.jpg".to_string(), Some("/b_thumb.jpg".to_string())),
            ("/c.jpg".to_string(), None),
        ];
        let new_set: HashSet<&str> = ["/a.jpg", "/c.jpg"].into_iter().collect();

        let removed = compute_removed_files(&old_files, &new_set);
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].0, "/b.jpg");
        assert_eq!(removed[0].1, Some("/b_thumb.jpg".to_string()));
    }

    #[test]
    fn test_compute_removed_files_all_kept() {
        let old_files = vec![("/a.jpg".to_string(), None)];
        let new_set: HashSet<&str> = ["/a.jpg"].into_iter().collect();

        let removed = compute_removed_files(&old_files, &new_set);
        assert!(removed.is_empty());
    }

    #[test]
    fn test_try_delete_file_nonexistent() {
        assert!(!try_delete_file("/nonexistent/path/file.jpg"));
    }

    #[test]
    fn test_try_delete_file_exists() {
        let tmp = tempfile::NamedTempFile::new().ok();
        if let Some(tmp) = tmp {
            let _path = tmp.path().to_string_lossy().to_string();
            // Keep the file alive by leaking the temp handle
            let (_, persisted) = tmp.keep().ok().unzip();
            if let Some(persisted) = persisted {
                let path = persisted.to_string_lossy().to_string();
                assert!(try_delete_file(&path));
                assert!(!std::path::Path::new(&path).exists());
            }
        }
    }
}
