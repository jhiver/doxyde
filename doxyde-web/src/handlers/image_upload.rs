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

use axum::extract::Multipart;
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json, Redirect, Response},
};
use chrono::Utc;
use doxyde_core::models::{component::Component, site::Site};
use doxyde_db::repositories::ComponentRepository;
use serde::Deserialize;
use serde_json::json;
use std::path::PathBuf;

use crate::{
    auth::CurrentUser,
    draft::get_or_create_draft,
    state::AppState,
    uploads::{
        create_upload_directory, extract_image_metadata, generate_unique_filename, sanitize_slug,
        save_upload,
    },
};

use doxyde_core::models::permission::SiteRole;
use doxyde_db::repositories::{PageRepository, SiteUserRepository};

/// Check if user can edit the page
async fn can_edit_page(
    state: &AppState,
    site: &Site,
    user: &CurrentUser,
) -> Result<bool, StatusCode> {
    // Admins can always edit
    if user.user.is_admin {
        return Ok(true);
    }

    // Check site permissions
    let site_user_repo = SiteUserRepository::new(state.db.clone());
    let site_user = site_user_repo
        .find_by_site_and_user(site.id.unwrap(), user.user.id.unwrap())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(su) = site_user {
        Ok(matches!(su.role, SiteRole::Owner | SiteRole::Editor))
    } else {
        Ok(false)
    }
}

/// Build the full path to a page
async fn build_page_path(
    state: &AppState,
    page: &doxyde_core::models::page::Page,
) -> Result<String, StatusCode> {
    if page.parent_page_id.is_none() {
        return Ok("/".to_string());
    }

    let page_repo = PageRepository::new(state.db.clone());
    let breadcrumb = page_repo
        .get_breadcrumb_trail(page.id.unwrap())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let path = breadcrumb
        .iter()
        .skip(1) // Skip the root page
        .map(|p| p.slug.as_str())
        .collect::<Vec<_>>()
        .join("/");

    Ok(format!("/{}", path))
}

#[derive(Debug, Deserialize)]
pub struct ImageUploadForm {
    pub slug: String,
    pub title: Option<String>,
    pub description: Option<String>,
}

/// Handle image upload via multipart form
pub async fn upload_image_handler(
    State(state): State<AppState>,
    site: Site,
    page: doxyde_core::models::page::Page,
    user: CurrentUser,
    mut multipart: Multipart,
) -> Result<Response, StatusCode> {
    // Check permissions
    if !can_edit_page(&state, &site, &user).await? {
        return Err(StatusCode::FORBIDDEN);
    }

    let mut image_data: Option<Vec<u8>> = None;
    let mut original_filename: Option<String> = None;
    let mut form_data = ImageUploadForm {
        slug: String::new(),
        title: None,
        description: None,
    };

    // Process multipart form data
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
    {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "image" => {
                // Get filename
                original_filename = field.file_name().map(|f| f.to_string());

                // Read file data
                let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;

                // Check file size
                if data.len() > state.config.max_upload_size {
                    return Err(StatusCode::PAYLOAD_TOO_LARGE);
                }

                image_data = Some(data.to_vec());
            }
            "slug" => {
                form_data.slug = field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?;
            }
            "title" => {
                let text = field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?;
                if !text.is_empty() {
                    form_data.title = Some(text);
                }
            }
            "description" => {
                let text = field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?;
                if !text.is_empty() {
                    form_data.description = Some(text);
                }
            }
            _ => {} // Ignore unknown fields
        }
    }

    // Validate we have image data
    let image_data = image_data.ok_or(StatusCode::BAD_REQUEST)?;
    let original_filename = original_filename.ok_or(StatusCode::BAD_REQUEST)?;

    // Validate and sanitize slug
    if form_data.slug.is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let slug = sanitize_slug(&form_data.slug);

    // Extract image metadata
    let metadata =
        extract_image_metadata(&image_data).map_err(|_| StatusCode::UNSUPPORTED_MEDIA_TYPE)?;

    // Create upload directory
    let now = Utc::now();
    let upload_base = PathBuf::from(&state.config.uploads_dir);
    let upload_dir = create_upload_directory(&upload_base, now)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Generate unique filename
    let filename = generate_unique_filename(&original_filename);

    // Save file to disk
    let file_path = save_upload(&image_data, &upload_dir, &filename)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get or create draft version
    let draft_version = get_or_create_draft(
        &state.db,
        page.id.unwrap(),
        Some(user.user.username.clone()),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get current components to determine position
    let component_repo = ComponentRepository::new(state.db.clone());
    let components = component_repo
        .list_by_page_version(draft_version.id.unwrap())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let next_position = components.len() as i32;

    // Create component content
    let content = json!({
        "slug": slug,
        "title": form_data.title.clone().unwrap_or_else(|| slug.clone()),
        "description": form_data.description.unwrap_or_default(),
        "format": metadata.format.extension(),
        "file_path": file_path.to_string_lossy(),
        "original_name": original_filename,
        "mime_type": metadata.format.mime_type(),
        "size": metadata.size,
        "width": metadata.width,
        "height": metadata.height,
    });

    // Create new component
    let mut component = Component::new(
        draft_version.id.unwrap(),
        "image".to_string(),
        next_position,
        content,
    );

    // Set title if provided
    component.title = form_data.title;

    component_repo
        .create(&component)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Redirect back to edit page
    let redirect_path = build_page_path(&state, &page).await?;
    let edit_path = if redirect_path == "/" {
        "/.edit".to_string()
    } else {
        format!("{}/.edit", redirect_path)
    };
    Ok(Redirect::to(&edit_path).into_response())
}

