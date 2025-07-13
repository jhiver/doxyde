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

use crate::{auth::CurrentUser, draft::get_or_create_draft, template_context::add_base_context, AppState};

#[derive(Debug, Deserialize)]
pub struct AddComponentForm {
    pub content: String,
    pub component_type: String,
}

#[derive(Debug, Deserialize)]
pub struct NewPageForm {
    pub title: String,
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

    // Get or create a draft version
    let draft_version = match get_or_create_draft(
        &state.db,
        page.id.unwrap(),
        Some(user.user.username.clone()),
    )
    .await
    {
        Ok(draft) => draft,
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
        Ok(comps) => comps,
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
    
    // Add base context (site_title, root_page_title, logo data)
    add_base_context(&mut context, &state, &site)
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
    
    // Add base context (site_title, root_page_title, logo data)
    add_base_context(&mut context, &state, &site)
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
    let new_page = Page::new_with_parent(
        site.id.unwrap(),
        parent_page.id.unwrap(),
        form.slug.clone(),
        form.title.clone(),
    );

    // Validate the page
    if let Err(_e) = new_page.is_valid() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let page_repo = PageRepository::new(state.db.clone());

    // Check if slug already exists under this parent
    let children = page_repo
        .list_children(parent_page.id.unwrap())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if children.iter().any(|child| child.slug == form.slug) {
        return Err(StatusCode::CONFLICT); // Slug already exists
    }

    // Create the page
    let _new_page_id = page_repo
        .create(&new_page)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Build the path to the new page
    let parent_path = build_page_path(&state, &parent_page).await?;
    let new_page_path = if parent_path == "/" {
        format!("/{}", form.slug)
    } else {
        format!("{}/{}", parent_path, form.slug)
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
    // Check permissions
    if !can_edit_page(&state, &site, &user).await? {
        return Err(StatusCode::FORBIDDEN);
    }

    let component_repo = ComponentRepository::new(state.db.clone());

    // Get or create a draft version
    let _draft_version = get_or_create_draft(
        &state.db,
        page.id.unwrap(),
        Some(user.user.username.clone()),
    )
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Update each component
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
        component_repo
            .update_content(component_id, content, title, template.clone())
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
