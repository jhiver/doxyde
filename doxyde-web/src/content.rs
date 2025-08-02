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
    extract::{FromRequest, Request, State},
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use axum_extra::extract::Host;
use doxyde_core::models::{page::Page, site::Site};
use doxyde_db::repositories::{PageRepository, SiteRepository};
use once_cell::sync::Lazy;

use crate::{
    action_registry::ActionRegistry,
    auth::{CurrentUser, OptionalUser},
    error::AppError,
    handlers::serve_image_handler,
    AppState,
};

// Global action registry
static ACTION_REGISTRY: Lazy<ActionRegistry> = Lazy::new(ActionRegistry::build_default);

/// Represents a parsed content path with optional action
#[derive(Debug)]
pub struct ContentPath {
    pub path: String,
    pub action: Option<String>,
}

impl ContentPath {
    /// Parse a path like "/about/team/.edit" into path and action
    pub fn parse(path: &str) -> Self {
        // Find the last segment that starts with '.'
        let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        if let Some(last) = parts.last() {
            if last.starts_with('.') {
                // We have an action
                let path_parts = &parts[..parts.len() - 1];
                let path = if path_parts.is_empty() {
                    "/".to_string()
                } else {
                    format!("/{}", path_parts.join("/"))
                };

                return ContentPath {
                    path,
                    action: Some(last.to_string()),
                };
            }
        }

        // No action, just a content path
        ContentPath {
            path: if path.is_empty() || path == "/" {
                "/".to_string()
            } else {
                // Preserve the path as-is, including any trailing slashes
                path.to_string()
            },
            action: None,
        }
    }
}

/// Resolve site from host
async fn resolve_site(state: &AppState, host: &str) -> Result<Site, AppError> {
    let site_repo = SiteRepository::new(state.db.clone());
    match site_repo.find_by_domain(host).await {
        Ok(Some(site)) => Ok(site),
        Ok(None) => Err(
            AppError::not_found(format!("Site not found for domain: {}", host))
                .with_templates(state.templates.clone()),
        ),
        Err(e) => {
            tracing::error!(
                error = ?e,
                domain = %host,
                "Failed to query site by domain"
            );
            Err(
                AppError::internal_server_error("Failed to query site by domain")
                    .with_details(format!("{:?}", e))
                    .with_templates(state.templates.clone()),
            )
        }
    }
}

/// Navigate to page through path segments
async fn navigate_to_page(
    page_repo: &PageRepository,
    site_id: i64,
    path: &str,
    templates: &crate::autoreload_templates::TemplateEngine,
) -> Result<Page, AppError> {
    if path == "/" {
        // Get root page
        tracing::debug!("Getting root page for site {}", site_id);
        match page_repo.get_root_page(site_id).await {
            Ok(Some(page)) => Ok(page),
            Ok(None) => {
                Err(AppError::not_found("Root page not found").with_templates(templates.clone()))
            }
            Err(e) => {
                tracing::error!(
                    error = ?e,
                    site_id = ?site_id,
                    "Failed to get root page"
                );
                Err(AppError::internal_server_error("Failed to get root page")
                    .with_details(format!("{:?}", e))
                    .with_templates(templates.clone()))
            }
        }
    } else {
        // Navigate through the path segments
        let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

        tracing::debug!("Navigating to page through segments: {:?}", segments);

        // Start from root page
        let mut current_page = match page_repo.get_root_page(site_id).await {
            Ok(Some(page)) => page,
            Ok(None) => {
                return Err(
                    AppError::not_found("Root page not found").with_templates(templates.clone())
                )
            }
            Err(e) => {
                tracing::error!(
                    error = ?e,
                    site_id = ?site_id,
                    "Failed to get root page for navigation"
                );
                return Err(AppError::internal_server_error(
                    "Failed to get root page for navigation",
                )
                .with_details(format!("{:?}", e))
                .with_templates(templates.clone()));
            }
        };

        // Navigate through each segment
        for segment in segments {
            let current_page_id = current_page
                .id
                .ok_or_else(|| AppError::internal_server_error("Page has no ID"))?;
            let children = match page_repo.list_children(current_page_id).await {
                Ok(children) => children,
                Err(e) => {
                    tracing::error!(
                        error = ?e,
                        page_id = ?current_page.id,
                        "Failed to list children"
                    );
                    return Err(AppError::internal_server_error("Failed to list children")
                        .with_details(format!(
                            "Failed to list children for page {}: {:?}",
                            current_page_id,
                            e
                        ))
                        .with_templates(templates.clone()));
                }
            };

            // Find child with matching slug
            current_page = children
                .into_iter()
                .find(|p| p.slug == segment)
                .ok_or_else(|| {
                    AppError::not_found(format!("Page not found: {}", segment))
                        .with_templates(templates.clone())
                })?;
        }

        Ok(current_page)
    }
}

