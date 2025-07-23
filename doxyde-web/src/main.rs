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
use doxyde_web::{
    config::Config,
    db::init_database,
    rate_limit::{create_api_rate_limiter, create_login_rate_limiter},
    routes,
    state::AppState,
    templates::init_templates,
};
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "doxyde_web=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = Config::from_env()?;
    info!("Starting Doxyde web server");

    // Initialize database
    info!("Initializing database: {}", config.database_url);
    let db = init_database(&config.database_url).await?;

    // Initialize templates
    info!("Loading templates from: {}", config.templates_dir);
    let templates = init_templates(&config.templates_dir, config.development_mode)?;

    // Ensure uploads directory exists
    std::fs::create_dir_all(&config.uploads_dir)?;
    info!("Uploads directory: {}", config.uploads_dir);

    // Create rate limiters
    let login_rate_limiter = create_login_rate_limiter();
    let api_rate_limiter = create_api_rate_limiter();

    // Create application state
    let state = AppState::new(
        db,
        templates,
        config.clone(),
        login_rate_limiter,
        api_rate_limiter,
    );

    // Create router
    let app = routes::create_router(state);

    // Start server
    let listener = TcpListener::bind(&config.bind_addr()).await?;
    info!("Listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;

    Ok(())
}
