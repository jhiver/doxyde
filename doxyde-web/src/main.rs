use anyhow::Result;
use doxyde_web::{
    config::Config, db::init_database, routes, state::AppState, templates::init_templates,
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

    // Create application state
    let state = AppState::new(db, templates, config.clone());

    // Create router
    let app = routes::create_router(state);

    // Start server
    let listener = TcpListener::bind(&config.bind_addr()).await?;
    info!("Listening on {}", listener.local_addr()?);

    axum::serve(listener, app).await?;

    Ok(())
}