/// Handle trailing slash redirects for canonical URLs
fn handle_trailing_slash_redirect(uri: &Uri, content_path: &ContentPath) -> Option<Response> {
    let original_path = uri.path();

    match content_path.action {
        None => {
            // For page views, ensure trailing slash for canonical URLs
            // But only for non-root pages
            if content_path.path != "/" && !original_path.ends_with('/') {
                let redirect_path = format!("{}/", original_path);
                return Some(axum::response::Redirect::permanent(&redirect_path).into_response());
            }
        }
        Some(ref _action) => {
            // For actions, remove trailing slash for canonical URLs
            if original_path.ends_with('/') {
                let redirect_path = original_path.trim_end_matches('/');
                return Some(axum::response::Redirect::permanent(redirect_path).into_response());
            }
        }
    }

    None
}

/// Main content handler - resolves sites and pages dynamically
pub async fn content_handler(
    Host(host): Host,
    uri: Uri,
    State(state): State<AppState>,
    user: OptionalUser,
) -> Result<Response, AppError> {
    // Parse the path to extract content path and action
    let path = uri.path();

    // Add comprehensive debugging for .well-known requests
    if path.starts_with("/.well-known") || path.starts_with(".well-known") {
        tracing::warn!(
            path = %path,
            host = %host,
            uri = %uri,
            "DEBUGGING: .well-known request reached content handler (should be handled by route)"
        );
    }

    // Check if this is an image request (format: /slug.extension)
    if let Some((slug, format)) = check_image_pattern(path) {
        return handle_image_request(state, host.to_string(), slug, format).await;
    }

    let content_path = ContentPath::parse(path);

    tracing::debug!(
        path = %path,
        content_path = ?content_path,
        "Processing content request"
    );

    // Resolve site from host
    let site = resolve_site(&state, &host).await?;

    // Navigate to the requested page
    let page_repo = PageRepository::new(state.db.clone());
    let site_id = site
        .id
        .ok_or_else(|| AppError::internal_server_error("Site has no ID"))?;
    let page = navigate_to_page(
        &page_repo,
        site_id,
        &content_path.path,
        &state.templates,
    )
    .await?;

    // Handle trailing slash redirects
    if let Some(redirect) = handle_trailing_slash_redirect(&uri, &content_path) {
        return Ok(redirect);
    }

    tracing::info!(
        page_id = ?page.id,
        page_title = %page.title,
        action = ?content_path.action,
        "Routing request to handler"
    );

    // Get the action name (empty string for display)
    let action_name = content_path.action.as_deref().unwrap_or("");

    // Look up handler in registry
    if let Some(handler) = ACTION_REGISTRY.get(action_name) {
        handler(state, site, page, user).await
    } else {
        // Unknown action
        tracing::warn!(
            action = %action_name,
            "Unknown action requested"
        );
        Err(
            AppError::not_found(format!("Unknown action: {}", action_name))
                .with_templates(state.templates.clone()),
        )
    }
}

