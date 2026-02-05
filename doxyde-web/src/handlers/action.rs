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
    extract::State,
    http::{StatusCode, Uri},
    response::{IntoResponse, Response},
};
use axum_extra::extract::Host;
use doxyde_core::models::{Page, Site};
use doxyde_db::repositories::PageRepository;

use crate::{
    auth::CurrentUser,
    content::ContentPath,
    handlers::{
        delete_page::DeletePageForm,
        edit::{AddComponentForm, NewPageForm, SaveDraftForm},
        move_page::MovePageForm,
        properties::PagePropertiesForm,
    },
    AppState,
};

/// Handle POST requests to action URLs
pub async fn handle_action(
    Host(host): Host,
    uri: Uri,
    State(state): State<AppState>,
    db: sqlx::SqlitePool,
    user: CurrentUser,
    body: String,
) -> Result<Response, StatusCode> {
    let path = uri.path();
    let content_path = ContentPath::parse(path);

    tracing::info!(
        "handle_action called - path: {}, action: {:?}, body length: {}",
        path,
        content_path.action,
        body.len()
    );

    let site = resolve_site(&db, &host).await?;
    let page = resolve_page(&db, &site, &content_path).await?;

    route_to_handler(state, db, site, page, user, body, content_path).await
}

/// Route to the appropriate handler based on action
async fn route_to_handler(
    state: AppState,
    db: sqlx::SqlitePool,
    site: Site,
    page: Page,
    user: CurrentUser,
    body: String,
    content_path: ContentPath,
) -> Result<Response, StatusCode> {
    match content_path.action.as_deref() {
        Some(".edit") | Some(".content") => {
            handle_edit_action(state, db, site, page, user, body).await
        }
        Some(".new") => handle_new_page(state, db, site, page, user, body).await,
        Some(".properties") => handle_properties(state, db, site, page, user, body).await,
        Some(".move") => handle_move_page(state, db, site, page, user, body).await,
        Some(".delete") => handle_delete_page(state, db, site, page, user, body).await,
        Some(".reorder") => handle_reorder(state, db, site, page, user, body, content_path).await,
        _ => Err(StatusCode::NOT_FOUND),
    }
}

/// Handle edit/content action
async fn handle_edit_action(
    state: AppState,
    db: sqlx::SqlitePool,
    site: Site,
    page: Page,
    user: CurrentUser,
    body: String,
) -> Result<Response, StatusCode> {
    let form_data = parse_form_data(&body)?;
    let action = extract_action(&form_data);

    match action.as_deref() {
        Some("save_draft") | Some("publish_draft") => {
            handle_save_or_publish(state, db, site, page, user, form_data, &action).await
        }
        Some("discard_draft") => handle_discard_draft(state, db, site, page, user).await,
        Some("add_component") => handle_add_component(state, db, site, page, user, form_data).await,
        Some("delete_component") => {
            handle_delete_component(state, db, site, page, user, form_data).await
        }
        Some("move_component") => {
            handle_move_component(state, db, site, page, user, form_data).await
        }
        _ => Err(StatusCode::BAD_REQUEST),
    }
}

/// Handle save_draft or publish_draft actions
async fn handle_save_or_publish(
    state: AppState,
    db: sqlx::SqlitePool,
    site: Site,
    page: Page,
    user: CurrentUser,
    form_data: Vec<(String, String)>,
    action: &Option<String>,
) -> Result<Response, StatusCode> {
    tracing::info!("Processing save_draft/publish_draft action");

    let save_form = parse_save_draft_form(&form_data)?;

    if action.as_deref() == Some("save_draft") {
        save_draft(state, db, site, page, user, save_form).await
    } else {
        save_and_publish(state, db, site, page, user, save_form).await
    }
}

