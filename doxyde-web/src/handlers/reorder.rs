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
    site: Site,
    page: Page,
    _current_user: CurrentUser,
) -> Result<Response, StatusCode> {
    // Simple permission check - user must be logged in
    // In production, you might want to check site permissions here
    let can_edit = true;

    // Get child pages using the sorted method
    let page_repo = PageRepository::new(state.db.clone());
    let children = page_repo
        .list_children_sorted(page.id.unwrap())
        .await
        .map_err(|e| {
            tracing::error!(error = ?e, "Failed to list children");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Convert to template-friendly format
    let child_pages: Vec<ChildPage> = children
        .iter()
        .map(|child| ChildPage {
            id: child.id.unwrap(),
            title: child.title.clone(),
            position: child.position,
            created_at: child.created_at.format("%Y-%m-%d").to_string(),
        })
        .collect();

    let has_children = !children.is_empty();
    
    let context = ReorderContext {
        site: site.clone(),
        page: page.clone(),
        children: child_pages,
        sort_mode: page.sort_mode.clone(),
        can_edit,
        action: ".reorder".to_string(),
        has_children,
    };

    // Convert to Tera context
    let mut tera_context = tera::Context::new();
    
    // Add base context (site_title, root_page_title, logo data, navigation)
    add_base_context(&mut tera_context, &state, &site, Some(&page))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
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
    
    tera_context.insert("page", &context.page);
    tera_context.insert("children", &context.children);
    tera_context.insert("sort_mode", &context.sort_mode);
    tera_context.insert("can_edit", &context.can_edit);
    tera_context.insert("action", &context.action);
    tera_context.insert("has_children", &context.has_children);
    tera_context.insert("current_path", &current_path);
    
    match state.templates.render("page_reorder.html", &tera_context) {
        Ok(html) => Ok(Html(html).into_response()),
        Err(e) => {
            tracing::error!(error = ?e, "Failed to render reorder page");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

pub async fn update_page_order_handler(
    State(state): State<Arc<AppState>>,
    _site: Site,
    page: Page,
    _current_user: CurrentUser,
    sort_mode: String,
    positions: Option<Vec<(i64, i32)>>,
) -> Result<(), AppError> {
    // Simple permission check - user must be logged in
    // In production, you might want to check site permissions here

    let page_repo = PageRepository::new(state.db.clone());

    // Update the page's sort mode if it changed
    if page.sort_mode != sort_mode {
        let mut updated_page = page.clone();
        updated_page.sort_mode = sort_mode.clone();
        page_repo
            .update(&updated_page)
            .await
            .map_err(|e| {
                tracing::error!(error = ?e, "Failed to update page sort mode");
                AppError::internal_server_error("Failed to update page sort mode")
            })?;
    }

    // If manual mode and positions provided, update them
    if sort_mode == "manual" {
        if let Some(positions) = positions {
            page_repo
                .update_positions(&positions)
                .await
                .map_err(|e| {
                    tracing::error!(error = ?e, "Failed to update page positions");
                    AppError::internal_server_error("Failed to update page positions")
                })?;
        }
    }

    Ok(())
}