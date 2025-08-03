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
use doxyde_core::models::site::Site;
use sqlx::SqlitePool;

/// Get site configuration from the site_config table
/// In multi-database mode, each database has exactly one site
pub async fn get_site_config(db: &SqlitePool, domain: &str) -> Result<Site> {
    let site_config = sqlx::query!("SELECT title FROM site_config WHERE id = 1")
        .fetch_optional(db)
        .await?;

    match site_config {
        Some(config) => {
            // Create a Site object from the configuration
            // The domain comes from the request, not the database
            Ok(Site {
                id: Some(1), // In multi-db mode, site ID is always 1
                domain: domain.to_string(),
                title: config.title,
                created_at: chrono::Utc::now(), // These timestamps don't matter for display
                updated_at: chrono::Utc::now(),
            })
        }
        None => {
            anyhow::bail!("Site configuration not found")
        }
    }
}