/// Save draft
async fn save_draft(
    state: AppState,
    db: sqlx::SqlitePool,
    site: Site,
    page: Page,
    user: CurrentUser,
    save_form: SaveDraftForm,
) -> Result<Response, StatusCode> {
    crate::handlers::save_draft_handler(state, db, site, page, user, axum::extract::Form(save_form))
        .await
}

/// Save and publish draft
async fn save_and_publish(
    state: AppState,
    db: sqlx::SqlitePool,
    site: Site,
    page: Page,
    user: CurrentUser,
    save_form: SaveDraftForm,
) -> Result<Response, StatusCode> {
    // First save the draft
    crate::handlers::save_draft_handler(
        state.clone(),
        db.clone(),
        site.clone(),
        page.clone(),
        user.clone(),
        axum::extract::Form(save_form),
    )
    .await?;

    // Then publish it
    crate::handlers::publish_draft_handler(state, db, site, page, user).await
}

/// Handle discard draft
async fn handle_discard_draft(
    state: AppState,
    db: sqlx::SqlitePool,
    site: Site,
    page: Page,
    user: CurrentUser,
) -> Result<Response, StatusCode> {
    crate::handlers::discard_draft_handler(state, db, site, page, user).await
}

/// Handle add component
async fn handle_add_component(
    state: AppState,
    db: sqlx::SqlitePool,
    site: Site,
    page: Page,
    user: CurrentUser,
    form_data: Vec<(String, String)>,
) -> Result<Response, StatusCode> {
    let add_form = parse_add_component_form(&form_data);

    crate::handlers::add_component_handler(
        state,
        db,
        site,
        page,
        user,
        axum::extract::Form(add_form),
    )
    .await
}

/// Handle delete component
async fn handle_delete_component(
    state: AppState,
    db: sqlx::SqlitePool,
    site: Site,
    page: Page,
    user: CurrentUser,
    form_data: Vec<(String, String)>,
) -> Result<Response, StatusCode> {
    let component_id = extract_component_id(&form_data, "delete_component_id")?;

    crate::handlers::delete_component_handler(state, db, site, page, user, component_id).await
}

/// Handle move component
async fn handle_move_component(
    state: AppState,
    db: sqlx::SqlitePool,
    site: Site,
    page: Page,
    user: CurrentUser,
    form_data: Vec<(String, String)>,
) -> Result<Response, StatusCode> {
    // First save all components
    let save_form = parse_save_draft_form(&form_data)?;

    crate::handlers::save_draft_handler(
        state.clone(),
        db.clone(),
        site.clone(),
        page.clone(),
        user.clone(),
        axum::extract::Form(save_form),
    )
    .await?;

    // Then move the component
    let move_component_id = extract_component_id(&form_data, "move_component_id")?;
    let move_direction = extract_field(&form_data, "move_direction")?;

    crate::handlers::move_component_handler(
        state,
        db,
        site,
        page,
        user,
        move_component_id,
        move_direction.as_str(),
    )
    .await
}

/// Handle new page creation
async fn handle_new_page(
    state: AppState,
    db: sqlx::SqlitePool,
    site: Site,
    page: Page,
    user: CurrentUser,
    body: String,
) -> Result<Response, StatusCode> {
    let new_form: NewPageForm = parse_form(&body)?;

    crate::handlers::create_page_handler(state, db, site, page, user, axum::extract::Form(new_form))
        .await
}

/// Handle properties update
async fn handle_properties(
    state: AppState,
    db: sqlx::SqlitePool,
    site: Site,
    page: Page,
    user: CurrentUser,
    body: String,
) -> Result<Response, StatusCode> {
    let props_form: PagePropertiesForm = parse_form(&body)?;

    crate::handlers::update_page_properties_handler(
        state,
        db,
        site,
        page,
        user,
        axum::extract::Form(props_form),
    )
    .await
}