/// Check if a path matches the image URL pattern
fn check_image_pattern(path: &str) -> Option<(String, String)> {
    // Skip if path contains '/' beyond the first character
    let trimmed = path.trim_start_matches('/');
    if trimmed.contains('/') {
        return None;
    }

    // Check for pattern: slug.extension
    if let Some(dot_pos) = trimmed.rfind('.') {
        if dot_pos > 0 && dot_pos < trimmed.len() - 1 {
            let slug = &trimmed[..dot_pos];
            let format = &trimmed[dot_pos + 1..];

            // Validate format is a known image extension
            let valid_formats = ["jpg", "jpeg", "png", "gif", "webp", "svg"];
            if valid_formats.contains(&format) {
                return Some((slug.to_string(), format.to_string()));
            }
        }
    }

    None
}

/// Handle image request
async fn handle_image_request(
    state: AppState,
    host: String,
    slug: String,
    format: String,
) -> Result<Response, AppError> {
    // Find the site by domain
    let site_repo = SiteRepository::new(state.db.clone());
    let templates = state.templates.clone();
    let site = match site_repo.find_by_domain(&host).await {
        Ok(Some(site)) => site,
        Ok(None) => {
            return Err(AppError::not_found("Site not found").with_templates(templates.clone()))
        }
        Err(e) => {
            tracing::error!(error = ?e, "Failed to query site");
            return Err(AppError::internal_server_error("Failed to query site")
                .with_templates(templates.clone()));
        }
    };

    // Serve the image
    match serve_image_handler(State(state), site, axum::extract::Path((slug, format))).await {
        Ok(response) => Ok(response),
        Err(StatusCode::NOT_FOUND) => {
            Err(AppError::not_found("Image not found").with_templates(templates.clone()))
        }
        Err(_) => {
            Err(AppError::internal_server_error("Failed to serve image").with_templates(templates))
        }
    }
}

/// Main content POST handler - routes to appropriate action handlers
pub async fn content_post_handler(
    Host(host): Host,
    uri: Uri,
    State(state): State<AppState>,
    user: CurrentUser,
    request: Request,
) -> Result<Response, AppError> {
    // Parse the path to extract content path and action
    let path = uri.path();
    let content_path = ContentPath::parse(path);

    // Check if this is a multipart upload
    let is_multipart = request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|ct| ct.to_str().ok())
        .map(|ct| ct.starts_with("multipart/form-data"))
        .unwrap_or(false);

    if is_multipart
        && (content_path.action.as_deref() == Some(".upload-image")
            || content_path.action.as_deref() == Some(".upload-component-image"))
    {
        // Handle image upload with multipart
        // Find the site
        let site_repo = SiteRepository::new(state.db.clone());
        let templates = state.templates.clone();
        let site = match site_repo.find_by_domain(&host).await {
            Ok(Some(site)) => site,
            Ok(None) => {
                return Err(AppError::not_found("Site not found").with_templates(templates.clone()))
            }
            Err(e) => {
                tracing::error!(error = ?e, "Failed to query site");
                return Err(AppError::internal_server_error("Failed to query site")
                    .with_templates(templates));
            }
        };

        // Find the page
        let page_repo = PageRepository::new(state.db.clone());
        let site_id = site
            .id
            .ok_or_else(|| AppError::internal_server_error("Site has no ID"))?;
        let page = match resolve_page(&page_repo, site_id, &content_path.path).await {
            Ok(page) => page,
            Err(e) => return Err(e),
        };

        // Extract multipart from request - note this consumes the request
        let parts = request.into_parts();
        let request = Request::from_parts(parts.0, parts.1);
        let templates = state.templates.clone();
        let multipart = match axum::extract::Multipart::from_request(request, &state).await {
            Ok(mp) => mp,
            Err(_) => {
                return Err(AppError::bad_request("Invalid multipart data")
                    .with_templates(templates.clone()))
            }
        };

        // Handle upload based on action
        let response = if content_path.action.as_deref() == Some(".upload-component-image") {
            crate::handlers::upload_component_image_handler(
                State(state),
                site,
                page,
                user,
                multipart,
            )
            .await
        } else {
            crate::handlers::upload_image_handler(State(state), site, page, user, multipart).await
        };
        match response {
            Ok(response) => Ok(response),
            Err(StatusCode::FORBIDDEN) => Err(AppError::forbidden(
                "You don't have permission to upload images",
            )
            .with_templates(templates.clone())),
            Err(StatusCode::PAYLOAD_TOO_LARGE) => Err(AppError::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                "File too large",
            )
            .with_templates(templates.clone())),
            Err(_) => {
                Err(AppError::internal_server_error("Failed to upload image")
                    .with_templates(templates))
            }
        }
    } else {
        // Handle regular form POST - convert request body to String
        let body = match axum::body::to_bytes(request.into_body(), usize::MAX).await {
            Ok(bytes) => String::from_utf8_lossy(&bytes).to_string(),
            Err(_) => {
                return Err(AppError::bad_request("Invalid request body")
                    .with_templates(state.templates.clone()))
            }
        };

        // Call the existing action handler
        match crate::handlers::handle_action(Host(host), uri, State(state.clone()), user, body)
            .await
        {
            Ok(response) => Ok(response),
            Err(status) => {
                Err(AppError::new(status, "Action failed").with_templates(state.templates))
            }
        }
    }
}

