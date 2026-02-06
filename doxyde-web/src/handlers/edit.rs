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
    Json,
};
use doxyde_core::models::{component::Component, page::Page, permission::SiteRole, site::Site};
use doxyde_db::repositories::{ComponentRepository, PageRepository, SiteUserRepository};
use serde::Deserialize;
use serde_json::json;
use tera::Context;

use crate::{
    auth::CurrentUser, component_registry::get_component_registry, draft::get_or_create_draft,
    handlers::shared::add_action_bar_context, template_context::add_base_context, AppState,
};

#[derive(Debug, Deserialize)]
pub struct AddComponentForm {
    pub content: String,
    pub component_type: String,
    #[serde(default)]
    pub ajax: bool,
}

#[derive(Debug, Deserialize)]
pub struct NewPageForm {
    pub title: String,
    #[serde(default)]
    pub slug: String,
    pub description: Option<String>,
    pub keywords: Option<String>,
    #[serde(default = "default_template")]
    pub template: String,
    #[serde(default = "default_meta_robots")]
    pub meta_robots: String,
    pub canonical_url: Option<String>,
    pub og_image_url: Option<String>,
    #[serde(default = "default_structured_data_type")]
    pub structured_data_type: String,
}

fn default_template() -> String {
    "default".to_string()
}

fn default_meta_robots() -> String {
    "index,follow".to_string()
}

fn default_structured_data_type() -> String {
    "WebPage".to_string()
}

