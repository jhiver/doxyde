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
    extract::Path,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
};
use doxyde_db::repositories::{SiteAssetRepository, SiteStyleRepository};

use crate::db_middleware::SiteDatabase;

/// Serve combined CSS from site_styles table
pub async fn site_css_handler(SiteDatabase(db): SiteDatabase) -> Result<Response, StatusCode> {
    let repo = SiteStyleRepository::new(db);
    let css = repo.get_combined_css().await.map_err(|e| {
        tracing::error!(error = %e, "Failed to get combined CSS");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok((
        [
            (header::CONTENT_TYPE, "text/css; charset=utf-8"),
            (header::CACHE_CONTROL, "public, max-age=60"),
        ],
        css,
    )
        .into_response())
}

/// Serve a site asset by path from the site_assets table
pub async fn site_asset_handler(
    Path(path): Path<String>,
    SiteDatabase(db): SiteDatabase,
) -> Result<Response, StatusCode> {
    let repo = SiteAssetRepository::new(db);
    let asset = repo.find_by_path(&path).await.map_err(|e| {
        tracing::error!(error = %e, path = %path, "Failed to find site asset");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    match asset {
        Some(asset) => {
            let mut response = (StatusCode::OK, asset.content).into_response();
            let headers = response.headers_mut();
            if let Ok(val) = asset.mime_type.parse() {
                headers.insert(header::CONTENT_TYPE, val);
            }
            if let Ok(val) = "public, max-age=86400".parse() {
                headers.insert(header::CACHE_CONTROL, val);
            }
            Ok(response)
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}