/// Helper to resolve a page from path
async fn resolve_page(
    page_repo: &PageRepository,
    site_id: i64,
    path: &str,
) -> Result<doxyde_core::models::page::Page, AppError> {
    if path == "/" {
        page_repo
            .get_root_page(site_id)
            .await
            .map_err(|_| AppError::internal_server_error("Failed to get root page"))?
            .ok_or_else(|| AppError::not_found("Root page not found"))
    } else {
        let segments: Vec<&str> = path
            .trim_start_matches('/')
            .trim_end_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        // Navigate through the path segments
        let mut current_page = page_repo
            .get_root_page(site_id)
            .await
            .map_err(|_| AppError::internal_server_error("Failed to get root page"))?
            .ok_or_else(|| AppError::not_found("Root page not found"))?;

        for slug in segments {
            let current_page_id = current_page
                .id
                .ok_or_else(|| AppError::internal_server_error("Page has no ID"))?;
            let children = page_repo
                .list_children(current_page_id)
                .await
                .map_err(|_| AppError::internal_server_error("Failed to list children"))?;

            match children.into_iter().find(|p| p.slug == slug) {
                Some(page) => current_page = page,
                None => return Err(AppError::not_found("Page not found")),
            }
        }

        Ok(current_page)
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_content_path_no_action() {
        let path = ContentPath::parse("/about/team");
        assert_eq!(path.path, "/about/team");
        assert!(path.action.is_none());
    }

    #[test]
    fn test_parse_content_path_with_action() {
        let path = ContentPath::parse("/about/team/.edit");
        assert_eq!(path.path, "/about/team");
        assert_eq!(path.action, Some(".edit".to_string()));
    }

    #[test]
    fn test_parse_content_path_root() {
        let path = ContentPath::parse("/");
        assert_eq!(path.path, "/");
        assert!(path.action.is_none());
    }

    #[test]
    fn test_parse_content_path_root_action() {
        let path = ContentPath::parse("/.new");
        assert_eq!(path.path, "/");
        assert_eq!(path.action, Some(".new".to_string()));
    }

    #[test]
    fn test_parse_content_path_empty() {
        let path = ContentPath::parse("");
        assert_eq!(path.path, "/");
        assert!(path.action.is_none());
    }

    #[test]
    fn test_parse_content_path_multiple_dots() {
        let path = ContentPath::parse("/about/.test/team/.edit");
        assert_eq!(path.path, "/about/.test/team");
        assert_eq!(path.action, Some(".edit".to_string()));
    }

    #[test]
    fn test_parse_content_path_trailing_slash() {
        let path = ContentPath::parse("/about/team/");
        assert_eq!(path.path, "/about/team/");
        assert!(path.action.is_none());
    }

    #[test]
    fn test_parse_content_path_action_trailing_slash() {
        let path = ContentPath::parse("/about/team/.edit/");
        assert_eq!(path.path, "/about/team");
        assert_eq!(path.action, Some(".edit".to_string()));
    }
}
