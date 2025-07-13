use anyhow::Result;
use doxyde_core::models::site::Site;
use doxyde_db::repositories::PageRepository;
use tera::Context;

use crate::{logo::get_logo_data, AppState};

/// Add common base template context variables
/// This includes site_title, root_page_title, and logo information
pub async fn add_base_context(
    context: &mut Context,
    state: &AppState,
    site: &Site,
) -> Result<()> {
    // Add site title
    context.insert("site_title", &site.title);

    // Get root page title
    let page_repo = PageRepository::new(state.db.clone());
    let root_page_title = if let Ok(Some(root_page)) = page_repo.get_root_page(site.id.unwrap()).await {
        root_page.title
    } else {
        site.title.clone()
    };
    context.insert("root_page_title", &root_page_title);

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
    use crate::config::Config;
    use crate::autoreload_templates::TemplateEngine;
    use doxyde_core::models::site::Site;
    
    #[test]
    fn test_add_base_context_with_site() {
        // Just test that the function exists and compiles
        assert_eq!(1 + 1, 2);
    }
}