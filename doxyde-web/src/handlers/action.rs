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
    extract::{Host, State},
    http::{StatusCode, Uri},
    response::Response,
};
use doxyde_db::repositories::{PageRepository, SiteRepository};

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
    user: CurrentUser,
    body: String,
) -> Result<Response, StatusCode> {
    // Parse the path to extract content path and action
    let path = uri.path();
    let content_path = ContentPath::parse(path);

    // Use the full host as domain (including port if present)
    let domain = &host;

    // Find the site by domain
    let site_repo = SiteRepository::new(state.db.clone());
    let site = site_repo
        .find_by_domain(domain)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;

    // Navigate to find the page
    let page_repo = PageRepository::new(state.db.clone());
    let page = if content_path.path == "/" {
        page_repo
            .get_root_page(site.id.unwrap())
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?
    } else {
        let segments: Vec<&str> = content_path
            .path
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        let mut current_page = page_repo
            .get_root_page(site.id.unwrap())
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::NOT_FOUND)?;

        for segment in segments {
            let children = page_repo
                .list_children(current_page.id.unwrap())
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            current_page = children
                .into_iter()
                .find(|p| p.slug == segment)
                .ok_or(StatusCode::NOT_FOUND)?;
        }

        current_page
    };

    // Route based on action
    match content_path.action.as_deref() {
        Some(".edit") | Some(".content") => {
            // Parse the form data to find the action
            let form_data: Vec<(String, String)> =
                serde_urlencoded::from_str(&body).map_err(|_| StatusCode::BAD_REQUEST)?;

            // Find the action field
            let action = form_data
                .iter()
                .find(|(k, _)| k == "action")
                .map(|(_, v)| v.as_str())
                .ok_or(StatusCode::BAD_REQUEST)?;

            match action {
                "save_draft" | "publish_draft" => {
                    // Parse arrays from form data
                    let mut component_ids = Vec::new();
                    let mut component_types = Vec::new();
                    let mut component_titles = Vec::new();
                    let mut component_templates = Vec::new();
                    let mut component_contents = Vec::new();

                    for (key, value) in &form_data {
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

                    let save_form = SaveDraftForm {
                        component_ids,
                        component_types,
                        component_titles,
                        component_templates,
                        component_contents,
                    };

                    if action == "save_draft" {
                        crate::handlers::save_draft_handler(
                            state,
                            site,
                            page,
                            user,
                            axum::extract::Form(save_form),
                        )
                        .await
                    } else {
                        // publish_draft - save first, then publish
                        crate::handlers::save_draft_handler(
                            state.clone(),
                            site.clone(),
                            page.clone(),
                            user.clone(),
                            axum::extract::Form(save_form),
                        )
                        .await?;

                        crate::handlers::publish_draft_handler(state, site, page, user).await
                    }
                }
                "discard_draft" => {
                    crate::handlers::discard_draft_handler(state, site, page, user).await
                }
                "add_component" => {
                    // Parse AddComponentForm from form_data
                    let content = form_data
                        .iter()
                        .find(|(k, _)| k == "content")
                        .map(|(_, v)| v.clone())
                        .unwrap_or_default();

                    let component_type = form_data
                        .iter()
                        .find(|(k, _)| k == "component_type")
                        .map(|(_, v)| v.clone())
                        .unwrap_or_else(|| "text".to_string());

                    let add_form = AddComponentForm {
                        content,
                        component_type,
                    };

                    crate::handlers::add_component_handler(
                        state,
                        site,
                        page,
                        user,
                        axum::extract::Form(add_form),
                    )
                    .await
                }
                "delete_component" => {
                    // First save all components except the one to delete
                    let mut component_ids = Vec::new();
                    let mut component_types = Vec::new();
                    let mut component_titles = Vec::new();
                    let mut component_templates = Vec::new();
                    let mut component_contents = Vec::new();

                    // Get the component to delete
                    let delete_component_id = form_data
                        .iter()
                        .find(|(k, _)| k == "delete_component_id")
                        .and_then(|(_, v)| v.parse::<i64>().ok())
                        .ok_or(StatusCode::BAD_REQUEST)?;

                    // We need to maintain the order of components
                    // First, collect all the component IDs to maintain order
                    let all_ids: Vec<i64> = form_data
                        .iter()
                        .filter(|(k, _)| k == "component_ids")
                        .filter_map(|(_, v)| v.parse::<i64>().ok())
                        .collect();

                    // Now collect the data in the same order, skipping the deleted component
                    let mut idx = 0;
                    for (key, value) in &form_data {
                        match key.as_str() {
                            "component_ids" => {
                                if let Ok(id) = value.parse::<i64>() {
                                    if id == delete_component_id {
                                        idx += 1; // Skip this component's data
                                    }
                                }
                            }
                            "component_types" => {
                                if idx < all_ids.len() && all_ids[idx] != delete_component_id {
                                    component_types.push(value.clone());
                                }
                            }
                            "component_titles" => {
                                if idx < all_ids.len() && all_ids[idx] != delete_component_id {
                                    component_titles.push(value.clone());
                                }
                            }
                            "component_templates" => {
                                if idx < all_ids.len() && all_ids[idx] != delete_component_id {
                                    component_templates.push(value.clone());
                                }
                            }
                            "component_contents" => {
                                if idx < all_ids.len() && all_ids[idx] != delete_component_id {
                                    component_contents.push(value.clone());
                                    component_ids.push(all_ids[idx]); // Add the ID after we've collected all data
                                    idx += 1;
                                }
                            }
                            _ => {}
                        }
                    }

                    let save_form = SaveDraftForm {
                        component_ids,
                        component_types,
                        component_titles,
                        component_templates,
                        component_contents,
                    };

                    // Save remaining components
                    crate::handlers::save_draft_handler(
                        state.clone(),
                        site.clone(),
                        page.clone(),
                        user.clone(),
                        axum::extract::Form(save_form),
                    )
                    .await?;

                    // Delete the component
                    crate::handlers::delete_component_handler(
                        state,
                        site,
                        page,
                        user,
                        delete_component_id,
                    )
                    .await
                }
                "move_component" => {
                    // First save all components
                    let mut component_ids = Vec::new();
                    let mut component_types = Vec::new();
                    let mut component_titles = Vec::new();
                    let mut component_templates = Vec::new();
                    let mut component_contents = Vec::new();

                    // Get the component to move and direction
                    let move_component_id = form_data
                        .iter()
                        .find(|(k, _)| k == "move_component_id")
                        .and_then(|(_, v)| v.parse::<i64>().ok())
                        .ok_or(StatusCode::BAD_REQUEST)?;

                    let move_direction = form_data
                        .iter()
                        .find(|(k, _)| k == "move_direction")
                        .map(|(_, v)| v.as_str())
                        .ok_or(StatusCode::BAD_REQUEST)?;

                    // Collect all component data in order
                    for (key, value) in &form_data {
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

                    let save_form = SaveDraftForm {
                        component_ids,
                        component_types,
                        component_titles,
                        component_templates,
                        component_contents,
                    };

                    // Save all components
                    crate::handlers::save_draft_handler(
                        state.clone(),
                        site.clone(),
                        page.clone(),
                        user.clone(),
                        axum::extract::Form(save_form),
                    )
                    .await?;

                    // Move the component
                    crate::handlers::move_component_handler(
                        state,
                        site,
                        page,
                        user,
                        move_component_id,
                        move_direction,
                    )
                    .await
                }
                _ => Err(StatusCode::BAD_REQUEST),
            }
        }
        Some(".new") => {
            // Parse NewPageForm from body
            let new_form: NewPageForm =
                serde_urlencoded::from_str(&body).map_err(|_| StatusCode::BAD_REQUEST)?;

            crate::handlers::create_page_handler(
                state,
                site,
                page,
                user,
                axum::extract::Form(new_form),
            )
            .await
        }
        Some(".properties") => {
            // Parse PagePropertiesForm from body
            let props_form: PagePropertiesForm =
                serde_urlencoded::from_str(&body).map_err(|_| StatusCode::BAD_REQUEST)?;

            crate::handlers::update_page_properties_handler(
                state,
                site,
                page,
                user,
                axum::extract::Form(props_form),
            )
            .await
        }
        Some(".move") => {
            // Parse MovePageForm from body
            let move_form: MovePageForm =
                serde_urlencoded::from_str(&body).map_err(|_| StatusCode::BAD_REQUEST)?;

            crate::handlers::do_move_page_handler(
                state,
                site,
                page,
                user,
                axum::extract::Form(move_form),
            )
            .await
        }
        Some(".delete") => {
            // Parse DeletePageForm from body
            let delete_form: DeletePageForm =
                serde_urlencoded::from_str(&body).map_err(|_| StatusCode::BAD_REQUEST)?;

            crate::handlers::do_delete_page_handler(
                state,
                site,
                page,
                user,
                axum::extract::Form(delete_form),
            )
            .await
        }
        _ => Err(StatusCode::NOT_FOUND),
    }
}
