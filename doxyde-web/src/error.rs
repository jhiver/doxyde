use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::fmt;

/// Application error type that includes context for better debugging
#[derive(Debug)]
pub struct AppError {
    pub status: StatusCode,
    pub message: String,
    pub details: Option<String>,
}

impl AppError {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
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

        // Return a simple error response to the client
        (self.status, self.message).into_response()
    }
}

// Conversion from anyhow::Error
impl From<anyhow::Error> for AppError {
    fn from(err: anyhow::Error) -> Self {
        tracing::error!("Anyhow error: {:?}", err);
        Self::internal_server_error("Internal server error").with_details(format!("{:?}", err))
    }
}
