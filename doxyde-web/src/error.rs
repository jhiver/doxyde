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
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse, Response},
};
use std::fmt;
use tera::Context;

/// Application error type that includes context for better debugging
pub struct AppError {
    pub status: StatusCode,
    pub message: String,
    pub details: Option<String>,
    pub templates: Option<TemplateEngine>,
}

impl fmt::Debug for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppError")
            .field("status", &self.status)
            .field("message", &self.message)
            .field("details", &self.details)
            .field("templates", &self.templates.is_some())
            .finish()
    }
}

impl AppError {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
            details: None,
            templates: None,
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    pub fn with_templates(mut self, templates: TemplateEngine) -> Self {
        self.templates = Some(templates);
        self
    }

    pub fn internal_server_error(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, message)
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, message)
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, message)
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(details) = &self.details {
            write!(f, "{}: {}", self.message, details)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for AppError {}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // Log the full error with details
        tracing::error!(
            status = ?self.status,
            message = %self.message,
            details = ?self.details,
            "Request failed"
        );

        // Try to render a nice HTML error page if we have templates
        if let Some(templates) = &self.templates {
            if let Ok(html) = self.render_error_page(templates) {
                return (self.status, Html(html)).into_response();
            }
        }

        // Fall back to simple error response
        (self.status, self.message).into_response()
    }
}

impl AppError {
    fn render_error_page(&self, templates: &TemplateEngine) -> Result<String, ()> {
        // Create template context
        let mut context = Context::new();

        // Add error-specific context
        context.insert("error_code", &self.status.as_u16());
        context.insert("error_message", &self.message);
        context.insert("error_details", &self.details);

        // Add helpful context based on error type
        match self.status {
            StatusCode::NOT_FOUND => {
                context.insert("error_title", "Page Not Found");
                context.insert(
                    "error_description",
                    "The page you're looking for doesn't exist.",
                );
            }
            StatusCode::FORBIDDEN => {
                context.insert("error_title", "Access Denied");
                context.insert(
                    "error_description",
                    "You don't have permission to access this page.",
                );
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
                context.insert("error_description", &self.message);
            }
        }

        // Determine template based on status code
        let template_name = match self.status {
            StatusCode::NOT_FOUND => "errors/404.html",
            StatusCode::FORBIDDEN => "errors/403.html",
            StatusCode::INTERNAL_SERVER_ERROR => "errors/500.html",
            _ => "errors/generic.html",
        };

        // Try to render the error template
        match templates.render(template_name, &context) {
            Ok(html) => Ok(html),
            Err(e) => {
                tracing::error!("Failed to render error template: {:?}", e);
                // Try fallback to generic error template
                if template_name != "errors/generic.html" {
                    templates
                        .render("errors/generic.html", &context)
                        .map_err(|e| {
                            tracing::error!("Failed to render generic error template: {:?}", e);
                        })
                } else {
                    Err(())
                }
            }
        }
    }
}

// Conversion from anyhow::Error
impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        tracing::error!("Anyhow error: {:?}", err);
        Self::internal_server_error("Internal server error").with_details(format!("{:?}", err))
    }
}
