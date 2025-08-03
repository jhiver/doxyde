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

use axum::http::StatusCode;
use doxyde_core::models::page::Page;
use doxyde_db::repositories::PageRepository;
use tera::Context;

use crate::{auth::CurrentUser, AppState};

/// Add all necessary context variables for the action bar template
pub async fn add_action_bar_context(
    context: &mut Context,
    _state: &AppState,
    db: &sqlx::SqlitePool,
    page: &Page,
    _user: &CurrentUser,
    action: &str,
) -> Result<(), StatusCode> {
    let page_repo = PageRepository::new(db.clone());

    // Set the current action
    context.insert("action", action);

    // Always set can_edit to true since these handlers require authentication
    context.insert("can_edit", &true);

    // Check if page has children for "Reorder" link
    if let Some(page_id) = page.id {
        let children = page_repo.list_children(page_id).await.map_err(|e| {
            tracing::error!("Failed to list children: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
        context.insert("has_children", &!children.is_empty());

        // Check if page is movable (has valid move targets)
        let is_movable = if page.parent_page_id.is_some() {
            // Only non-root pages can be moved
            match page_repo.get_valid_move_targets(page_id).await {
                Ok(targets) => !targets.is_empty(),
                Err(e) => {
                    tracing::error!("Failed to get valid move targets: {:?}", e);
                    false
                }
            }
        } else {
            false
        };
        context.insert("is_movable", &is_movable);

        // Check if page can be deleted (not root and has no children)
        let can_delete = page.parent_page_id.is_some() && children.is_empty();
        context.insert("can_delete", &can_delete);
    } else {
        // Page without ID cannot have children, be moved, or be deleted
        context.insert("has_children", &false);
        context.insert("is_movable", &false);
        context.insert("can_delete", &false);
    }

    Ok(())
}
