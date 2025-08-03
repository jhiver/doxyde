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

use crate::autoreload_templates::TemplateEngine;
use crate::config::Config;
use crate::db_router::DatabaseRouter;
use crate::rate_limit::SharedRateLimiter;

#[derive(Clone)]
pub struct AppState {
    pub db_router: DatabaseRouter,
    pub templates: TemplateEngine,
    pub config: Config,
    pub login_rate_limiter: SharedRateLimiter,
    pub api_rate_limiter: SharedRateLimiter,
}

impl AppState {
    pub fn new(
        db_router: DatabaseRouter,
        templates: TemplateEngine,
        config: Config,
        login_rate_limiter: SharedRateLimiter,
        api_rate_limiter: SharedRateLimiter,
    ) -> Self {
        Self {
            db_router,
            templates,
            config,
            login_rate_limiter,
            api_rate_limiter,
        }
    }

    /// Get a database pool for cross-site operations like OAuth
    /// For now, this returns a pool for site_id 1, but in the future
    /// we might want a dedicated central database for OAuth tokens
    pub async fn get_oauth_db(&self) -> anyhow::Result<sqlx::SqlitePool> {
        use crate::site_resolver::SiteContext;

        // Create a context for the default site (site_id 1)
        let context = SiteContext::new("default".to_string(), &self.config.get_sites_directory()?);

        self.db_router.get_pool(&context).await
    }
}

// Note: SqlitePool is no longer directly extractable from AppState.
// Handlers must use the request extensions to get the site-specific database pool.
