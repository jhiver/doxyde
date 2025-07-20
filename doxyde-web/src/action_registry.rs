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
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use doxyde_core::models::{page::Page, site::Site};
use std::collections::HashMap;

use crate::{auth::OptionalUser, error::AppError, AppState};

/// Type for action handler functions
type ActionHandlerFn = fn(
    AppState,
    Site,
    Page,
    OptionalUser,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, AppError>> + Send>>;

/// Registry for action handlers
pub struct ActionRegistry {
    handlers: HashMap<String, ActionHandlerFn>,
}

impl ActionRegistry {
    /// Create a new action registry
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Register an action handler
    pub fn register(&mut self, action: &str, handler: ActionHandlerFn) {
        self.handlers.insert(action.to_string(), handler);
    }

    /// Get a handler for an action
    pub fn get(&self, action: &str) -> Option<&ActionHandlerFn> {
        self.handlers.get(action)
    }

    /// Build the default action registry
    pub fn build_default() -> Self {
        let mut registry = Self::new();

        // Register all handlers
        registry.register("", handle_display_page);
        registry.register(".edit", handle_edit_page);
        registry.register(".content", handle_edit_page); // Same as .edit
        registry.register(".properties", handle_properties);
        registry.register(".new", handle_new_page);
        registry.register(".move", handle_move_page);
        registry.register(".reorder", handle_reorder);
        registry.register(".delete", handle_delete_page);
        registry.register(".add-component", handle_add_component);
        registry.register(".upload-image", handle_upload);
        registry.register(".upload-component-image", handle_upload);

        registry
    }
}

// Handler functions

fn handle_display_page(
    state: AppState,
    site: Site,
    page: Page,
    user: OptionalUser,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, AppError>> + Send>> {
    Box::pin(async move {
        crate::handlers::pages::show_page_handler(state, site, page, user)
            .await
            .map(|r| r.into_response())
            .map_err(|e| {
                AppError::internal_server_error("Failed to render page")
                    .with_details(format!("Status: {:?}", e))
            })
    })
}

fn handle_edit_page(
    state: AppState,
    site: Site,
    page: Page,
    user: OptionalUser,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, AppError>> + Send>> {
    Box::pin(async move {
        if let OptionalUser(Some(current_user)) = user {
            match crate::handlers::edit_page_content_handler(state, site, page, current_user).await
            {
                Ok(response) => Ok(response),
                Err(status) => {
                    tracing::error!(status = ?status, "Failed to render edit page");
                    match status {
                        StatusCode::FORBIDDEN => {
                            Err(AppError::forbidden(
                                "You don't have permission to edit this page",
                            ))
                        }
                        StatusCode::NOT_FOUND => Err(AppError::not_found("Page not found")),
                        StatusCode::INTERNAL_SERVER_ERROR => {
                            Err(AppError::internal_server_error("Failed to render edit page"))
                        }
                        _ => Err(AppError::new(status, "An error occurred")),
                    }
                }
            }
        } else {
            Ok(axum::response::Redirect::to("/.login").into_response())
        }
    })
}

fn handle_properties(
    state: AppState,
    site: Site,
    page: Page,
    user: OptionalUser,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, AppError>> + Send>> {
    Box::pin(async move {
        if let OptionalUser(Some(current_user)) = user {
            match crate::handlers::page_properties_handler(state, site, page, current_user).await {
                Ok(response) => Ok(response),
                Err(status) => {
                    tracing::error!(status = ?status, "Failed to render properties page");
                    match status {
                        StatusCode::FORBIDDEN => {
                            Err(AppError::forbidden(
                                "You don't have permission to edit this page",
                            ))
                        }
                        StatusCode::NOT_FOUND => Err(AppError::not_found("Page not found")),
                        StatusCode::INTERNAL_SERVER_ERROR => {
                            Err(AppError::internal_server_error(
                                "Failed to render properties page",
                            ))
                        }
                        _ => Err(AppError::new(status, "An error occurred")),
                    }
                }
            }
        } else {
            Ok(axum::response::Redirect::to("/.login").into_response())
        }
    })
}

fn handle_new_page(
    state: AppState,
    site: Site,
    page: Page,
    user: OptionalUser,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, AppError>> + Send>> {
    Box::pin(async move {
        if let OptionalUser(Some(current_user)) = user {
            match crate::handlers::new_page_handler(state, site, page, current_user).await {
                Ok(response) => Ok(response),
                Err(status) => {
                    tracing::error!(status = ?status, "Failed to render new page form");
                    match status {
                        StatusCode::FORBIDDEN => {
                            Err(AppError::forbidden(
                                "You don't have permission to create pages",
                            ))
                        }
                        StatusCode::NOT_FOUND => Err(AppError::not_found("Parent page not found")),
                        StatusCode::INTERNAL_SERVER_ERROR => {
                            Err(AppError::internal_server_error(
                                "Failed to render new page form",
                            ))
                        }
                        _ => Err(AppError::new(status, "An error occurred")),
                    }
                }
            }
        } else {
            Ok(axum::response::Redirect::to("/.login").into_response())
        }
    })
}

fn handle_move_page(
    state: AppState,
    site: Site,
    page: Page,
    user: OptionalUser,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, AppError>> + Send>> {
    Box::pin(async move {
        if let OptionalUser(Some(current_user)) = user {
            match crate::handlers::move_page_handler(state, site, page, current_user).await {
                Ok(response) => Ok(response),
                Err(status) => {
                    tracing::error!(status = ?status, "Failed to render move page form");
                    match status {
                        StatusCode::FORBIDDEN => {
                            Err(AppError::forbidden(
                                "You don't have permission to move this page",
                            ))
                        }
                        StatusCode::NOT_FOUND => Err(AppError::not_found("Page not found")),
                        StatusCode::INTERNAL_SERVER_ERROR => {
                            Err(AppError::internal_server_error(
                                "Failed to render move page form",
                            ))
                        }
                        _ => Err(AppError::new(status, "An error occurred")),
                    }
                }
            }
        } else {
            Ok(axum::response::Redirect::to("/.login").into_response())
        }
    })
}

fn handle_reorder(
    state: AppState,
    site: Site,
    page: Page,
    user: OptionalUser,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, AppError>> + Send>> {
    Box::pin(async move {
        if let OptionalUser(Some(current_user)) = user {
            match crate::handlers::reorder_page_handler(
                State(state.clone().into()),
                site,
                page,
                current_user,
            )
            .await
            {
                Ok(response) => Ok(response),
                Err(status) => {
                    tracing::error!(status = ?status, "Failed to render reorder page form");
                    match status {
                        StatusCode::FORBIDDEN => {
                            Err(AppError::forbidden(
                                "You don't have permission to reorder pages",
                            ))
                        }
                        StatusCode::NOT_FOUND => Err(AppError::not_found("Page not found")),
                        StatusCode::INTERNAL_SERVER_ERROR => {
                            Err(AppError::internal_server_error(
                                "Failed to render reorder page form",
                            ))
                        }
                        _ => Err(AppError::new(status, "An error occurred")),
                    }
                }
            }
        } else {
            Ok(axum::response::Redirect::to("/.login").into_response())
        }
    })
}

fn handle_delete_page(
    state: AppState,
    site: Site,
    page: Page,
    user: OptionalUser,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, AppError>> + Send>> {
    Box::pin(async move {
        if let OptionalUser(Some(current_user)) = user {
            match crate::handlers::delete_page_handler(state, site, page, current_user).await {
                Ok(response) => Ok(response),
                Err(status) => {
                    tracing::error!(status = ?status, "Failed to render delete page");
                    match status {
                        StatusCode::FORBIDDEN => {
                            Err(AppError::forbidden(
                                "You don't have permission to delete this page",
                            ))
                        }
                        StatusCode::NOT_FOUND => Err(AppError::not_found("Page not found")),
                        StatusCode::INTERNAL_SERVER_ERROR => {
                            Err(AppError::internal_server_error("Failed to render delete page"))
                        }
                        _ => Err(AppError::new(status, "An error occurred")),
                    }
                }
            }
        } else {
            Ok(axum::response::Redirect::to("/.login").into_response())
        }
    })
}

fn handle_add_component(
    _state: AppState,
    _site: Site,
    _page: Page,
    user: OptionalUser,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, AppError>> + Send>> {
    Box::pin(async move {
        if let OptionalUser(Some(_)) = user {
            // For now, this isn't implemented as GET
            Err(AppError::new(
                StatusCode::METHOD_NOT_ALLOWED,
                "Use POST to add components",
            ))
        } else {
            Ok(axum::response::Redirect::to("/.login").into_response())
        }
    })
}

fn handle_upload(
    _state: AppState,
    _site: Site,
    _page: Page,
    user: OptionalUser,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, AppError>> + Send>> {
    Box::pin(async move {
        if let OptionalUser(Some(_)) = user {
            Err(AppError::new(
                StatusCode::METHOD_NOT_ALLOWED,
                "Use POST for upload",
            ))
        } else {
            Ok(axum::response::Redirect::to("/.login").into_response())
        }
    })
}