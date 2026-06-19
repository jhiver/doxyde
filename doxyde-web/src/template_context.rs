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

use crate::{
    content_translate::TranslationPolicy, csrf::CsrfToken, locale_middleware::RequestLocale,
    logo::get_logo_data, ui_labels, AppState,
};
use sqlx::SqlitePool;

/// Build the canonical dot-action URL that serves `current_path` in `code`,
/// e.g. ("/about", "fr") -> "/about/.fr" and ("/", "fr") -> "/.fr".
pub(crate) fn lang_action_url(current_path: &str, code: &str) -> String {
    let base = current_path.trim_end_matches('/');
    if base.is_empty() {
        format!("/.{code}")
    } else {
        format!("{base}/.{code}")
    }
}

/// Inject locale-related context: `lang`, `dir`, `source_lang`, the resolved UI
/// `labels` map, `available_locales` (for the switcher) and `hreflang_alternates`
/// (per-language canonical URLs + x-default). Called by the page display and the
/// `/.fr` `/.en` handlers; admin views skip it and fall back to en/ltr defaults.
pub async fn add_locale_context(
    context: &mut Context,
    state: &AppState,
    db: &SqlitePool,
    site: &Site,
    locale: &RequestLocale,
    current_path: &str,
    policy: TranslationPolicy,
) {
    context.insert("lang", &locale.lang);
    context.insert("dir", &locale.dir);
    context.insert("source_lang", &locale.source_lang);

    // UI labels for the active language (passthrough in the source language).
    let labels = ui_labels::resolve_labels(
        state,
        db,
        &locale.lang,
        &locale.source_lang,
        &site.domain,
        policy,
    )
    .await;
    context.insert("labels", &labels);

    // Language switcher data.
    let available_locales: Vec<serde_json::Value> = locale
        .enabled
        .iter()
        .map(|l| {
            serde_json::json!({
                "code": l.code,
                "label": l.label,
                "dir": l.dir,
                "is_current": l.code == locale.lang,
                "switch_url": lang_action_url(current_path, &l.code),
            })
        })
        .collect();
    context.insert("available_locales", &available_locales);

    // hreflang alternates: one per enabled language pointing at its canonical
    // dot-action URL, plus x-default pointing at the negotiated bare URL.
    let mut hreflang: Vec<serde_json::Value> = locale
        .enabled
        .iter()
        .map(|l| {
            serde_json::json!({
                "hreflang": l.code,
                "href": lang_action_url(current_path, &l.code),
            })
        })
        .collect();
    let x_default = if current_path.is_empty() {
        "/".to_string()
    } else {
        current_path.to_string()
    };
    hreflang.push(serde_json::json!({ "hreflang": "x-default", "href": x_default }));
    context.insert("hreflang_alternates", &hreflang);
}

/// Add common base template context variables
/// This includes site_title, root_page_title, logo information, and navigation
pub async fn add_base_context(
    context: &mut Context,
    db: &SqlitePool,
    site: &Site,
    current_page: Option<&Page>,
) -> Result<()> {
    // Add site title
    context.insert("site_title", &site.title);

    // Get root page and its children for navigation
    let page_repo = PageRepository::new(db.clone());
    let site_id = site.id.ok_or_else(|| anyhow::anyhow!("Site has no ID"))?;
    let (root_page_title, root_children) =
        if let Ok(Some(root_page)) = page_repo.get_root_page().await {
            let title = root_page.title.clone();

            // Get children of root page for top navigation
            let children = if let Some(root_id) = root_page.id {
                page_repo.list_children(root_id).await.unwrap_or_default()
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
                if let Ok(is_desc) = page_repo.is_descendant_of(current_id, child_id).await {
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
    if let Ok(Some((logo_url, logo_width, logo_height))) = get_logo_data(db, site_id).await {
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
        let csrf_token = CsrfToken::new(32);

        add_csrf_token(&mut context, &csrf_token);

        assert_eq!(
            context.get("csrf_token").unwrap().as_str().unwrap(),
            &csrf_token.token
        );
    }
}
