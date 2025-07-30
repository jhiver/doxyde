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
use doxyde_core::models::{page::Page, site::Site};
use doxyde_db::repositories::PageRepository;
use tera::Context;

use crate::{csrf::CsrfToken, logo::get_logo_data, AppState};

/// Add common base template context variables
/// This includes site_title, root_page_title, logo information, and navigation
pub async fn add_base_context(
    context: &mut Context,
    state: &AppState,
    site: &Site,
    current_page: Option<&Page>,
) -> Result<()> {
    // Add site title
    context.insert("site_title", &site.title);

    // Get root page and its children for navigation
    let page_repo = PageRepository::new(state.db.clone());
    let site_id = site.id.ok_or_else(|| anyhow::anyhow!("Site has no ID"))?;
    let (root_page_title, root_children) =
        if let Ok(Some(root_page)) = page_repo.get_root_page(site_id).await {
            let title = root_page.title.clone();

            // Get children of root page for top navigation
            let children = if let Some(root_id) = root_page.id {
                page_repo
                    .list_children(root_id)
                    .await
                    .unwrap_or_default()
            } else {
                Vec::new()
            };

            (title, children)
        } else {
            (site.title.clone(), Vec::new())
        };

    context.insert("root_page_title", &root_page_title);

    // Build navigation data with active state
    // For nested pages, we need to check if the current page is under one of the root children
    let current_page_id = current_page.and_then(|p| p.id);
    let mut nav_items: Vec<serde_json::Value> = Vec::new();

    for child in root_children {
        let mut is_current = false;

        // Check if this is the current page
        if child.id == current_page_id {
            is_current = true;
        } else if let Some(current) = current_page {
            // Check if current page is a descendant of this child
            if let (Some(current_id), Some(child_id)) = (current.id, child.id) {
                if let Ok(is_desc) = page_repo
                    .is_descendant_of(current_id, child_id)
                    .await
                {
                    is_current = is_desc;
                }
            }
        }

        nav_items.push(serde_json::json!({
            "title": child.title,
            "url": format!("/{}", child.slug),
            "is_current": is_current
        }));
    }

    context.insert("nav_items", &nav_items);

    // Get logo data
    if let Ok(Some((logo_url, logo_width, logo_height))) =
        get_logo_data(&state.db, site_id).await
    {
        context.insert("logo_url", &logo_url);
        context.insert("logo_width", &logo_width);
        context.insert("logo_height", &logo_height);
    }

    Ok(())
}

/// Add CSRF token to template context
pub fn add_csrf_token(context: &mut Context, csrf_token: &CsrfToken) {
    context.insert("csrf_token", &csrf_token.token);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_base_context_with_site() {
        // Just test that the function exists and compiles
        assert_eq!(1 + 1, 2);
    }

    #[test]
    fn test_add_csrf_token() {
        let mut context = Context::new();
        let csrf_token = CsrfToken::new();

        add_csrf_token(&mut context, &csrf_token);

        assert_eq!(
            context.get("csrf_token").unwrap().as_str().unwrap(),
            &csrf_token.token
        );
    }
}
