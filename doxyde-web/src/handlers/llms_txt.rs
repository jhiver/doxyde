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

use axum::{
    extract::State,
    http::{header, StatusCode},
    response::Response,
};
use axum_extra::extract::Host;
use doxyde_core::models::site::Site;
use doxyde_db::repositories::PageRepository;

use crate::{db_middleware::SiteDatabase, site_config::get_site_config, AppState};

async fn load_site(db: &sqlx::SqlitePool, host: &str) -> Result<Site, StatusCode> {
    get_site_config(db, host)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)
}

fn build_page_path(all_pages: &[doxyde_core::Page], page: &doxyde_core::Page) -> String {
    if page.parent_page_id.is_none() {
        return "/".to_string();
    }

    let mut path_segments = vec![page.slug.clone()];
    let mut current_parent = page.parent_page_id;

    while let Some(parent_id) = current_parent {
        if let Some(parent) = all_pages.iter().find(|p| p.id == Some(parent_id)) {
            if parent.parent_page_id.is_some() {
                path_segments.insert(0, parent.slug.clone());
            }
            current_parent = parent.parent_page_id;
        } else {
            break;
        }
    }

    format!("/{}", path_segments.join("/"))
}

pub async fn llms_txt_handler(
    Host(host): Host,
    State(_state): State<AppState>,
    SiteDatabase(db): SiteDatabase,
) -> Result<Response, StatusCode> {
    let site = load_site(&db, &host).await?;

    let repo = PageRepository::new(db.clone());
    let mut pages = repo.list_all().await.map_err(|e| {
        tracing::error!("Failed to list pages for llms.txt: {:?}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Filter out pages where meta_robots contains "noindex"
    pages.retain(|p| !p.meta_robots.contains("noindex"));

    // Ensure the root page is at the beginning, followed by the rest in list order.
    let mut ordered_pages = Vec::new();
    let pages_for_path = pages.clone();

    if let Some(root_idx) = pages.iter().position(|p| p.parent_page_id.is_none()) {
        let root = pages.remove(root_idx);
        ordered_pages.push(root);
    }
    ordered_pages.extend(pages);

    let mut body = format!("# {}\n\n## Pages\n\n", site.title);
    for page in ordered_pages {
        if page.parent_page_id.is_none() && page.title.is_empty() {
            continue;
        }

        let path = build_page_path(&pages_for_path, &page);
        let url = format!("https://{}{}", host, path);

        match &page.description {
            Some(desc) if !desc.trim().is_empty() => {
                body.push_str(&format!("- [{}]({}): {}\n", page.title, url, desc.trim()));
            }
            _ => {
                body.push_str(&format!("- [{}]({})\n", page.title, url));
            }
        }
    }

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(axum::body::Body::from(body))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}