/// Handle AJAX image upload (returns JSON response)
pub async fn upload_image_ajax_handler(
    State(state): State<AppState>,
    site: Site,
    _page: doxyde_core::models::page::Page,
    user: CurrentUser,
    mut multipart: Multipart,
) -> Result<Response, StatusCode> {
    // Check permissions
    if !can_edit_page(&state, &site, &user).await? {
        return Err(StatusCode::FORBIDDEN);
    }

    let mut image_data: Option<Vec<u8>> = None;
    let mut original_filename: Option<String> = None;

    // Process multipart form data (only looking for image field)
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
    {
        if field.name().unwrap_or("") == "image" {
            // Get filename
            original_filename = field.file_name().map(|f| f.to_string());

            // Read file data
            let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;

            // Check file size
            if data.len() > state.config.max_upload_size {
                return Ok(Json(json!({
                    "error": "File too large"
                }))
                .into_response());
            }

            image_data = Some(data.to_vec());
            break;
        }
    }

    // Validate we have image data
    let image_data = image_data.ok_or(StatusCode::BAD_REQUEST)?;
    let original_filename = original_filename.ok_or(StatusCode::BAD_REQUEST)?;

    // Extract image metadata
    let metadata = match extract_image_metadata(&image_data) {
        Ok(m) => m,
        Err(_) => {
            return Ok(Json(json!({
                "error": "Invalid image format"
            }))
            .into_response());
        }
    };

    // Create upload directory
    let now = Utc::now();
    let upload_base = PathBuf::from(&state.config.uploads_dir);
    let upload_dir = create_upload_directory(&upload_base, now)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Generate unique filename
    let filename = generate_unique_filename(&original_filename);

    // Save file to disk
    let file_path = save_upload(&image_data, &upload_dir, &filename)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Generate a suggested slug from the filename
    let suggested_slug = sanitize_slug(
        &PathBuf::from(&original_filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("image"),
    );

    // Return upload info as JSON
    Ok(Json(json!({
        "success": true,
        "file_path": file_path.to_string_lossy(),
        "original_name": original_filename,
        "suggested_slug": suggested_slug,
        "format": metadata.format.extension(),
        "mime_type": metadata.format.mime_type(),
        "size": metadata.size,
        "width": metadata.width,
        "height": metadata.height,
    }))
    .into_response())
}

/// Handle component image upload - uploads image and updates component in one go
pub async fn upload_component_image_handler(
    State(state): State<AppState>,
    site: Site,
    page: doxyde_core::models::page::Page,
    user: CurrentUser,
    mut multipart: Multipart,
) -> Result<Response, StatusCode> {
    tracing::debug!("Starting upload_component_image_handler");

    // Check permissions
    if !can_edit_page(&state, &site, &user).await? {
        tracing::warn!(
            "User {} does not have permission to edit page",
            user.user.username
        );
        return Err(StatusCode::FORBIDDEN);
    }

    let mut image_data: Option<Vec<u8>> = None;
    let mut original_filename: Option<String> = None;
    let mut component_id: Option<i64> = None;
    let mut slug: Option<String> = None;
    let mut title: Option<String> = None;
    let mut description: Option<String> = None;

    // Process multipart form data
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        tracing::error!("Failed to get next field from multipart: {:?}", e);
        StatusCode::BAD_REQUEST
    })? {
        let name = field.name().unwrap_or("").to_string();
        tracing::debug!("Processing multipart field: {}", name);

        match name.as_str() {
            "image" => {
                // Get filename
                original_filename = field.file_name().map(|f| f.to_string());
                tracing::debug!("Image filename: {:?}", original_filename);

                // Read file data
                let data = field.bytes().await.map_err(|e| {
                    tracing::error!("Failed to read image bytes: {:?}", e);
                    StatusCode::BAD_REQUEST
                })?;

                tracing::debug!("Read {} bytes of image data", data.len());

                // Check file size
                if data.len() > state.config.max_upload_size {
                    tracing::warn!(
                        "File too large: {} bytes > {} bytes",
                        data.len(),
                        state.config.max_upload_size
                    );
                    return Err(StatusCode::PAYLOAD_TOO_LARGE);
                }

                image_data = Some(data.to_vec());
            }
            "component_id" => {
                let text = field.text().await.map_err(|e| {
                    tracing::error!("Failed to read component_id text: {:?}", e);
                    StatusCode::BAD_REQUEST
                })?;
                component_id = text.parse::<i64>().ok();
                tracing::debug!("component_id: {:?}", component_id);
            }
            "slug" => {
                slug = Some(field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?);
            }
            "title" => {
                let text = field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?;
                if !text.is_empty() {
                    title = Some(text);
                }
            }
            "description" => {
                let text = field.text().await.map_err(|_| StatusCode::BAD_REQUEST)?;
                if !text.is_empty() {
                    description = Some(text);
                }
            }
            _ => {} // Ignore unknown fields
        }
    }

    // Validate required fields
    let image_data = image_data.ok_or_else(|| {
        tracing::error!("No image data received");
        StatusCode::BAD_REQUEST
    })?;
    let original_filename = original_filename.ok_or_else(|| {
        tracing::error!("No original filename received");
        StatusCode::BAD_REQUEST
    })?;
    let component_id = component_id.ok_or_else(|| {
        tracing::error!("No component_id received");
        StatusCode::BAD_REQUEST
    })?;
    let slug = sanitize_slug(&slug.unwrap_or_else(|| "image".to_string()));

    tracing::debug!(
        "Validated fields - component_id: {}, slug: {}, filename: {}",
        component_id,
        slug,
        original_filename
    );

    // Extract image metadata
    let metadata = extract_image_metadata(&image_data).map_err(|e| {
        tracing::error!("Failed to extract image metadata: {:?}", e);
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    })?;
    tracing::debug!(
        "Extracted metadata - format: {:?}, size: {}",
        metadata.format,
        metadata.size
    );

    // Create upload directory
    let now = Utc::now();
    let upload_base = PathBuf::from(&state.config.uploads_dir);
    tracing::debug!("Upload base directory: {:?}", upload_base);
    let upload_dir = create_upload_directory(&upload_base, now).map_err(|e| {
        tracing::error!("Failed to create upload directory: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    tracing::debug!("Created upload directory: {:?}", upload_dir);

    // Generate unique filename
    let filename = generate_unique_filename(&original_filename);
    tracing::debug!("Generated filename: {}", filename);

    // Save file to disk
    let file_path = save_upload(&image_data, &upload_dir, &filename).map_err(|e| {
        tracing::error!("Failed to save upload: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    tracing::debug!("Saved file to: {:?}", file_path);

    // Update the component with the new image data
    let component_repo = ComponentRepository::new(state.db.clone());

    // Get or create draft version
    let draft_version = get_or_create_draft(
        &state.db,
        page.id.unwrap(),
        Some(user.user.username.clone()),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Verify the component belongs to the draft version
    let component = component_repo
        .find_by_id(component_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    if component.page_version_id != draft_version.id.unwrap() {
        return Err(StatusCode::FORBIDDEN);
    }

    // Create new component content
    let content = json!({
        "slug": slug,
        "title": title.clone().unwrap_or_else(|| slug.clone()),
        "description": description.unwrap_or_default(),
        "format": metadata.format.extension(),
        "file_path": file_path.to_string_lossy(),
        "original_name": original_filename,
        "mime_type": metadata.format.mime_type(),
        "size": metadata.size,
        "width": metadata.width,
        "height": metadata.height,
    });

    // Update the component
    component_repo
        .update_content(
            component_id,
            content.clone(),
            title,
            component.template.clone(),
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Return the updated image data as JSON
    Ok(Json(json!({
        "success": true,
        "slug": slug,
        "title": content["title"],
        "description": content["description"],
        "format": metadata.format.extension(),
        "file_path": file_path.to_string_lossy(),
        "original_name": original_filename,
        "mime_type": metadata.format.mime_type(),
        "size": metadata.size,
        "width": metadata.width,
        "height": metadata.height,
    }))
    .into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use doxyde_core::models::{page::Page, session::Session, user::User};
    use sqlx::SqlitePool;

    #[test]
    fn test_sanitize_slug_for_filenames() {
        // Test that common filename patterns are handled well
        assert_eq!(sanitize_slug("My Photo.jpg"), "my-photojpg");
        assert_eq!(sanitize_slug("vacation-2024"), "vacation-2024");
        assert_eq!(sanitize_slug("IMG_1234"), "img-1234");
        assert_eq!(sanitize_slug("Test Image"), "test-image");
        assert_eq!(sanitize_slug("my@#$%image"), "myimage");
        assert_eq!(sanitize_slug("___test___"), "test");
    }

    #[test]
    fn test_image_upload_form_parsing() {
        // Test that the ImageUploadForm structure works correctly
        let form = ImageUploadForm {
            slug: "test-image".to_string(),
            title: Some("Test Image".to_string()),
            description: Some("A test image".to_string()),
        };

        assert_eq!(form.slug, "test-image");
        assert_eq!(form.title.unwrap(), "Test Image");
        assert_eq!(form.description.unwrap(), "A test image");

        // Test with minimal data
        let minimal_form = ImageUploadForm {
            slug: "minimal".to_string(),
            title: None,
            description: None,
        };

        assert_eq!(minimal_form.slug, "minimal");
        assert!(minimal_form.title.is_none());
        assert!(minimal_form.description.is_none());
    }
}