/// Handle page move
async fn handle_move_page(
    state: AppState,
    db: sqlx::SqlitePool,
    site: Site,
    page: Page,
    user: CurrentUser,
    body: String,
) -> Result<Response, StatusCode> {
    let move_form: MovePageForm = parse_form(&body)?;

    crate::handlers::do_move_page_handler(
        state,
        db,
        site,
        page,
        user,
        axum::extract::Form(move_form),
    )
    .await
}

/// Handle page deletion
async fn handle_delete_page(
    state: AppState,
    db: sqlx::SqlitePool,
    site: Site,
    page: Page,
    user: CurrentUser,
    body: String,
) -> Result<Response, StatusCode> {
    let delete_form: DeletePageForm = parse_form(&body)?;

    crate::handlers::do_delete_page_handler(
        state,
        db,
        site,
        page,
        user,
        axum::extract::Form(delete_form),
    )
    .await
}

/// Handle reorder pages
async fn handle_reorder(
    state: AppState,
    db: sqlx::SqlitePool,
    site: Site,
    page: Page,
    user: CurrentUser,
    body: String,
    content_path: ContentPath,
) -> Result<Response, StatusCode> {
    let form_data = parse_form_data(&body)?;
    let sort_mode = extract_sort_mode(&form_data);
    let positions = extract_positions(&form_data, &sort_mode);

    crate::handlers::update_page_order_handler(
        State(state.into()),
        db,
        site,
        page.clone(),
        user,
        sort_mode,
        positions,
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to update page order");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Redirect back to the page
    let redirect_path = build_redirect_path(&content_path);
    Ok(axum::response::Redirect::to(&redirect_path).into_response())
}

// Helper functions

/// Parse form data from body
fn parse_form_data(body: &str) -> Result<Vec<(String, String)>, StatusCode> {
    serde_urlencoded::from_str(body).map_err(|e| {
        tracing::error!(error = %e, "Failed to parse form data");
        StatusCode::BAD_REQUEST
    })
}

/// Parse a specific form type
fn parse_form<T: serde::de::DeserializeOwned>(body: &str) -> Result<T, StatusCode> {
    serde_urlencoded::from_str(body).map_err(|e| {
        tracing::error!(error = %e, "Failed to parse form");
        StatusCode::BAD_REQUEST
    })
}

/// Extract action from form data
fn extract_action(form_data: &[(String, String)]) -> Option<String> {
    form_data
        .iter()
        .find(|(k, _)| k == "action")
        .map(|(_, v)| v.clone())
}

/// Extract a field value from form data
fn extract_field(form_data: &[(String, String)], field: &str) -> Result<String, StatusCode> {
    form_data
        .iter()
        .find(|(k, _)| k == field)
        .map(|(_, v)| v.clone())
        .ok_or(StatusCode::BAD_REQUEST)
}

/// Extract component ID from form data
fn extract_component_id(form_data: &[(String, String)], field: &str) -> Result<i64, StatusCode> {
    form_data
        .iter()
        .find(|(k, _)| k == field)
        .and_then(|(_, v)| v.parse::<i64>().ok())
        .ok_or(StatusCode::BAD_REQUEST)
}

/// Extract sort mode from form data
fn extract_sort_mode(form_data: &[(String, String)]) -> String {
    form_data
        .iter()
        .find(|(k, _)| k == "sort_mode")
        .map(|(_, v)| v.clone())
        .unwrap_or_else(|| "created_at_asc".to_string())
}

/// Extract positions for manual sorting
fn extract_positions(form_data: &[(String, String)], sort_mode: &str) -> Option<Vec<(i64, i32)>> {
    if sort_mode != "manual" {
        return None;
    }

    let mut positions = Vec::new();
    for (key, value) in form_data {
        if let Some(page_id_str) = key.strip_prefix("position_") {
            if let (Ok(page_id), Ok(position)) = (page_id_str.parse::<i64>(), value.parse::<i32>())
            {
                positions.push((page_id, position));
            }
        }
    }

    if positions.is_empty() {
        None
    } else {
        Some(positions)
    }
}

/// Build redirect path
fn build_redirect_path(content_path: &ContentPath) -> String {
    if content_path.path == "/" {
        "/".to_string()
    } else {
        format!("{}/", content_path.path)
    }
}

/// Parse SaveDraftForm from form data
fn parse_save_draft_form(form_data: &[(String, String)]) -> Result<SaveDraftForm, StatusCode> {
    let mut component_ids = Vec::new();
    let mut component_types = Vec::new();
    let mut component_titles = Vec::new();
    let mut component_templates = Vec::new();
    let mut component_contents = Vec::new();

    for (key, value) in form_data {
        match key.as_str() {
            "component_ids" => {
                if let Ok(id) = value.parse::<i64>() {
                    component_ids.push(id);
                }
            }
            "component_types" => component_types.push(value.clone()),
            "component_titles" => component_titles.push(value.clone()),
            "component_templates" => component_templates.push(value.clone()),
            "component_contents" => component_contents.push(value.clone()),
            _ => {}
        }
    }

    Ok(SaveDraftForm {
        component_ids,
        component_types,
        component_titles,
        component_templates,
        component_contents,
        action: Some("save_draft".to_string()),
    })
}

/// Parse AddComponentForm from form data
fn parse_add_component_form(form_data: &[(String, String)]) -> AddComponentForm {
    let content = form_data
        .iter()
        .find(|(k, _)| k == "content")
        .map(|(_, v)| v.clone())
        .unwrap_or_else(|| {
            tracing::debug!("No content field in form data, using empty string");
            String::new()
        });

    let component_type = form_data
        .iter()
        .find(|(k, _)| k == "component_type")
        .map(|(_, v)| v.clone())
        .unwrap_or_else(|| "text".to_string());

    let ajax = form_data
        .iter()
        .find(|(k, _)| k == "ajax")
        .map(|(_, v)| v == "true")
        .unwrap_or(false);

    AddComponentForm {
        content,
        component_type,
        ajax,
    }
}

/// Resolve site from host domain
async fn resolve_site(db: &sqlx::SqlitePool, host: &str) -> Result<Site, StatusCode> {
    crate::site_config::get_site_config(db, host)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, host = host, "Failed to get site config");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

/// Resolve page from content path
async fn resolve_page(
    db: &sqlx::SqlitePool,
    _site: &Site,
    content_path: &ContentPath,
) -> Result<Page, StatusCode> {
    let page_repo = PageRepository::new(db.clone());

    if content_path.path == "/" {
        get_root_page(&page_repo).await
    } else {
        navigate_to_page(&page_repo, &content_path.path).await
    }
}

/// Get root page for a site
async fn get_root_page(page_repo: &PageRepository) -> Result<Page, StatusCode> {
    page_repo
        .get_root_page()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to get root page");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::NOT_FOUND)
}

/// Navigate through page hierarchy to find target page
async fn navigate_to_page(page_repo: &PageRepository, path: &str) -> Result<Page, StatusCode> {
    let segments = parse_path_segments(path);
    let mut current_page = get_root_page(page_repo).await?;

    for segment in segments {
        let current_page_id = current_page.id.ok_or(StatusCode::NOT_FOUND)?;
        current_page = find_child_by_slug(page_repo, current_page_id, segment).await?;
    }

    Ok(current_page)
}

/// Parse path into segments
fn parse_path_segments(path: &str) -> Vec<&str> {
    path.split('/').filter(|s| !s.is_empty()).collect()
}

/// Find child page by slug
async fn find_child_by_slug(
    page_repo: &PageRepository,
    parent_id: i64,
    slug: &str,
) -> Result<Page, StatusCode> {
    let children = page_repo.list_children(parent_id).await.map_err(|e| {
        tracing::error!(error = %e, parent_id = parent_id, "Failed to list children");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    children
        .into_iter()
        .find(|p| p.slug == slug)
        .ok_or(StatusCode::NOT_FOUND)
}
