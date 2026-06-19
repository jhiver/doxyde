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

use std::sync::Arc;

use anyhow::Result;
use dashmap::DashSet;
use doxyde_web::{
    config::Config,
    db_router::DatabaseRouter,
    rate_limit::{create_api_rate_limiter, create_login_rate_limiter},
    routes,
    services::{deferred_translation::run_worker, i18n::I18nClient},
    state::{AppState, TranslationHandle},
    templates::init_templates,
};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Semaphore};
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

    // Initialize database router for multi-site support
    info!(
        "Initializing database router with sites directory: {}",
        config.sites_directory
    );
    let db_router = DatabaseRouter::new(config.clone()).await?;

    // Initialize templates
    info!("Loading templates from: {}", config.templates_dir);
    let templates = init_templates(&config.templates_dir, config.development_mode)?;

    // Warn if deprecated uploads_dir is configured (per-site images/ is used now)
    if !config.uploads_dir.is_empty() && config.uploads_dir != "." {
        tracing::warn!(
            "uploads_dir config is deprecated. Images are now stored per-site in sites/<hash>/images/"
        );
    }
    info!(
        "Max upload size: {} bytes ({} MB)",
        config.max_upload_size,
        config.max_upload_size / 1024 / 1024
    );

    // Create rate limiters
    let login_rate_limiter = create_login_rate_limiter(config.login_attempts_per_minute);
    let api_rate_limiter = create_api_rate_limiter(config.api_requests_per_minute);

    // Initialize i18n translation client + background worker
    info!("i18n service address: {}", config.i18n_service_addr);
    let i18n = I18nClient::new(&config.i18n_service_addr);
    let (translation_tx, translation_rx) = mpsc::channel(256);
    let translation_in_flight = Arc::new(DashSet::new());
    let translation_semaphore = Arc::new(Semaphore::new(config.translation_workers));
    let translation = TranslationHandle {
        tx: translation_tx,
        in_flight: translation_in_flight.clone(),
        semaphore: translation_semaphore.clone(),
    };
    tokio::spawn(run_worker(
        translation_rx,
        i18n.clone(),
        translation_in_flight,
        translation_semaphore,
    ));

    // Create application state
    let state = AppState::new(
        db_router,
        templates,
        config.clone(),
        login_rate_limiter,
        api_rate_limiter,
        i18n,
        translation,
    );

    // Pre-warm the translation cache for all sites in the background, so the
    // first visit of a translated page is served warm (no English flash).
    doxyde_web::services::warm::spawn_prewarm(state.clone());

    // Create router
    let app = routes::create_router(state);

    // Start server
    let listener = TcpListener::bind(&config.bind_addr()).await?;
    info!("Listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;

    Ok(())
}
