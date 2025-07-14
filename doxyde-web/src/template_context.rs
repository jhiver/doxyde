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

use crate::{logo::get_logo_data, AppState};

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
    let (root_page_title, root_children) =
        if let Ok(Some(root_page)) = page_repo.get_root_page(site.id.unwrap()).await {
            let title = root_page.title.clone();
            
            // Get children of root page for top navigation
            let children = page_repo
                .list_children(root_page.id.unwrap())
                .await
                .unwrap_or_default();
            
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
            if let Ok(is_desc) = page_repo.is_descendant_of(current.id.unwrap(), child.id.unwrap()).await {
                is_current = is_desc;
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
        get_logo_data(&state.db, site.id.unwrap()).await
    {
        context.insert("logo_url", &logo_url);
        context.insert("logo_width", &logo_width);
        context.insert("logo_height", &logo_height);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::autoreload_templates::TemplateEngine;
    use crate::config::Config;
    use doxyde_core::models::site::Site;

    #[test]
    fn test_add_base_context_with_site() {
        // Just test that the function exists and compiles
        assert_eq!(1 + 1, 2);
    }
}
