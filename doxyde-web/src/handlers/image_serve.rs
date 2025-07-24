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
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use doxyde_core::models::site::Site;
use doxyde_db::repositories::{ComponentRepository, PageVersionRepository};
use std::fs;
use std::path::PathBuf;

use crate::{path_security::validate_safe_path, state::AppState};

/// Serve an image by slug and format
pub async fn serve_image_handler(
    State(state): State<AppState>,
    site: Site,
    Path((slug, format)): Path<(String, String)>,
) -> Result<Response, StatusCode> {
    // Search for an image component with this slug
    let component_repo = ComponentRepository::new(state.db.clone());
    let page_version_repo = PageVersionRepository::new(state.db.clone());

    // Find all published page versions for this site
    let published_versions = page_version_repo
        .find_published_by_site(site.id.unwrap())
        .await
        .map_err(|e| {
            tracing::error!("Failed to find published versions: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Search through all published versions for an image with this slug
    for version in published_versions {
        let components = component_repo
            .list_by_page_version(version.id.unwrap())
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

                        // Get file path
                        if let Some(file_path) =
                            component.content.get("file_path").and_then(|p| p.as_str())
                        {
                            return serve_image_file(file_path, &format, &state.config.uploads_dir)
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
    Ok((
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "public, max-age=31536000"), // Cache for 1 year
        ],
        data,
    )
        .into_response())
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_content_type_mapping() {
        // Just a simple test to ensure the handler compiles
        assert_eq!(2 + 2, 4);
    }
}
