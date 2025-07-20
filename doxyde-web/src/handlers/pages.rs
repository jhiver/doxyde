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

use anyhow::Result;
use axum::{
    extract::Path,
    http::StatusCode,
    response::{Html, IntoResponse},
};
use doxyde_core::models::{page::Page, site::Site};
use doxyde_db::repositories::{
    ComponentRepository, PageRepository, PageVersionRepository, SiteUserRepository,
};
use tera::Context;

use crate::{auth::OptionalUser, template_context::add_base_context, AppState};

/// Display a page by slug (old route handler - kept for compatibility)
pub async fn show_page(Path(_slug): Path<String>) -> Result<&'static str, StatusCode> {
    Ok("Page handler placeholder")
}

/// List all pages for a site
pub async fn list_pages() -> Result<&'static str, StatusCode> {
    Ok("Pages list placeholder")
}

/// Display a page with its components
pub async fn show_page_handler(
    state: AppState,
    site: Site,
    page: Page,
    user: OptionalUser,
) -> Result<impl IntoResponse, StatusCode> {
    let page_repo = PageRepository::new(state.db.clone());
    let version_repo = PageVersionRepository::new(state.db.clone());
    let component_repo = ComponentRepository::new(state.db.clone());

    // Get the published version of the page
    let published_version = version_repo
        .get_published(page.id.unwrap())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get components if we have a published version
    let mut components = if let Some(version) = &published_version {
        component_repo
            .list_by_page_version(version.id.unwrap())
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        Vec::new()
    };

    // Process dynamic components (e.g., blog_summary)
    for component in &mut components {
        if component.component_type == "blog_summary" {
            // Parse the config
            if let Ok(mut config) =
                serde_json::from_value::<serde_json::Value>(component.content.clone())
            {
                if let Some(parent_page_id) = config.get("parent_page_id").and_then(|v| v.as_i64())
                {
                    // Fetch child pages
                    let child_pages = page_repo
                        .list_children_sorted(parent_page_id)
                        .await
                        .unwrap_or_default();

                    // Get item count
                    let item_count = config
                        .get("item_count")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(5) as usize;

                    // Build page data with URLs
                    let mut pages_data = Vec::new();
                    for child in child_pages.iter().take(item_count) {
                        // Build URL for child page
                        let child_breadcrumb = page_repo
                            .get_breadcrumb_trail(child.id.unwrap())
                            .await
                            .unwrap_or_default();

                        let child_url = if child_breadcrumb.len() <= 1 {
                            "/".to_string()
                        } else {
                            let path_parts: Vec<&str> = child_breadcrumb[1..]
                                .iter()
                                .map(|p| p.slug.as_str())
                                .collect();
                            format!("/{}", path_parts.join("/"))
                        };

                        pages_data.push(serde_json::json!({
                            "id": child.id,
                            "title": child.title,
                            "slug": child.slug,
                            "description": child.description,
                            "created_at": child.created_at.format("%B %d, %Y").to_string(),
                            "url": child_url
                        }));
                    }

                    // Inject pages data into component content
                    config["pages"] = serde_json::json!(pages_data);
                    component.content = config;
                }
            }
        }
    }

    // Get child pages using sorted method
    let children = page_repo
        .list_children_sorted(page.id.unwrap())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get breadcrumb trail
    let breadcrumb = page_repo
        .get_breadcrumb_trail(page.id.unwrap())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Build breadcrumb data for template
    let mut breadcrumb_data = Vec::new();
    for (i, crumb) in breadcrumb.iter().enumerate() {
        // Build URL from breadcrumb
        let url = if i == 0 {
            "/".to_string()
        } else {
            let path_parts: Vec<&str> = breadcrumb[1..=i].iter().map(|p| p.slug.as_str()).collect();
            format!("/{}", path_parts.join("/"))
        };

        breadcrumb_data.push(serde_json::json!({
            "title": crumb.title,
            "url": url
        }));
    }

    // Build current page path
    let current_path = if breadcrumb.len() <= 1 {
        "/".to_string()
    } else {
        // Build current page path from breadcrumb (excluding root)
        let path_parts: Vec<&str> = breadcrumb[1..].iter().map(|p| p.slug.as_str()).collect();
        format!("/{}", path_parts.join("/"))
    };

    // Build hierarchical navigation data
    let mut navigation_levels = Vec::new();

    // Build navigation from current page up to root, showing children of each
    for (i, nav_page) in breadcrumb.iter().enumerate().rev() {
        // Get children of this page
        let page_children = page_repo
            .list_children_sorted(nav_page.id.unwrap())
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // Skip if no children
        if page_children.is_empty() {
            continue;
        }

        // Find which child (if any) is in our breadcrumb path
        let next_in_path = if i < breadcrumb.len() - 1 {
            breadcrumb.get(i + 1).and_then(|p| p.id)
        } else {
            None
        };

        // Build URLs for children
        let children_data: Vec<serde_json::Value> = page_children
            .into_iter()
            .map(|child| {
                let child_url = if i == 0 {
                    // Children of root
                    format!("/{}", child.slug)
                } else {
                    // Build path from breadcrumb up to current level
                    let path_parts: Vec<&str> =
                        breadcrumb[1..=i].iter().map(|p| p.slug.as_str()).collect();
                    format!("/{}/{}", path_parts.join("/"), child.slug)
                };

                // Check if this child is the current page or in the path to current page
                let is_active = child.id == page.id || child.id == next_in_path;

                serde_json::json!({
                    "title": child.title,
                    "url": child_url,
                    "is_active": is_active,
                    "is_current_page": child.id == page.id
                })
            })
            .collect();

        // Add level to navigation (will be reversed to show top-down)
        navigation_levels.push(serde_json::json!({
            "title": nav_page.title.clone(),
            "pages": children_data
        }));
    }

    // Reverse to show from top to bottom
    navigation_levels.reverse();

    // Keep children data for backward compatibility
    let children_data: Vec<serde_json::Value> = children
        .iter()
        .map(|child| {
            let child_url = if current_path == "/" {
                format!("/{}", child.slug)
            } else {
                format!("{}/{}", current_path, child.slug)
            };
            serde_json::json!({
                "title": child.title,
                "url": child_url
            })
        })
        .collect();

    // Prepare template context
    let mut context = Context::new();

    // Add base context (site_title, root_page_title, logo data, navigation)
    add_base_context(&mut context, &state, &site, Some(&page))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    context.insert("page", &page);
    context.insert("components", &components);
    context.insert("breadcrumbs", &breadcrumb_data);
    context.insert("navigation_levels", &navigation_levels);
    context.insert("children", &children_data); // Keep for backward compatibility
    context.insert("current_path", &current_path);
    context.insert("has_children", &!children.is_empty());

    // Add user info and check edit permissions if logged in
    let mut can_edit = false;
    if let OptionalUser(Some(current_user)) = &user {
        context.insert("user", &current_user.user);

        // Check if user can edit this page
        if current_user.user.is_admin {
            can_edit = true;
        } else {
            // Check site permissions
            let site_user_repo = SiteUserRepository::new(state.db.clone());
            if let Ok(Some(site_user)) = site_user_repo
                .find_by_site_and_user(site.id.unwrap(), current_user.user.id.unwrap())
                .await
            {
                use doxyde_core::models::permission::SiteRole;
                can_edit = site_user.role == SiteRole::Editor || site_user.role == SiteRole::Owner;
            }
        }
    }
    context.insert("can_edit", &can_edit);
    context.insert("action", "view");

    // Check if page is movable (has valid move targets)
    let is_movable = if let Some(page_id) = page.id {
        if page.parent_page_id.is_some() && can_edit {
            // Only non-root pages can be moved, and only by editors
            match page_repo.get_valid_move_targets(page_id).await {
                Ok(targets) => {
                    tracing::debug!(
                        page_id = page_id,
                        page_slug = %page.slug,
                        target_count = targets.len(),
                        "Checked move targets for page"
                    );
                    !targets.is_empty()
                }
                Err(e) => {
                    tracing::error!(
                        page_id = page_id,
                        error = %e,
                        "Failed to get valid move targets"
                    );
                    false
                }
            }
        } else {
            tracing::debug!(
                page_id = page_id,
                has_parent = page.parent_page_id.is_some(),
                can_edit = can_edit,
                "Page not movable: missing parent or edit permission"
            );
            false
        }
    } else {
        false
    };
    context.insert("is_movable", &is_movable);

    // Check if page can be deleted (not root and has no children)
    let can_delete = if let Some(page_id) = page.id {
        if page.parent_page_id.is_some() && can_edit {
            // Only non-root pages can be deleted, and only by editors
            match page_repo.has_children(page_id).await {
                Ok(has_children) => !has_children,
                Err(_) => false,
            }
        } else {
            false
        }
    } else {
        false
    };
    context.insert("can_delete", &can_delete);

    // Render the template based on page template type
    let template_name = format!("page_templates/{}.html", page.template);

    // Try to render with specific template, fall back to default if not found
    let html = match state.templates.render(&template_name, &context) {
        Ok(html) => html,
        Err(_) => {
            // Fall back to default template
            state
                .templates
                .render("page_templates/default.html", &context)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        }
    };

    Ok(Html(html))
}
