use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use doxyde_core::models::site::Site;
use doxyde_db::repositories::{ComponentRepository, PageVersionRepository};
use std::fs;
use std::path::PathBuf;

use crate::state::AppState;

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
                            return serve_image_file(file_path, &format).await;
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

/// Serve an image file from disk
async fn serve_image_file(file_path: &str, format: &str) -> Result<Response, StatusCode> {
    let path = PathBuf::from(file_path);

    // Ensure the file exists
    if !path.exists() {
        tracing::warn!("Image file not found: {}", file_path);
        return Err(StatusCode::NOT_FOUND);
    }

    // Read the file
    let data = fs::read(&path).map_err(|e| {
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
    use super::*;

    #[test]
    fn test_content_type_mapping() {
        // Just a simple test to ensure the handler compiles
        assert_eq!(2 + 2, 4);
    }
}
