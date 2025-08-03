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

use super::shared::add_action_bar_context;
use crate::{auth::CurrentUser, error::AppError, template_context::add_base_context, AppState};
use axum::{
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use doxyde_core::{Page, Site};
use doxyde_db::repositories::PageRepository;
use serde::Serialize;
use std::sync::Arc;

#[derive(Serialize)]
struct ChildPage {
    id: i64,
    title: String,
    position: i32,
    created_at: String,
    url: String,
}

#[derive(Serialize)]
struct ReorderContext {
    site: Site,
    page: Page,
    children: Vec<ChildPage>,
    sort_mode: String,
    can_edit: bool,
    action: String,
    has_children: bool,
}

pub async fn reorder_page_handler(
    State(state): State<Arc<AppState>>,
    db: sqlx::SqlitePool,
    site: Site,
    page: Page,
    current_user: CurrentUser,
) -> Result<Response, StatusCode> {
    // Get child pages using the sorted method
    let page_repo = PageRepository::new(db.clone());
    let page_id = page.id.ok_or(StatusCode::NOT_FOUND)?;
    let children = page_repo.list_children_sorted(page_id).await.map_err(|e| {
        tracing::error!(error = ?e, "Failed to list children");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Build current path by traversing parent pages
    let mut path_parts = Vec::new();
    let mut current_page = Some(page.clone());

    // Build path from bottom to top
    while let Some(p) = current_page {
        if p.parent_page_id.is_some() {
            path_parts.push(p.slug.clone());
        }

        // Get parent page
        if let Some(parent_id) = p.parent_page_id {
            match page_repo.find_by_id(parent_id).await {
                Ok(Some(parent)) => current_page = Some(parent),
                _ => current_page = None,
            }
        } else {
            current_page = None;
        }
    }

    // Reverse to get top-to-bottom order
    path_parts.reverse();

    // Build the path
    let current_path = if path_parts.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", path_parts.join("/"))
    };

    // Convert to template-friendly format
    let child_pages: Vec<ChildPage> = children
        .iter()
        .filter_map(|child| {
            // Skip children without ID
            let child_id = child.id?;

            // Build child URL
            let child_url = if current_path == "/" {
                format!("/{}", child.slug)
            } else {
                format!("{}/{}", current_path, child.slug)
            };

            Some(ChildPage {
                id: child_id,
                title: child.title.clone(),
                position: child.position,
                created_at: child.created_at.format("%Y-%m-%d").to_string(),
                url: child_url,
            })
        })
        .collect();

    let context = ReorderContext {
        site: site.clone(),
        page: page.clone(),
        children: child_pages,
        sort_mode: page.sort_mode.clone(),
        can_edit: true,
        action: ".reorder".to_string(),
        has_children: !children.is_empty(),
    };

    // Convert to Tera context
    let mut tera_context = tera::Context::new();

    // Add base context (site_title, root_page_title, logo data, navigation)
    add_base_context(&mut tera_context, &db, &site, Some(&page))
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to add base context for reorder page");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    tera_context.insert("page", &context.page);
    tera_context.insert("children", &context.children);
    tera_context.insert("sort_mode", &context.sort_mode);
    tera_context.insert("current_path", &current_path);
    tera_context.insert("user", &current_user.user);

    // Add all action bar context variables
    add_action_bar_context(
        &mut tera_context,
        &state,
        &db,
        &page,
        &current_user,
        ".reorder",
    )
    .await?;

    match state.templates.render("page_reorder.html", &tera_context) {
        Ok(html) => Ok(Html(html).into_response()),
        Err(e) => {
            tracing::error!(error = ?e, "Failed to render reorder page");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn update_page_order_handler(
    State(_state): State<Arc<AppState>>,
    db: sqlx::SqlitePool,
    _site: Site,
    page: Page,
    _current_user: CurrentUser,
    sort_mode: String,
    positions: Option<Vec<(i64, i32)>>,
) -> Result<(), AppError> {
    // Simple permission check - user must be logged in
    // In production, you might want to check site permissions here

    let page_repo = PageRepository::new(db.clone());

    // Update the page's sort mode if it changed
    if page.sort_mode != sort_mode {
        let mut updated_page = page.clone();
        updated_page.sort_mode = sort_mode.clone();
        page_repo.update(&updated_page).await.map_err(|e| {
            tracing::error!(error = ?e, "Failed to update page sort mode");
            AppError::internal_server_error("Failed to update page sort mode")
        })?;
    }

    // If manual mode and positions provided, update them
    if sort_mode == "manual" {
        if let Some(positions) = positions {
            page_repo.update_positions(&positions).await.map_err(|e| {
                tracing::error!(error = ?e, "Failed to update page positions");
                AppError::internal_server_error("Failed to update page positions")
            })?;
        }
    }

    Ok(())
}
