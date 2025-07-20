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
    body::Body,
    extract::{Host, State},
    http::{Request, Response, StatusCode},
    middleware::Next,
};
use doxyde_db::repositories::SiteRepository;
use std::sync::Arc;
use tera::Context;

use crate::{template_context::add_base_context, AppState};

/// Middleware to enhance error responses with proper templates
pub async fn error_enhancer_middleware(
    Host(host): Host,
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response<Body>, Response<Body>> {
    // Call the next handler
    let response = next.run(request).await;

    // Check if it's an error response we should enhance
    let status = response.status();
    if status.is_client_error() || status.is_server_error() {
        // Try to enhance the error response
        if let Ok(enhanced) = enhance_error_response(status, &host, &state).await {
            return Ok(enhanced);
        }
    }

    Ok(response)
}

/// Enhance error response with proper template
async fn enhance_error_response(
    status: StatusCode,
    host: &str,
    state: &AppState,
) -> Result<Response<Body>, ()> {
    // Try to find the site
    let site_repo = SiteRepository::new(state.db.clone());
    let site = match site_repo.find_by_domain(host).await {
        Ok(Some(site)) => site,
        _ => return Err(()),
    };

    // Create template context
    let mut context = Context::new();

    // Add base context for consistent site branding
    if add_base_context(&mut context, state, &site, None)
        .await
        .is_err()
    {
        return Err(());
    }

    // Add error-specific context
    context.insert("error_code", &status.as_u16());

    match status {
        StatusCode::NOT_FOUND => {
            context.insert("error_title", "Page Not Found");
            context.insert("error_description", "The page you're looking for doesn't exist.");
        }
        StatusCode::FORBIDDEN => {
            context.insert("error_title", "Access Denied");
            context.insert("error_description", "You don't have permission to access this page.");
        }
        StatusCode::INTERNAL_SERVER_ERROR => {
            context.insert("error_title", "Server Error");
            context.insert(
                "error_description",
                "Something went wrong on our end. Please try again later.",
            );
        }
        _ => {
            context.insert("error_title", "Error");
            context.insert("error_description", status.canonical_reason().unwrap_or("An error occurred"));
        }
    }

    // Determine template based on status code
    let template_name = match status {
        StatusCode::NOT_FOUND => "errors/404.html",
        StatusCode::FORBIDDEN => "errors/403.html",
        StatusCode::INTERNAL_SERVER_ERROR => "errors/500.html",
        _ => "errors/generic.html",
    };

    // Try to render the error template
    match state.templates.render(template_name, &context) {
        Ok(html) => {
            let response = Response::builder()
                .status(status)
                .header("content-type", "text/html; charset=utf-8")
                .body(Body::from(html))
                .unwrap();
            Ok(response)
        }
        Err(e) => {
            tracing::error!("Failed to render error template: {:?}", e);
            Err(())
        }
    }
}