pub mod authorization;
pub mod client_registration;
pub mod discovery;
pub mod errors;
pub mod models;
pub mod token;

// Re-export commonly used types
pub use errors::{AuthorizationError, BearerError, OAuthErrorResponse};
pub use models::{AccessToken, AuthorizationCode, OAuthClient, OAuthError, RefreshToken};

#[cfg(test)]
mod tests;
