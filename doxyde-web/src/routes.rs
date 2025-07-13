use crate::{content, handlers, AppState};
use axum::extract::DefaultBodyLimit;
use axum::{routing::get, Router};
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

pub fn create_router(state: AppState) -> Router {
    let max_upload_size = state.config.max_upload_size;

    Router::new()
        // Health check
        .route("/health", get(health))
        // System routes (dot-prefixed)
        .route("/.login", get(handlers::login_form).post(handlers::login))
        .route("/.logout", get(handlers::logout).post(handlers::logout))
        // Dynamic content routes (last, to catch all)
        .fallback(get(content::content_handler).post(content::content_post_handler))
        // Add middleware
        .layer(
            ServiceBuilder::new()
                .layer(DefaultBodyLimit::max(max_upload_size))
                .layer(TraceLayer::new_for_http()),
        )
        .with_state(state)
}

// Health check handler
async fn health() -> &'static str {
    "OK"
}
