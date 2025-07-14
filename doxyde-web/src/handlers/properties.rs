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
    extract::Form,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use doxyde_core::models::{page::Page, site::Site};
use doxyde_db::repositories::PageRepository;
use serde::Deserialize;
use tera::Context;

use super::edit::can_edit_page;
use crate::{
    auth::CurrentUser, template_context::add_base_context, template_utils::discover_page_templates,
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct PagePropertiesForm {
    pub title: String,
    pub description: Option<String>,
    pub keywords: Option<String>,
    pub template: String,
    pub meta_robots: String,
    pub canonical_url: Option<String>,
    pub og_image_url: Option<String>,
    pub structured_data_type: String,
}

/// Display page properties form
pub async fn page_properties_handler(
    state: AppState,
    site: Site,
    page: Page,
    user: CurrentUser,
) -> Result<Response, StatusCode> {
    // Check permissions
    if !can_edit_page(&state, &site, &user).await? {
        return Err(StatusCode::FORBIDDEN);
    }

    let page_repo = PageRepository::new(state.db.clone());

    // Get breadcrumb for navigation
    let breadcrumb = page_repo
        .get_breadcrumb_trail(page.id.unwrap())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Build breadcrumb data
    let mut breadcrumb_data = Vec::new();
    for (i, crumb) in breadcrumb.iter().enumerate() {
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
    let current_path = if page.parent_page_id.is_none() {
        "/".to_string()
    } else {
        let path_parts: Vec<&str> = breadcrumb[1..].iter().map(|p| p.slug.as_str()).collect();
        format!("/{}", path_parts.join("/"))
    };

    // Prepare template context
    let mut context = Context::new();

    // Add base context (site_title, root_page_title, logo data, navigation)
    add_base_context(&mut context, &state, &site, Some(&page))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    context.insert("page", &page);
    context.insert("breadcrumbs", &breadcrumb_data);
    context.insert("current_path", &current_path);
    context.insert("user", &user.user);
    context.insert("can_edit", &true);
    context.insert("action", ".properties");

    // Check if page is movable (has valid move targets)
    let is_movable = if let Some(page_id) = page.id {
        if page.parent_page_id.is_some() {
            // Only non-root pages can be moved
            match page_repo.get_valid_move_targets(page_id).await {
                Ok(targets) => !targets.is_empty(),
                Err(_) => false,
            }
        } else {
            false
        }
    } else {
        false
    };
    context.insert("is_movable", &is_movable);

    // Check if page can be deleted (not root and has no children)
    let can_delete = if let Some(page_id) = page.id {
        if page.parent_page_id.is_some() {
            // Only non-root pages can be deleted
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

    // Discover available templates dynamically
    let templates_path = std::path::Path::new(&state.config.templates_dir);
    let available_templates = discover_page_templates(templates_path);
    context.insert("available_templates", &available_templates);
    context.insert(
        "available_robots",
        &[
            "index,follow",
            "noindex,follow",
            "index,nofollow",
            "noindex,nofollow",
        ],
    );
    context.insert(
        "available_data_types",
        &[
            "WebPage",
            "Article",
            "BlogPosting",
            "Product",
            "Organization",
            "Person",
            "Event",
            "FAQPage",
        ],
    );

    // Render the properties template
    let html = state
        .templates
        .render("page_properties.html", &context)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Html(html).into_response())
}

/// Handle page properties update
pub async fn update_page_properties_handler(
    state: AppState,
    site: Site,
    mut page: Page,
    user: CurrentUser,
    Form(form): Form<PagePropertiesForm>,
) -> Result<Response, StatusCode> {
    // Check permissions
    if !can_edit_page(&state, &site, &user).await? {
        return Err(StatusCode::FORBIDDEN);
    }

    // Update page properties
    page.title = form.title;
    page.description = form.description.filter(|s| !s.is_empty());
    page.keywords = form.keywords.filter(|s| !s.is_empty());
    page.template = form.template;
    page.meta_robots = form.meta_robots;
    page.canonical_url = form.canonical_url.filter(|s| !s.is_empty());
    page.og_image_url = form.og_image_url.filter(|s| !s.is_empty());
    page.structured_data_type = form.structured_data_type;

    // Update timestamp
    page.updated_at = chrono::Utc::now();

    // Validate the page
    if let Err(_e) = page.is_valid() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Save to database
    let page_repo = PageRepository::new(state.db);
    page_repo
        .update(&page)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Build redirect path
    let redirect_path = if page.parent_page_id.is_none() {
        "/".to_string()
    } else {
        let breadcrumb = page_repo
            .get_breadcrumb_trail(page.id.unwrap())
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let path_parts: Vec<&str> = breadcrumb[1..].iter().map(|p| p.slug.as_str()).collect();
        format!("/{}", path_parts.join("/"))
    };

    Ok(Redirect::to(&redirect_path).into_response())
}
