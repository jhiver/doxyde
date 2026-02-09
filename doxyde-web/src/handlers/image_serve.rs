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

use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use doxyde_core::models::{permission::SiteRole, site::Site};
use doxyde_db::repositories::{
    ComponentRepository, PageRepository, PageVersionRepository, SiteUserRepository,
};
use serde::Deserialize;
use sqlx::SqlitePool;
use std::fs;

use crate::{
    auth::CurrentUser, db_middleware::SiteDatabase, site_resolver::SiteContext, state::AppState,
    uploads::resolve_image_path,
};

#[derive(Debug, Deserialize)]
pub struct ImagePreviewQuery {
    pub component_id: i64,
}

#[derive(Debug, Deserialize, Default)]
pub struct ImageServeQuery {
    pub full: Option<u8>,
}

/// Serve an image by slug and format, searching within a specific page
pub async fn serve_image_handler(
    State(state): State<AppState>,
    _site: Site,
    Path((slug, format)): Path<(String, String)>,
    query: Option<Query<ImageServeQuery>>,
    site_ctx: SiteContext,
    SiteDatabase(db): SiteDatabase,
    page_path: String,
) -> Result<Response, StatusCode> {
    let want_full = query
        .as_ref()
        .and_then(|q| q.full)
        .map(|v| v == 1)
        .unwrap_or(false);

    let component_repo = ComponentRepository::new(db.clone());
    let page_version_repo = PageVersionRepository::new(db.clone());
    let page_repo = PageRepository::new(db.clone());

    // Navigate to the specific page from the path
    let page = navigate_to_page_by_path(&page_repo, &page_path)
        .await
        .map_err(|e| {
            tracing::debug!("Page not found for image: {} - {}", page_path, e);
            StatusCode::NOT_FOUND
        })?;

    let page_id = page.id.ok_or_else(|| {
        tracing::error!("Page has no ID");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Get published version for this page
    let version = page_version_repo
        .get_published(page_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to get published version: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let version_id = version.id.ok_or_else(|| {
        tracing::error!("Version has no ID");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Search for the image component in this page version
    let components = component_repo
        .list_by_page_version(version_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list components: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    for component in components {
        if component.component_type != "image" {
            continue;
        }

        let component_slug = match component.content.get("slug").and_then(|s| s.as_str()) {
            Some(s) => s,
            None => continue,
        };

        if component_slug != slug {
            continue;
        }

        let component_format = match component.content.get("format").and_then(|f| f.as_str()) {
            Some(f) => f,
            None => continue,
        };

        if component_format != format {
            continue;
        }

        let file_path = match component.content.get("file_path").and_then(|p| p.as_str()) {
            Some(p) => p,
            None => {
                tracing::warn!(
                    "Image component missing file_path. Slug: {}, ID: {:?}",
                    slug,
                    component.id
                );
                continue;
            }
        };

        // Choose thumbnail or original
        let serve_path = if want_full {
            file_path.to_string()
        } else {
            component
                .content
                .get("thumb_file_path")
                .and_then(|p| p.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| file_path.to_string())
        };

        // Resolve to absolute path using site context
        let resolved = resolve_image_path(&serve_path, &site_ctx.site_directory);

        return serve_image_file(&resolved, &format, state.config.static_files_max_age).await;
    }

    // Image not found in this page
    Err(StatusCode::NOT_FOUND)
}

/// Navigate to a page by its URL path
async fn navigate_to_page_by_path(
    page_repo: &PageRepository,
    path: &str,
) -> Result<doxyde_core::models::page::Page, String> {
    let root = page_repo
        .get_root_page()
        .await
        .map_err(|e| format!("Failed to get root page: {}", e))?
        .ok_or_else(|| "Root page not found".to_string())?;

    if path == "/" {
        return Ok(root);
    }

    let segments: Vec<&str> = path
        .trim_start_matches('/')
        .trim_end_matches('/')
        .split('/')
        .filter(|s| !s.is_empty())
        .collect();

    let mut current = root;
    for segment in segments {
        let current_id = current.id.ok_or_else(|| "Page has no ID".to_string())?;
        let children = page_repo
            .list_children(current_id)
            .await
            .map_err(|e| format!("Failed to list children: {}", e))?;
        current = children
            .into_iter()
            .find(|p| p.slug == segment)
            .ok_or_else(|| format!("Child page not found: {}", segment))?;
    }

    Ok(current)
}

/// Serve an image file from disk
async fn serve_image_file(
    resolved_path: &std::path::Path,
    format: &str,
    max_age: u64,
) -> Result<Response, StatusCode> {
    // Ensure the file exists
    if !resolved_path.exists() {
        tracing::warn!("Image file not found: {:?}", resolved_path);
        return Err(StatusCode::NOT_FOUND);
    }

    // Read the file
    let data = fs::read(resolved_path).map_err(|e| {
        tracing::error!("Failed to read image file: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Determine content type
    let content_type = match format {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    };

    // Build response with appropriate headers
    let cache_control = format!("public, max-age={}", max_age);
    Ok((
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, cache_control.as_str()),
        ],
        data,
    )
        .into_response())
}

/// Serve an image preview for draft components
pub async fn image_preview_handler(
    State(state): State<AppState>,
    site: Site,
    Query(params): Query<ImagePreviewQuery>,
    user: CurrentUser,
    site_ctx: SiteContext,
    SiteDatabase(db): SiteDatabase,
) -> Result<Response, StatusCode> {
    tracing::debug!(
        "Image preview requested - component_id: {}, site_id: {:?}, user_id: {:?}",
        params.component_id,
        site.id,
        user.user.id
    );

    // Get and validate the component
    let component = get_and_validate_component(&db, params.component_id).await?;

    // Check permissions
    check_component_permissions(&db, &site, &component, &user).await?;

    // Extract image data and serve
    let (file_path, format) = extract_image_data(&component)?;

    // Resolve to absolute path using site context
    let resolved = resolve_image_path(file_path, &site_ctx.site_directory);

    tracing::debug!(
        "Serving image preview - file_path: {}, resolved: {:?}, format: {}",
        file_path,
        resolved,
        format,
    );

    serve_image_file(&resolved, format, state.config.static_files_max_age).await
}

/// Get component and validate it's an image type
async fn get_and_validate_component(
    db: &SqlitePool,
    component_id: i64,
) -> Result<doxyde_core::models::component::Component, StatusCode> {
    tracing::debug!("Fetching component with id: {}", component_id);

    let component_repo = ComponentRepository::new(db.clone());
    let component = component_repo
        .find_by_id(component_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to find component {}: {}", component_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or_else(|| {
            tracing::warn!("Component {} not found", component_id);
            StatusCode::NOT_FOUND
        })?;

    tracing::debug!(
        "Component found - type: {}, page_version_id: {}",
        component.component_type,
        component.page_version_id
    );

    // Verify it's an image component
    if component.component_type != "image" {
        tracing::warn!(
            "Component {} is not an image type (type: {})",
            component_id,
            component.component_type
        );
        return Err(StatusCode::BAD_REQUEST);
    }

    Ok(component)
}

/// Check if user has permission to view this component
async fn check_component_permissions(
    db: &SqlitePool,
    site: &Site,
    component: &doxyde_core::models::component::Component,
    user: &CurrentUser,
) -> Result<(), StatusCode> {
    // Get the page version
    let page_version_repo = PageVersionRepository::new(db.clone());
    let page_version = page_version_repo
        .find_by_id(component.page_version_id)
        .await
        .map_err(|e| {
            tracing::error!(
                "Failed to find page version {}: {}",
                component.page_version_id,
                e
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or_else(|| {
            tracing::warn!("Page version {} not found", component.page_version_id);
            StatusCode::NOT_FOUND
        })?;

    tracing::debug!(
        "Page version found - id: {}, page_id: {}",
        page_version.id.unwrap_or(-1),
        page_version.page_id
    );

    // Get the page
    let page_repo = PageRepository::new(db.clone());
    let page = page_repo
        .find_by_id(page_version.page_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to find page {}: {}", page_version.page_id, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or_else(|| {
            tracing::warn!("Page {} not found", page_version.page_id);
            StatusCode::NOT_FOUND
        })?;

    tracing::debug!(
        "Page found - id: {}, current_site_id: {:?}",
        page.id.unwrap_or(-1),
        site.id
    );

    // Get site_id for permission checking
    let site_id = site.id.ok_or_else(|| {
        tracing::error!("Site has no ID");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Check if user can edit the page
    if !user.user.is_admin {
        let user_id = user.user.id.ok_or_else(|| {
            tracing::error!("User has no ID");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        let site_user_repo = SiteUserRepository::new(db.clone());
        let site_user = site_user_repo
            .find_by_site_and_user(site_id, user_id)
            .await
            .map_err(|e| {
                tracing::error!("Failed to check site user permissions: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        let can_edit = if let Some(su) = site_user {
            let has_permission = matches!(su.role, SiteRole::Owner | SiteRole::Editor);
            tracing::debug!(
                "User {} has role {:?}, can_edit: {}",
                user.user.id.unwrap_or(-1),
                su.role,
                has_permission
            );
            has_permission
        } else {
            tracing::debug!("User {} has no site role", user.user.id.unwrap_or(-1));
            false
        };

        if !can_edit {
            tracing::warn!(
                "User {} doesn't have edit permission for site {}",
                user.user.id.unwrap_or(-1),
                site.id.unwrap_or(-1)
            );
            return Err(StatusCode::FORBIDDEN);
        }
    }

    Ok(())
}

/// Extract image file path and format from component content
fn extract_image_data(
    component: &doxyde_core::models::component::Component,
) -> Result<(&str, &str), StatusCode> {
    tracing::debug!("Component content: {:?}", component.content);

    let file_path = component
        .content
        .get("file_path")
        .and_then(|p| p.as_str())
        .ok_or_else(|| {
            tracing::error!(
                "Component {} missing file_path in content",
                component.id.unwrap_or(-1)
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let format = component
        .content
        .get("format")
        .and_then(|f| f.as_str())
        .ok_or_else(|| {
            tracing::error!(
                "Component {} missing format in content",
                component.id.unwrap_or(-1)
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tracing::debug!(
        "Extracted image data - file_path: {}, format: {}",
        file_path,
        format
    );

    Ok((file_path, format))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_serve_query_default() {
        let query: ImageServeQuery = Default::default();
        assert!(query.full.is_none());
    }

    #[test]
    fn test_image_serve_query_parse_full() {
        let query: ImageServeQuery = serde_urlencoded::from_str("full=1").unwrap();
        assert_eq!(query.full, Some(1));
    }

    #[test]
    fn test_image_serve_query_parse_empty() {
        let query: ImageServeQuery = serde_urlencoded::from_str("").unwrap();
        assert!(query.full.is_none());
    }
}
