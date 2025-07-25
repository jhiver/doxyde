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
use doxyde_core::{
    models::{page::Page, site::Site},
    utils::slug::generate_slug_from_title,
};
use doxyde_db::repositories::PageRepository;
use serde::Deserialize;
use tera::Context;

use super::{edit::can_edit_page, shared::add_action_bar_context};
use crate::{
    auth::CurrentUser, template_context::add_base_context, template_utils::discover_page_templates,
    AppState,
};

#[derive(Debug, Deserialize)]
pub struct PagePropertiesForm {
    pub title: String,
    pub slug: Option<String>,
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

    // Add all action bar context variables
    add_action_bar_context(&mut context, &state, &page, &user, ".properties").await?;

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

    // Handle slug change (only for non-root pages)
    if page.parent_page_id.is_some() {
        if let Some(new_slug) = form.slug.filter(|s| !s.is_empty()) {
            // Only update if slug is different
            if new_slug != page.slug {
                page.slug = new_slug;
            }
        } else {
            // Auto-generate slug from title if empty
            page.slug = generate_slug_from_title(&page.title);
        }
    }

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
    if let Err(validation_error) = page.is_valid() {
        tracing::error!("Page validation failed: {}", validation_error);

        // Create an error response with the validation details
        let error_html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>Validation Error</title>
    <style>
        body {{ font-family: Inter, sans-serif; margin: 40px; background: #f5f5f5; }}
        .error-box {{ background: white; border-radius: 8px; padding: 30px; box-shadow: 0 2px 8px rgba(0,0,0,0.1); max-width: 600px; margin: 0 auto; }}
        h1 {{ color: #dc3545; margin-bottom: 20px; }}
        p {{ color: #666; line-height: 1.6; }}
        .error-detail {{ background: #f8f9fa; padding: 15px; border-radius: 4px; border-left: 4px solid #dc3545; margin: 20px 0; }}
        .back-link {{ display: inline-block; margin-top: 20px; color: #007bff; text-decoration: none; }}
        .back-link:hover {{ text-decoration: underline; }}
    </style>
</head>
<body>
    <div class="error-box">
        <h1>Validation Error</h1>
        <p>The form could not be saved due to a validation error:</p>
        <div class="error-detail">
            <strong>{}</strong>
        </div>
        <p>Please go back and correct the error before submitting again.</p>
        <a href="javascript:history.back()" class="back-link">← Go Back</a>
    </div>
</body>
</html>"#,
            html_escape::encode_text(&validation_error)
        );

        return Ok((StatusCode::BAD_REQUEST, Html(error_html)).into_response());
    }

    // Save to database
    let page_repo = PageRepository::new(state.db.clone());
    page_repo
        .update(&page)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Build redirect path using the updated page
    let redirect_path = if page.parent_page_id.is_none() {
        "/".to_string()
    } else {
        // Get the updated breadcrumb trail (which includes the new slug)
        let breadcrumb = page_repo
            .get_breadcrumb_trail(page.id.unwrap())
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let path_parts: Vec<&str> = breadcrumb[1..].iter().map(|p| p.slug.as_str()).collect();
        format!("/{}", path_parts.join("/"))
    };

    Ok(Redirect::to(&redirect_path).into_response())
}
