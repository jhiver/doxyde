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
use std::path::PathBuf;

use crate::{
    auth::CurrentUser, db_middleware::SiteDatabase, path_security::validate_safe_path,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct ImagePreviewQuery {
    pub component_id: i64,
}

#[derive(Debug, Deserialize, Default)]
pub struct ImageServeQuery {
    pub full: Option<u8>,
}

/// Serve an image by slug and format
pub async fn serve_image_handler(
    State(state): State<AppState>,
    _site: Site,
    Path((slug, format)): Path<(String, String)>,
    query: Option<Query<ImageServeQuery>>,
    SiteDatabase(db): SiteDatabase,
) -> Result<Response, StatusCode> {
    let want_full = query
        .as_ref()
        .and_then(|q| q.full)
        .map(|v| v == 1)
        .unwrap_or(false);
    // Search for an image component with this slug
    let component_repo = ComponentRepository::new(db.clone());
    let page_version_repo = PageVersionRepository::new(db.clone());

    // Find all published page versions (per-site DB, no site_id filter needed)
    let published_versions = page_version_repo.find_all_published().await.map_err(|e| {
        tracing::error!("Failed to find published versions: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Search through all published versions for an image with this slug
    for version in published_versions {
        let version_id = version.id.ok_or_else(|| {
            tracing::error!("Version has no ID");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

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

            // Check if this component has the requested slug
            if let Some(component_slug) = component.content.get("slug").and_then(|s| s.as_str()) {
                if component_slug == slug {
                    // Check format matches
                    if let Some(component_format) =
                        component.content.get("format").and_then(|f| f.as_str())
                    {
                        if component_format != format {
                            continue; // Format doesn't match
                        }

                        // Get file path - prefer thumbnail unless full requested
                        if let Some(file_path) =
                            component.content.get("file_path").and_then(|p| p.as_str())
                        {
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

                            let uploads_dir = PathBuf::from(&state.config.uploads_dir);
                            let base_dir =
                                if std::path::Path::new(serve_path.as_str()).is_absolute() {
                                    uploads_dir
                                        .parent()
                                        .map(|p| p.to_path_buf())
                                        .unwrap_or_else(|| PathBuf::from("/"))
                                } else {
                                    state
                                        .config
                                        .get_sites_directory()
                                        .map(|p| p.parent().map(|p| p.to_path_buf()).unwrap_or(p))
                                        .unwrap_or_else(|_| PathBuf::from("."))
                                };

                            return serve_image_file(
                                &serve_path,
                                &format,
                                base_dir.to_str().unwrap_or("."),
                                state.config.static_files_max_age,
                            )
                            .await;
                        } else {
                            // Log missing file_path for debugging
                            tracing::warn!(
                                "Image component found but missing file_path. Slug: {}, Component ID: {:?}",
                                slug,
                                component.id
                            );
                        }
                    }
                }
            }
        }
    }

    // Image not found
    Err(StatusCode::NOT_FOUND)
}

/// Serve an image file from disk with path validation
async fn serve_image_file(
    file_path: &str,
    format: &str,
    uploads_dir: &str,
    max_age: u64,
) -> Result<Response, StatusCode> {
    // Validate the path is safe and within uploads directory
    let uploads_base = PathBuf::from(uploads_dir);
    let safe_path = validate_safe_path(file_path, &uploads_base).map_err(|e| {
        tracing::warn!("Path validation failed for image {}: {}", file_path, e);
        StatusCode::FORBIDDEN
    })?;

    // Ensure the file exists
    if !safe_path.exists() {
        tracing::warn!("Image file not found: {}", file_path);
        return Err(StatusCode::NOT_FOUND);
    }

    // Read the file
    let data = fs::read(&safe_path).map_err(|e| {
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

    // file_path can be absolute (from uploads_dir config) or relative
    // Use the uploads directory as the base for path validation
    let uploads_dir = PathBuf::from(&state.config.uploads_dir);
    let base_dir = if std::path::Path::new(file_path).is_absolute() {
        // For absolute paths, use the uploads directory or its parent
        uploads_dir
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("/"))
    } else {
        // For relative paths, use the sites directory parent
        state
            .config
            .get_sites_directory()
            .map(|p| p.parent().map(|p| p.to_path_buf()).unwrap_or(p))
            .unwrap_or_else(|_| PathBuf::from("."))
    };

    tracing::debug!(
        "Serving image preview - file_path: {}, format: {}, base_dir: {:?}",
        file_path,
        format,
        base_dir
    );

    serve_image_file(
        file_path,
        format,
        base_dir.to_str().unwrap_or("."),
        state.config.static_files_max_age,
    )
    .await
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