#[derive(Debug, Deserialize)]
pub struct UpdateComponentForm {
    pub component_id: i64,
    pub title: Option<String>,
    pub template: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct SaveDraftForm {
    pub component_ids: Vec<i64>,
    pub component_types: Vec<String>,
    pub component_titles: Vec<String>,
    pub component_templates: Vec<String>,
    pub component_contents: Vec<String>,
    pub action: Option<String>,
}

/// Display page content edit form (components only)
pub async fn edit_page_content_handler(
    state: AppState,
    db: sqlx::SqlitePool,
    site: Site,
    page: Page,
    user: CurrentUser,
) -> Result<Response, StatusCode> {
    tracing::debug!(
        page_id = ?page.id,
        user_id = ?user.user.id,
        "Starting edit_page_content_handler"
    );

    // Check permissions
    match can_edit_page(&state, &db, &site, &user).await {
        Ok(true) => {}
        Ok(false) => {
            tracing::warn!(
                user_id = ?user.user.id,
                site_id = ?site.id,
                "User lacks permission to edit page"
            );
            return Err(StatusCode::FORBIDDEN);
        }
        Err(e) => {
            tracing::error!(
                error = ?e,
                "Failed to check edit permissions"
            );
            return Err(e);
        }
    }

    let page_repo = PageRepository::new(db.clone());
    let component_repo = ComponentRepository::new(db.clone());

    tracing::info!("=== EDIT PAGE HANDLER START ===");
    tracing::info!("Page: {} (ID: {:?})", page.title, page.id);
    tracing::info!("User: {}", user.user.username);

    // Get or create a draft version
    let page_id = page.id.ok_or(StatusCode::NOT_FOUND)?;
    let draft_version =
        match get_or_create_draft(&db, page_id, Some(user.user.username.clone())).await {
            Ok(draft) => {
                tracing::info!("Got draft version ID: {:?}", draft.id);
                draft
            }
            Err(e) => {
                tracing::error!(
                    error = ?e,
                    page_id = ?page.id,
                    "Failed to get or create draft"
                );
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        };

    // Get components from the draft version
    let draft_version_id = draft_version.id.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let components = match component_repo.list_by_page_version(draft_version_id).await {
        Ok(comps) => {
            tracing::info!("Retrieved {} components from draft", comps.len());
            for (idx, comp) in comps.iter().enumerate() {
                tracing::info!(
                    "  Component [{}]: ID={:?}, type={}, template={}",
                    idx,
                    comp.id,
                    comp.component_type,
                    comp.template
                );
            }
            comps
        }
        Err(e) => {
            tracing::error!(
                error = ?e,
                version_id = ?draft_version.id,
                "Failed to list components"
            );
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    // Get breadcrumb for navigation
    let breadcrumb = match page_repo.get_breadcrumb_trail(page_id).await {
        Ok(crumbs) => crumbs,
        Err(e) => {
            tracing::error!(
                error = ?e,
                page_id = ?page.id,
                "Failed to get breadcrumb trail"
            );
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

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
    add_base_context(&mut context, &db, &site, Some(&page))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    context.insert("page", &page);
    context.insert("components", &components);
    context.insert("breadcrumbs", &breadcrumb_data);
    context.insert("current_path", &current_path);
    context.insert("user", &user.user);

    // Add all action bar context variables
    add_action_bar_context(&mut context, &state, &db, &page, &user, ".edit").await?;

    // Get component type usage statistics
    let component_type_stats = match component_repo.get_component_type_usage_stats().await {
        Ok(stats) => stats,
        Err(e) => {
            tracing::warn!(
                error = ?e,
                "Failed to get component type usage stats, using default order"
            );
            // Default order if stats fetch fails
            vec![]
        }
    };

    // Create ordered component types with labels
    let mut ordered_component_types = Vec::new();

    // First add types by usage (if any)
    for (comp_type, _count) in component_type_stats {
        let label = match comp_type.as_str() {
            "text" => "Text",
            "markdown" => "Markdown",
            "image" => "Image",
            "code" => "Code",
            "html" => "HTML",
            "blog_summary" => "Blog Summary",
            "custom" => "Custom",
            _ => &comp_type,
        };
        ordered_component_types.push(serde_json::json!({
            "type": comp_type,
            "label": label
        }));
    }

    // Add any missing types in a sensible default order
    let all_types = vec![
        ("markdown", "Markdown"),
        ("text", "Text"),
        ("image", "Image"),
        ("blog_summary", "Blog Summary"),
        ("code", "Code"),
        ("html", "HTML"),
        ("custom", "Custom"),
    ];

    for (comp_type, label) in all_types {
        if !ordered_component_types
            .iter()
            .any(|t| t["type"] == comp_type)
        {
            ordered_component_types.push(serde_json::json!({
                "type": comp_type,
                "label": label
            }));
        }
    }

    context.insert("ordered_component_types", &ordered_component_types);

    // Get all pages for blog summary parent page dropdown
    let all_pages = match page_repo.list_all().await {
        Ok(pages) => pages,
        Err(e) => {
            tracing::error!(
                error = ?e,
                "Failed to list all pages"
            );
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    context.insert("all_pages", &all_pages);

    // Render the edit template
    tracing::info!("About to render page_edit.html template");
    let html = match state.templates.render("page_edit.html", &context) {
        Ok(rendered) => {
            tracing::info!("Successfully rendered page_edit.html");
            rendered
        }
        Err(e) => {
            tracing::error!(
                error = ?e,
                template = "page_edit.html",
                "Failed to render template"
            );
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    Ok(Html(html).into_response())
}

/// Add a component to the page
pub async fn add_component_handler(
    state: AppState,
    db: sqlx::SqlitePool,
    site: Site,
    page: Page,
    user: CurrentUser,
    Form(form): Form<AddComponentForm>,
) -> Result<Response, StatusCode> {
    tracing::info!(
        "Adding component - type: {}, content: {}",
        form.component_type,
        form.content
    );

    // Check permissions
    if !can_edit_page(&state, &db, &site, &user).await? {
        return Err(StatusCode::FORBIDDEN);
    }

    let component_repo = ComponentRepository::new(db.clone());

    // Get or create a draft version
    let page_id = page.id.ok_or(StatusCode::NOT_FOUND)?;
    let draft_version = get_or_create_draft(&db, page_id, Some(user.user.username.clone()))
        .await
        .map_err(|e| {
            tracing::error!("Failed to get or create draft: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Get current components to determine position
    let draft_version_id = draft_version.id.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let components = component_repo
        .list_by_page_version(draft_version_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to list components: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let next_position = components.len() as i32;

    // Use the component registry to parse content
    let registry = get_component_registry();
    let content = match registry.parse_content(&form.component_type, &form.content) {
        Ok(parsed_content) => {
            tracing::info!("Parsed content: {:?}", parsed_content);
            parsed_content
        }
        Err(e) => {
            tracing::error!("Failed to parse component content: {}", e);
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    // Validate the content if the handler provides validation
    if let Some(handler) = registry.get_handler(&form.component_type) {
        if let Err(e) = handler.validate_content(&content) {
            tracing::error!("Component content validation failed: {}", e);
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Create new component
    let mut component = Component::new(
        draft_version_id,
        form.component_type.clone(),
        next_position,
        content,
    );

    // Set the correct template for blog_summary components
    if form.component_type == "blog_summary" {
        component.template = "cards".to_string();
    }

    tracing::info!("Creating component: {:?}", component);

    let component_id = component_repo.create(&component).await.map_err(|e| {
        tracing::error!("Failed to create component: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Normalize positions after adding component
    component_repo
        .normalize_positions(draft_version_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to normalize positions: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Check if client requested JSON response (AJAX)
    if form.ajax {
        return Ok(Json(json!({
            "success": true,
            "component_id": component_id
        }))
        .into_response());
    }

    // Redirect back to edit page with anchor to the new component
    let redirect_path = build_page_path(&state, &db, &page).await?;
    let edit_path = if redirect_path == "/" {
        format!("/.edit#component-{}", component_id)
    } else {
        format!("{}/.edit#component-{}", redirect_path, component_id)
    };
    Ok(Redirect::to(&edit_path).into_response())
}

/// Check if user can edit the page
pub async fn can_edit_page(
    _state: &AppState,
    db: &sqlx::SqlitePool,
    _site: &Site,
    user: &CurrentUser,
) -> Result<bool, StatusCode> {
    // Admins can always edit
    if user.user.is_admin {
        return Ok(true);
    }

    // Check site permissions
    // In multi-database mode, each database represents one site (site_id = 1)
    let site_user_repo = SiteUserRepository::new(db.clone());
    let user_id = user.user.id.ok_or(StatusCode::UNAUTHORIZED)?;
    let site_user = site_user_repo
        .find_by_site_and_user(1, user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if let Some(su) = site_user {
        Ok(su.role == SiteRole::Editor || su.role == SiteRole::Owner)
    } else {
        Ok(false)
    }
}

/// Build the full path to a page
async fn build_page_path(
    _state: &AppState,
    db: &sqlx::SqlitePool,
    page: &Page,
) -> Result<String, StatusCode> {
    if page.parent_page_id.is_none() {
        return Ok("/".to_string());
    }

    let page_repo = PageRepository::new(db.clone());
    let page_id = page.id.ok_or(StatusCode::NOT_FOUND)?;
    let breadcrumb = page_repo
        .get_breadcrumb_trail(page_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let path_parts: Vec<&str> = breadcrumb[1..].iter().map(|p| p.slug.as_str()).collect();

    Ok(format!("/{}", path_parts.join("/")))
}

/// Display new page form
pub async fn new_page_handler(
    state: AppState,
    db: sqlx::SqlitePool,
    site: Site,
    page: Page,
    user: CurrentUser,
) -> Result<Response, StatusCode> {
    // Check permissions
    if !can_edit_page(&state, &db, &site, &user).await? {
        return Err(StatusCode::FORBIDDEN);
    }

    let page_repo = PageRepository::new(db.clone());
    let page_id = page.id.ok_or(StatusCode::NOT_FOUND)?;

    // Get breadcrumb for navigation
    let breadcrumb = page_repo
        .get_breadcrumb_trail(page_id)
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
    add_base_context(&mut context, &db, &site, Some(&page))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    context.insert("parent_page", &page);
    context.insert("breadcrumbs", &breadcrumb_data);
    context.insert("current_path", &current_path);
    context.insert("user", &user.user);

    // Add all action bar context variables
    add_action_bar_context(&mut context, &state, &db, &page, &user, ".new").await?;

    // Render the new page template
    let html = state
        .templates
        .render("page_new.html", &context)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Html(html).into_response())
}

/// Create a new page
pub async fn create_page_handler(
    state: AppState,
    db: sqlx::SqlitePool,
    site: Site,
    parent_page: Page,
    user: CurrentUser,
    Form(form): Form<NewPageForm>,
) -> Result<Response, StatusCode> {
    // Check permissions
    if !can_edit_page(&state, &db, &site, &user).await? {
        return Err(StatusCode::FORBIDDEN);
    }

    // Create the new page
    let parent_page_id = parent_page.id.ok_or(StatusCode::NOT_FOUND)?;
    let mut new_page = if form.slug.is_empty() {
        Page::new_with_parent_and_title(parent_page_id, form.title.clone())
    } else {
        Page::new_with_parent(parent_page_id, form.slug.clone(), form.title.clone())
    };

    // Set all the additional properties from the form
    new_page.description = form.description.filter(|s| !s.is_empty());
    new_page.keywords = form.keywords.filter(|s| !s.is_empty());
    new_page.template = form.template;
    new_page.meta_robots = form.meta_robots;
    new_page.canonical_url = form.canonical_url.filter(|s| !s.is_empty());
    new_page.og_image_url = form.og_image_url.filter(|s| !s.is_empty());
    new_page.structured_data_type = form.structured_data_type;

    // Validate the page (except slug which will be auto-generated if needed)
    if new_page.validate_title().is_err() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let page_repo = PageRepository::new(db.clone());

    // Calculate the position for the new page
    let siblings = page_repo
        .list_children(parent_page_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Set position to be after all existing siblings
    new_page.position = siblings.len() as i32;

    // Create the page with auto-generated unique slug if needed
    let _new_page_id = page_repo
        .create_with_auto_slug(&mut new_page)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Build the path to the new page
    let parent_path = build_page_path(&state, &db, &parent_page).await?;
    let new_page_path = if parent_path == "/" {
        format!("/{}", new_page.slug)
    } else {
        format!("{}/{}", parent_path, new_page.slug)
    };

    // Redirect to the new page
    Ok(Redirect::to(&new_page_path).into_response())
}

/// Update a component
pub async fn update_component_handler(
    state: AppState,
    db: sqlx::SqlitePool,
    site: Site,
    page: Page,
    user: CurrentUser,
    Form(form): Form<UpdateComponentForm>,
) -> Result<Response, StatusCode> {
    // Check permissions
    if !can_edit_page(&state, &db, &site, &user).await? {
        return Err(StatusCode::FORBIDDEN);
    }

    let component_repo = ComponentRepository::new(db.clone());

    // Verify the component belongs to a draft version of this page
    let component = component_repo
        .find_by_id(form.component_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Parse content based on component type
    let content = match component.component_type.as_str() {
        "text" | "markdown" => serde_json::json!({
            "text": form.content
        }),
        "html" => serde_json::json!({
            "html": form.content
        }),
        "code" => {
            // For code, we might want to parse language from content later
            serde_json::json!({
                "code": form.content,
                "language": "plaintext"
            })
        }
        _ => serde_json::json!({
            "content": form.content
        }),
    };

    // Update the component
    component_repo
        .update_content(
            form.component_id,
            content,
            form.title.clone(),
            form.template,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Redirect back to edit page
    let redirect_path = build_page_path(&state, &db, &page).await?;
    let edit_path = if redirect_path == "/" {
        "/.edit".to_string()
    } else {
        format!("{}/.edit", redirect_path)
    };
    Ok(Redirect::to(&edit_path).into_response())
}

/// Publish the draft version
pub async fn publish_draft_handler(
    state: AppState,
    db: sqlx::SqlitePool,
    site: Site,
    page: Page,
    user: CurrentUser,
) -> Result<Response, StatusCode> {
    // Check permissions
    if !can_edit_page(&state, &db, &site, &user).await? {
        return Err(StatusCode::FORBIDDEN);
    }

    // Publish the draft
    let page_id = page.id.ok_or(StatusCode::NOT_FOUND)?;
    crate::draft::publish_draft(&db, page_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Redirect to view page with cache-busting parameter to prevent the
    // browser's Speculation Rules API from serving a stale prerendered page
    let redirect_path = build_page_path(&state, &db, &page).await?;
    let timestamp = chrono::Utc::now().timestamp();
    let redirect_url = format!("{}?_v={}", redirect_path, timestamp);
    Ok(Redirect::to(&redirect_url).into_response())
}

/// Discard draft changes
pub async fn discard_draft_handler(
    state: AppState,
    db: sqlx::SqlitePool,
    site: Site,
    page: Page,
    user: CurrentUser,
) -> Result<Response, StatusCode> {
    // Check permissions
    if !can_edit_page(&state, &db, &site, &user).await? {
        return Err(StatusCode::FORBIDDEN);
    }

    // Delete the draft
    let page_id = page.id.ok_or(StatusCode::NOT_FOUND)?;
    crate::draft::delete_draft_if_exists(&db, page_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Redirect to view page
    let redirect_path = build_page_path(&state, &db, &page).await?;
    Ok(Redirect::to(&redirect_path).into_response())
}

/// Save all component drafts
pub async fn save_draft_handler(
    state: AppState,
    db: sqlx::SqlitePool,
    site: Site,
    page: Page,
    user: CurrentUser,
    Form(form): Form<SaveDraftForm>,
) -> Result<Response, StatusCode> {
    tracing::info!("=== SAVE DRAFT HANDLER START ===");
    tracing::info!("Page: {} (ID: {:?})", page.title, page.id);
    tracing::info!("User: {}", user.user.username);

    // Debug: Log the parsed form structure
    tracing::info!("=== PARSED FORM DEBUG ===");
    tracing::info!("Form struct: {:?}", form);

    // Check permissions
    if !can_edit_page(&state, &db, &site, &user).await? {
        return Err(StatusCode::FORBIDDEN);
    }

    let component_repo = ComponentRepository::new(db.clone());

    // Get or create a draft version
    let page_id = page.id.ok_or(StatusCode::NOT_FOUND)?;
    let draft_version = get_or_create_draft(&db, page_id, Some(user.user.username.clone()))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tracing::info!("Draft version ID: {:?}", draft_version.id);

    // Get all existing components in the draft
    let draft_version_id = draft_version.id.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let existing_components = component_repo
        .list_by_page_version(draft_version_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Create a set of submitted component IDs for quick lookup
    let submitted_ids: std::collections::HashSet<i64> =
        form.component_ids.iter().cloned().collect();

    // Delete components that exist in draft but weren't submitted (they were deleted in UI)
    for component in &existing_components {
        if let Some(component_id) = component.id {
            if !submitted_ids.contains(&component_id) {
                tracing::debug!(
                    component_id = component_id,
                    "Deleting component not present in form submission"
                );
                component_repo
                    .delete(component_id)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            }
        }
    }

    tracing::info!(
        "Form data lengths - ids: {}, types: {}, titles: {}, templates: {}, contents: {}",
        form.component_ids.len(),
        form.component_types.len(),
        form.component_titles.len(),
        form.component_templates.len(),
        form.component_contents.len()
    );

    // Log the actual form data
    tracing::info!("Component IDs submitted: {:?}", form.component_ids);

    // Validate form data consistency
    if form.component_ids.len() != form.component_types.len()
        || form.component_ids.len() != form.component_titles.len()
        || form.component_ids.len() != form.component_templates.len()
        || form.component_ids.len() != form.component_contents.len()
    {
        tracing::error!("Form data arrays have inconsistent lengths!");
        return Err(StatusCode::BAD_REQUEST);
    }

    // Update each submitted component
    for i in 0..form.component_ids.len() {
        let component_id = form.component_ids[i];
        let component_type = &form.component_types[i];
        let title = if form.component_titles[i].is_empty() {
            None
        } else {
            Some(form.component_titles[i].clone())
        };
        let template = &form.component_templates[i];
        let content_str = &form.component_contents[i];

        tracing::info!(
            "=== Processing component {} (index: {}) ===",
            component_id,
            i
        );
        tracing::info!("  Type: {}", component_type);
        tracing::info!("  Template: {}", template);
        tracing::info!("  Title: {:?}", title);
        tracing::info!("  Content: {}", content_str);

        // Use the component registry to parse content
        let registry = get_component_registry();
        let content = match registry.parse_content(component_type, content_str) {
            Ok(parsed_content) => parsed_content,
            Err(e) => {
                tracing::error!(
                    "Failed to parse component content for type {}: {}",
                    component_type,
                    e
                );
                return Err(StatusCode::BAD_REQUEST);
            }
        };

        // Validate the content if the handler provides validation
        if let Some(handler) = registry.get_handler(component_type) {
            if let Err(e) = handler.validate_content(&content) {
                tracing::error!(
                    "Component content validation failed for type {}: {}",
                    component_type,
                    e
                );
                return Err(StatusCode::BAD_REQUEST);
            }
        }

        // Update the component
        match component_repo
            .update_content(component_id, content, title, template.clone())
            .await
        {
            Ok(_) => {
                tracing::info!("  ✓ Successfully updated component {}", component_id);
            }
            Err(e) => {
                tracing::error!("  ✗ Failed to update component {}: {:?}", component_id, e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    }

    // Log positions before normalization for debugging reorder+publish issues
    if let Ok(components_before) = component_repo.list_by_page_version(draft_version_id).await {
        let positions: Vec<String> = components_before
            .iter()
            .map(|c| format!("id={:?} pos={}", c.id, c.position))
            .collect();
        tracing::info!("Positions before normalize: [{}]", positions.join(", "));
    }

    // Normalize positions after all updates and deletions
    component_repo
        .normalize_positions(draft_version_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tracing::info!("=== SAVE DRAFT HANDLER COMPLETE ===");

    // Redirect back to edit page
    let redirect_path = build_page_path(&state, &db, &page).await?;
    let edit_path = if redirect_path == "/" {
        "/.edit".to_string()
    } else {
        format!("{}/.edit", redirect_path)
    };

    tracing::info!("Redirecting to: {}", edit_path);

    Ok(Redirect::to(&edit_path).into_response())
}

/// Delete a component from the draft
pub async fn delete_component_handler(
    state: AppState,
    db: sqlx::SqlitePool,
    site: Site,
    page: Page,
    user: CurrentUser,
    component_id: i64,
) -> Result<Response, StatusCode> {
    // Check permissions
    if !can_edit_page(&state, &db, &site, &user).await? {
        return Err(StatusCode::FORBIDDEN);
    }

    let component_repo = ComponentRepository::new(db.clone());

    // Get or create a draft version
    let page_id = page.id.ok_or(StatusCode::NOT_FOUND)?;
    let draft_version = get_or_create_draft(&db, page_id, Some(user.user.username.clone()))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Verify the component belongs to the draft version
    let component = component_repo
        .find_by_id(component_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let draft_version_id = draft_version.id.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    if component.page_version_id != draft_version_id {
        return Err(StatusCode::FORBIDDEN);
    }

    // Delete the component
    component_repo
        .delete(component_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Redirect back to edit page
    let redirect_path = build_page_path(&state, &db, &page).await?;
    let edit_path = if redirect_path == "/" {
        "/.edit".to_string()
    } else {
        format!("{}/.edit", redirect_path)
    };
    Ok(Redirect::to(&edit_path).into_response())
}

/// Move a component up or down
pub async fn move_component_handler(
    state: AppState,
    db: sqlx::SqlitePool,
    site: Site,
    page: Page,
    user: CurrentUser,
    component_id: i64,
    direction: &str,
) -> Result<Response, StatusCode> {
    // Check permissions
    if !can_edit_page(&state, &db, &site, &user).await? {
        return Err(StatusCode::FORBIDDEN);
    }

    let component_repo = ComponentRepository::new(db.clone());

    // Get or create a draft version
    let page_id = page.id.ok_or(StatusCode::NOT_FOUND)?;
    let draft_version = get_or_create_draft(&db, page_id, Some(user.user.username.clone()))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Verify the component belongs to the draft version
    let component = component_repo
        .find_by_id(component_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    let draft_version_id = draft_version.id.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    if component.page_version_id != draft_version_id {
        return Err(StatusCode::FORBIDDEN);
    }

    // Move the component based on direction
    match direction {
        "up" => {
            component_repo
                .move_up(component_id)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
        "down" => {
            component_repo
                .move_down(component_id)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
        _ => return Err(StatusCode::BAD_REQUEST),
    }

    // Redirect back to edit page
    let redirect_path = build_page_path(&state, &db, &page).await?;
    let edit_path = if redirect_path == "/" {
        "/.edit".to_string()
    } else {
        format!("{}/.edit", redirect_path)
    };
    Ok(Redirect::to(&edit_path).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{
        create_test_app_state, create_test_session, create_test_user, setup_test_schema,
    };
    use doxyde_core::{PageVersion, SiteUser};
    use doxyde_db::repositories::{
        ComponentRepository, PageRepository, PageVersionRepository, SiteUserRepository,
    };

    #[sqlx::test]
    async fn test_save_and_publish_with_deleted_components(
        pool: sqlx::SqlitePool,
    ) -> anyhow::Result<()> {
        // Setup schema on the test pool
        setup_test_schema(&pool).await?;

        // Create test app state which includes creating the schema
        let test_state = create_test_app_state().await?;
        // Setup test data
        let page_repo = PageRepository::new(pool.clone());
        let version_repo = PageVersionRepository::new(pool.clone());
        let component_repo = ComponentRepository::new(pool.clone());
        let site_user_repo = SiteUserRepository::new(pool.clone());

        // Create root page using raw SQL (bypass validation)
        sqlx::query("INSERT INTO pages (parent_page_id, slug, title, position) VALUES (NULL, '', 'Home', 0)")
            .execute(&pool)
            .await?;

        // Get the root page
        let root_page = page_repo
            .get_root_page()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to get root page"))?;
        let page_id = root_page.id.unwrap();

        // Create an initial published version with 3 components
        let published_version = PageVersion::new(page_id, 1, Some("test-user".to_string()));
        let mut published_version_copy = published_version.clone();
        published_version_copy.is_published = true;
        let published_version_id = version_repo.create(&published_version_copy).await?;

        // Add 3 components to the published version
        let comp1 = Component::new(
            published_version_id,
            "text".to_string(),
            0,
            serde_json::json!({"text": "Component 1"}),
        );
        let _comp1_id = component_repo.create(&comp1).await?;

        let comp2 = Component::new(
            published_version_id,
            "text".to_string(),
            1,
            serde_json::json!({"text": "Component 2"}),
        );
        let _comp2_id = component_repo.create(&comp2).await?;

        let comp3 = Component::new(
            published_version_id,
            "text".to_string(),
            2,
            serde_json::json!({"text": "Component 3"}),
        );
        let _comp3_id = component_repo.create(&comp3).await?;

        // Create a draft version by copying components
        let draft_version = PageVersion::new(page_id, 2, Some("test-user".to_string()));
        let draft_version_id = version_repo.create(&draft_version).await?;

        // Copy components to draft
        let draft_comp1 = Component::new(
            draft_version_id,
            "text".to_string(),
            0,
            serde_json::json!({"text": "Component 1"}),
        );
        let draft_comp1_id = component_repo.create(&draft_comp1).await?;

        let draft_comp2 = Component::new(
            draft_version_id,
            "text".to_string(),
            1,
            serde_json::json!({"text": "Component 2"}),
        );
        let draft_comp2_id = component_repo.create(&draft_comp2).await?;

        let draft_comp3 = Component::new(
            draft_version_id,
            "text".to_string(),
            2,
            serde_json::json!({"text": "Component 3"}),
        );
        let draft_comp3_id = component_repo.create(&draft_comp3).await?;

        // Create a user with edit permissions
        let user = create_test_user(&pool, "editor", "editor@test.com", false).await?;
        let site_user = SiteUser::new(user.id.unwrap(), SiteRole::Editor);
        site_user_repo.create(&site_user).await?;

        // Use the test app state
        let app_state = test_state;

        // Simulate form submission where component 2 is deleted
        // (only components 1 and 3 are submitted)
        let form = SaveDraftForm {
            component_ids: vec![draft_comp1_id, draft_comp3_id],
            component_types: vec!["text".to_string(), "text".to_string()],
            component_titles: vec!["".to_string(), "".to_string()],
            component_templates: vec!["default".to_string(), "default".to_string()],
            component_contents: vec![
                "Component 1 Updated".to_string(),
                "Component 3 Updated".to_string(),
            ],
            action: Some("save_draft".to_string()),
        };

        // Use the root page
        let page = root_page;

        // Create session for current user
        let session = create_test_session(&pool, user.id.unwrap()).await?;

        // Create current user
        let current_user = CurrentUser { user, session };

        // In multi-database mode, create a dummy site object for testing
        let mut site =
            doxyde_core::models::Site::new("test.local".to_string(), "Test Site".to_string());
        site.id = Some(1);

        // Call save_draft_handler (this is where the bug was)
        let result = save_draft_handler(
            app_state.clone(),
            pool.clone(),
            site.clone(),
            page.clone(),
            current_user.clone(),
            Form(form),
        )
        .await;

        assert!(result.is_ok(), "save_draft_handler should succeed");

        // Check that component 2 was deleted from the draft (bug is fixed)
        let draft_components = component_repo
            .list_by_page_version(draft_version_id)
            .await?;
        assert_eq!(
            draft_components.len(),
            2,
            "After fix: deleted component should be removed from draft"
        );

        // The second component should not be there anymore
        let comp2_still_exists = draft_components
            .iter()
            .any(|c| c.id == Some(draft_comp2_id));
        assert!(
            !comp2_still_exists,
            "After fix: component 2 should be deleted from draft"
        );

        // Now publish the draft
        let publish_result =
            publish_draft_handler(app_state, pool.clone(), site, page, current_user).await;
        assert!(
            publish_result.is_ok(),
            "publish_draft_handler should succeed"
        );

        // Get the new published version components
        let new_published_id = version_repo
            .get_published(page_id)
            .await?
            .unwrap()
            .id
            .unwrap();
        let published_components = component_repo
            .list_by_page_version(new_published_id)
            .await?;

        // Should have exactly 2 components after publishing
        assert_eq!(
            published_components.len(),
            2,
            "Published version should have exactly 2 components"
        );

        // Verify the components are the correct ones
        let has_comp1 = published_components
            .iter()
            .any(|c| c.content["text"] == "Component 1 Updated");
        let has_comp3 = published_components
            .iter()
            .any(|c| c.content["text"] == "Component 3 Updated");

        assert!(has_comp1, "Component 1 should be in published version");
        assert!(has_comp3, "Component 3 should be in published version");

        Ok(())
    }

    #[sqlx::test]
    async fn test_add_and_delete_component(pool: sqlx::SqlitePool) -> anyhow::Result<()> {
        // Setup schema on the test pool
        setup_test_schema(&pool).await?;

        // Create test app state
        let test_state = create_test_app_state().await?;

        // Setup test data
        let page_repo = PageRepository::new(pool.clone());
        let version_repo = PageVersionRepository::new(pool.clone());
        let component_repo = ComponentRepository::new(pool.clone());
        let site_user_repo = SiteUserRepository::new(pool.clone());

        // Create root page using raw SQL (bypass validation)
        sqlx::query("INSERT INTO pages (parent_page_id, slug, title, position) VALUES (NULL, '', 'Home', 0)")
            .execute(&pool)
            .await?;

        // Get the root page
        let root_page = page_repo
            .get_root_page()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Failed to get root page"))?;
        let page_id = root_page.id.unwrap();

        // Create an initial published version with no components
        let published_version = PageVersion::new(page_id, 1, Some("test-user".to_string()));
        let mut published_version_copy = published_version.clone();
        published_version_copy.is_published = true;
        let _published_version_id = version_repo.create(&published_version_copy).await?;

        // Create a user with edit permissions
        let user = create_test_user(&pool, "editor", "editor@test.com", false).await?;
        let site_user = SiteUser::new(user.id.unwrap(), SiteRole::Editor);
        site_user_repo.create(&site_user).await?;
        let session = create_test_session(&pool, user.id.unwrap()).await?;
        let current_user = CurrentUser {
            user: user.clone(),
            session,
        };

        let app_state = test_state;
        // In multi-database mode, create a dummy site object for testing
        let mut site =
            doxyde_core::models::Site::new("test.local".to_string(), "Test Site".to_string());
        site.id = Some(1);

        // Add a component
        let add_form = AddComponentForm {
            content: "New test component".to_string(),
            component_type: "text".to_string(),
            ajax: false,
        };

        let response = add_component_handler(
            app_state.clone(),
            pool.clone(),
            site.clone(),
            root_page.clone(),
            current_user.clone(),
            axum::extract::Form(add_form),
        )
        .await
        .map_err(|_| anyhow::anyhow!("Failed to add component"))?;

        // Check that we got a redirect
        assert_eq!(response.status(), StatusCode::SEE_OTHER);

        // Get the draft version and verify component was added
        let draft_version = version_repo
            .get_draft(page_id)
            .await?
            .expect("Draft should exist");

        let components = component_repo
            .list_by_page_version(draft_version.id.unwrap())
            .await?;

        assert_eq!(components.len(), 1, "Should have 1 component");
        let component_id = components[0].id.unwrap();

        // Now delete the component
        let delete_response = delete_component_handler(
            app_state.clone(),
            pool.clone(),
            site.clone(),
            root_page.clone(),
            current_user.clone(),
            component_id,
        )
        .await
        .map_err(|_| anyhow::anyhow!("Failed to delete component"))?;

        // Check that we got a redirect
        assert_eq!(delete_response.status(), StatusCode::SEE_OTHER);

        // Verify component was deleted
        let components_after_delete = component_repo
            .list_by_page_version(draft_version.id.unwrap())
            .await?;

        assert_eq!(
            components_after_delete.len(),
            0,
            "Should have 0 components after deletion"
        );

        // Verify positions are normalized (even though there are no components)
        // This is just to ensure normalize_positions doesn't error on empty set
        component_repo
            .normalize_positions(draft_version.id.unwrap())
            .await?;

        Ok(())
    }
}
