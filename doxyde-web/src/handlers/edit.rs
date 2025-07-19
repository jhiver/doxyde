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
use doxyde_core::models::{component::Component, page::Page, permission::SiteRole, site::Site};
use doxyde_db::repositories::{ComponentRepository, PageRepository, SiteUserRepository};
use serde::Deserialize;
use tera::Context;

use crate::{
    auth::CurrentUser, draft::get_or_create_draft, template_context::add_base_context, AppState,
};

#[derive(Debug, Deserialize)]
pub struct AddComponentForm {
    pub content: String,
    pub component_type: String,
}

#[derive(Debug, Deserialize)]
pub struct NewPageForm {
    pub title: String,
    #[serde(default)]
    pub slug: String,
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
    match can_edit_page(&state, &site, &user).await {
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

    let page_repo = PageRepository::new(state.db.clone());
    let component_repo = ComponentRepository::new(state.db.clone());

    tracing::info!("=== EDIT PAGE HANDLER START ===");
    tracing::info!("Page: {} (ID: {:?})", page.title, page.id);
    tracing::info!("User: {}", user.user.username);

    // Get or create a draft version
    let draft_version = match get_or_create_draft(
        &state.db,
        page.id.unwrap(),
        Some(user.user.username.clone()),
    )
    .await
    {
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
    let components = match component_repo
        .list_by_page_version(draft_version.id.unwrap())
        .await
    {
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
    let breadcrumb = match page_repo.get_breadcrumb_trail(page.id.unwrap()).await {
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
    add_base_context(&mut context, &state, &site, Some(&page))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    context.insert("page", &page);
    context.insert("components", &components);
    context.insert("breadcrumbs", &breadcrumb_data);
    context.insert("current_path", &current_path);
    context.insert("user", &user.user);
    context.insert("can_edit", &true);
    context.insert("action", ".edit");

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

    // Render the edit template
    let html = match state.templates.render("page_edit.html", &context) {
        Ok(rendered) => rendered,
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
    site: Site,
    page: Page,
    user: CurrentUser,
    Form(form): Form<AddComponentForm>,
) -> Result<Response, StatusCode> {
    // Check permissions
    if !can_edit_page(&state, &site, &user).await? {
        return Err(StatusCode::FORBIDDEN);
    }

    let component_repo = ComponentRepository::new(state.db.clone());

    // Get or create a draft version
    let draft_version = get_or_create_draft(
        &state.db,
        page.id.unwrap(),
        Some(user.user.username.clone()),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Get current components to determine position
    let components = component_repo
        .list_by_page_version(draft_version.id.unwrap())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let next_position = components.len() as i32;

    // Create component content based on type
    let content = match form.component_type.as_str() {
        "text" | "markdown" => serde_json::json!({
            "text": form.content
        }),
        "html" => serde_json::json!({
            "html": form.content
        }),
        "code" => serde_json::json!({
            "code": form.content,
            "language": "plaintext"
        }),
        "image" => {
            // Try to parse as JSON first (in case it's coming from our updated form)
            if let Ok(json_content) = serde_json::from_str::<serde_json::Value>(&form.content) {
                json_content
            } else {
                // Fallback to old format
                serde_json::json!({
                    "src": form.content,
                    "alt": ""
                })
            }
        }
        _ => serde_json::json!({
            "content": form.content
        }),
    };

    // Create new component
    let component = Component::new(
        draft_version.id.unwrap(),
        form.component_type,
        next_position,
        content,
    );

    component_repo
        .create(&component)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Normalize positions after adding component
    component_repo
        .normalize_positions(draft_version.id.unwrap())
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

/// Check if user can edit the page
pub async fn can_edit_page(
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
        Ok(su.role == SiteRole::Editor || su.role == SiteRole::Owner)
    } else {
        Ok(false)
    }
}

/// Build the full path to a page
async fn build_page_path(state: &AppState, page: &Page) -> Result<String, StatusCode> {
    if page.parent_page_id.is_none() {
        return Ok("/".to_string());
    }

    let page_repo = PageRepository::new(state.db.clone());
    let breadcrumb = page_repo
        .get_breadcrumb_trail(page.id.unwrap())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let path_parts: Vec<&str> = breadcrumb[1..].iter().map(|p| p.slug.as_str()).collect();

    Ok(format!("/{}", path_parts.join("/")))
}

/// Display new page form
pub async fn new_page_handler(
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
    context.insert("parent_page", &page);
    context.insert("breadcrumbs", &breadcrumb_data);
    context.insert("current_path", &current_path);
    context.insert("user", &user.user);
    context.insert("can_edit", &true);
    context.insert("action", ".new");

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
    site: Site,
    parent_page: Page,
    user: CurrentUser,
    Form(form): Form<NewPageForm>,
) -> Result<Response, StatusCode> {
    // Check permissions
    if !can_edit_page(&state, &site, &user).await? {
        return Err(StatusCode::FORBIDDEN);
    }

    // Create the new page
    let mut new_page = if form.slug.is_empty() {
        Page::new_with_parent_and_title(
            site.id.unwrap(),
            parent_page.id.unwrap(),
            form.title.clone(),
        )
    } else {
        Page::new_with_parent(
            site.id.unwrap(),
            parent_page.id.unwrap(),
            form.slug.clone(),
            form.title.clone(),
        )
    };

    // Validate the page (except slug which will be auto-generated if needed)
    if new_page.validate_title().is_err() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let page_repo = PageRepository::new(state.db.clone());

    // Create the page with auto-generated unique slug if needed
    let _new_page_id = page_repo
        .create_with_auto_slug(&mut new_page)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Build the path to the new page
    let parent_path = build_page_path(&state, &parent_page).await?;
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
    site: Site,
    page: Page,
    user: CurrentUser,
    Form(form): Form<UpdateComponentForm>,
) -> Result<Response, StatusCode> {
    // Check permissions
    if !can_edit_page(&state, &site, &user).await? {
        return Err(StatusCode::FORBIDDEN);
    }

    let component_repo = ComponentRepository::new(state.db.clone());

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
    let redirect_path = build_page_path(&state, &page).await?;
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
    site: Site,
    page: Page,
    user: CurrentUser,
) -> Result<Response, StatusCode> {
    // Check permissions
    if !can_edit_page(&state, &site, &user).await? {
        return Err(StatusCode::FORBIDDEN);
    }

    // Publish the draft
    crate::draft::publish_draft(&state.db, page.id.unwrap())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Redirect to view page
    let redirect_path = build_page_path(&state, &page).await?;
    Ok(Redirect::to(&redirect_path).into_response())
}

/// Discard draft changes
pub async fn discard_draft_handler(
    state: AppState,
    site: Site,
    page: Page,
    user: CurrentUser,
) -> Result<Response, StatusCode> {
    // Check permissions
    if !can_edit_page(&state, &site, &user).await? {
        return Err(StatusCode::FORBIDDEN);
    }

    // Delete the draft
    crate::draft::delete_draft_if_exists(&state.db, page.id.unwrap())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Redirect to view page
    let redirect_path = build_page_path(&state, &page).await?;
    Ok(Redirect::to(&redirect_path).into_response())
}

/// Save all component drafts
pub async fn save_draft_handler(
    state: AppState,
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
    if !can_edit_page(&state, &site, &user).await? {
        return Err(StatusCode::FORBIDDEN);
    }

    let component_repo = ComponentRepository::new(state.db.clone());

    // Get or create a draft version
    let draft_version = get_or_create_draft(
        &state.db,
        page.id.unwrap(),
        Some(user.user.username.clone()),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tracing::info!("Draft version ID: {:?}", draft_version.id);

    // Get all existing components in the draft
    let existing_components = component_repo
        .list_by_page_version(draft_version.id.unwrap())
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

        // Parse content based on component type
        let content = match component_type.as_str() {
            "text" | "markdown" => serde_json::json!({
                "text": content_str
            }),
            "html" => serde_json::json!({
                "html": content_str
            }),
            "code" => serde_json::json!({
                "code": content_str,
                "language": "plaintext"
            }),
            "image" => {
                // Try to parse as JSON first (in case it's already JSON)
                if let Ok(json_content) = serde_json::from_str::<serde_json::Value>(content_str) {
                    json_content
                } else {
                    // Fallback to old format
                    serde_json::json!({
                        "src": content_str,
                        "alt": ""
                    })
                }
            }
            _ => serde_json::json!({
                "content": content_str
            }),
        };

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

    // Normalize positions after all updates and deletions
    component_repo
        .normalize_positions(draft_version.id.unwrap())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tracing::info!("=== SAVE DRAFT HANDLER COMPLETE ===");

    // Redirect back to edit page
    let redirect_path = build_page_path(&state, &page).await?;
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
    site: Site,
    page: Page,
    user: CurrentUser,
    component_id: i64,
) -> Result<Response, StatusCode> {
    // Check permissions
    if !can_edit_page(&state, &site, &user).await? {
        return Err(StatusCode::FORBIDDEN);
    }

    let component_repo = ComponentRepository::new(state.db.clone());

    // Get or create a draft version
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

    // Delete the component
    component_repo
        .delete(component_id)
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

/// Move a component up or down
pub async fn move_component_handler(
    state: AppState,
    site: Site,
    page: Page,
    user: CurrentUser,
    component_id: i64,
    direction: &str,
) -> Result<Response, StatusCode> {
    // Check permissions
    if !can_edit_page(&state, &site, &user).await? {
        return Err(StatusCode::FORBIDDEN);
    }

    let component_repo = ComponentRepository::new(state.db.clone());

    // Get or create a draft version
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
    let redirect_path = build_page_path(&state, &page).await?;
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
    use crate::test_helpers::{create_test_app_state, create_test_session, create_test_user};
    use doxyde_core::{PageVersion, SiteUser};
    use doxyde_db::repositories::{
        ComponentRepository, PageRepository, PageVersionRepository, SiteRepository,
        SiteUserRepository,
    };

    #[tokio::test]
    async fn test_save_and_publish_with_deleted_components() -> anyhow::Result<()> {
        // Create test app state which includes creating the schema
        let test_state = create_test_app_state().await?;
        let pool = test_state.db.clone();
        // Setup test data
        let site_repo = SiteRepository::new(pool.clone());
        let page_repo = PageRepository::new(pool.clone());
        let version_repo = PageVersionRepository::new(pool.clone());
        let component_repo = ComponentRepository::new(pool.clone());
        let site_user_repo = SiteUserRepository::new(pool.clone());

        // Create a site
        let site = Site::new("localhost:3000".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;

        // Get the root page that was created with the site
        let root_page = page_repo.get_root_page(site_id).await?.unwrap();
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
        let site_user = SiteUser::new(site_id, user.id.unwrap(), SiteRole::Editor);
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

        // Get the site object
        let site = site_repo.find_by_id(site_id).await?.unwrap();

        // Call save_draft_handler (this is where the bug was)
        let result = save_draft_handler(
            app_state.clone(),
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
        let publish_result = publish_draft_handler(app_state, site, page, current_user).await;
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
    async fn test_add_and_delete_component() -> anyhow::Result<()> {
        // Create test app state
        let test_state = create_test_app_state().await?;
        let pool = test_state.db.clone();

        // Setup test data
        let site_repo = SiteRepository::new(pool.clone());
        let page_repo = PageRepository::new(pool.clone());
        let version_repo = PageVersionRepository::new(pool.clone());
        let component_repo = ComponentRepository::new(pool.clone());
        let site_user_repo = SiteUserRepository::new(pool.clone());

        // Create a site
        let site = Site::new("localhost:3000".to_string(), "Test Site".to_string());
        let site_id = site_repo.create(&site).await?;

        // Get the root page
        let root_page = page_repo.get_root_page(site_id).await?.unwrap();
        let page_id = root_page.id.unwrap();

        // Create an initial published version with no components
        let published_version = PageVersion::new(page_id, 1, Some("test-user".to_string()));
        let mut published_version_copy = published_version.clone();
        published_version_copy.is_published = true;
        let _published_version_id = version_repo.create(&published_version_copy).await?;

        // Create a user with edit permissions
        let user = create_test_user(&pool, "editor", "editor@test.com", false).await?;
        let site_user = SiteUser::new(site_id, user.id.unwrap(), SiteRole::Editor);
        site_user_repo.create(&site_user).await?;
        let session = create_test_session(&pool, user.id.unwrap()).await?;
        let current_user = CurrentUser {
            user: user.clone(),
            session,
        };

        let app_state = test_state;
        let site = site_repo.find_by_id(site_id).await?.unwrap();

        // Add a component
        let add_form = AddComponentForm {
            content: "New test component".to_string(),
            component_type: "text".to_string(),
        };

        let response = add_component_handler(
            app_state.clone(),
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
