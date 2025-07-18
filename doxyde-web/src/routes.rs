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

use crate::{content, debug_middleware::debug_form_middleware, handlers, AppState};
use axum::extract::DefaultBodyLimit;
use axum::{middleware, routing, routing::get, Router};
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
        // MCP Token management
        .route(
            "/.settings/mcp",
            get(handlers::list_tokens_handler).post(handlers::create_token_handler),
        )
        .route(
            "/.settings/mcp/:token_id",
            get(handlers::show_token_handler),
        )
        .route(
            "/.settings/mcp/:token_id/revoke",
            get(handlers::revoke_token_handler).post(handlers::revoke_token_handler),
        )
        // MCP Server endpoint (supports both regular JSON-RPC and SSE)
        .route("/.mcp/:token_id", routing::post(handlers::mcp_http_handler))
        // Dynamic content routes (last, to catch all)
        .fallback(get(content::content_handler).post(content::content_post_handler))
        // Add middleware
        .layer(middleware::from_fn(debug_form_middleware))
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
