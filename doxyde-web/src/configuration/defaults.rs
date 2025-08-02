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

//! Default configuration values for Doxyde
//!
//! This module provides all default values used throughout the configuration system.
//! Each function returns the default value for a specific configuration field.

use std::{env, path::PathBuf};
use uuid::Uuid;

// Server defaults
pub fn default_host() -> String {
    "0.0.0.0".to_string()
}

pub fn default_port() -> u16 {
    3000
}

// Session defaults
pub fn default_session_timeout_minutes() -> i64 {
    1440 // 24 hours
}

pub fn default_secure_cookies() -> bool {
    true
}

pub fn default_session_secret() -> Option<String> {
    // Generate a random secret for development if none provided
    Some(Uuid::new_v4().to_string())
}

// Upload defaults
pub fn default_max_upload_size() -> usize {
    10_485_760 // 10MB
}

pub fn default_uploads_directory() -> String {
    env::var("HOME")
        .map(|home| PathBuf::from(home).join(".doxyde").join("uploads"))
        .unwrap_or_else(|_| PathBuf::from("/var/doxyde/uploads"))
        .to_string_lossy()
        .to_string()
}

pub fn default_upload_allowed_types() -> Option<Vec<String>> {
    None
}

// Rate limit defaults
pub fn default_login_attempts_per_minute() -> u32 {
    5
}

pub fn default_api_requests_per_minute() -> u32 {
    60
}

// CSRF defaults
pub fn default_csrf_enabled() -> bool {
    true
}

pub fn default_csrf_token_expiry_hours() -> u64 {
    24
}

pub fn default_csrf_token_length() -> usize {
    32
}

pub fn default_csrf_header_name() -> String {
    "X-CSRF-Token".to_string()
}

// Security headers defaults
pub fn default_enable_hsts() -> bool {
    true
}

pub fn default_enable_csp() -> bool {
    true
}

pub fn default_enable_frame_options() -> bool {
    true
}

pub fn default_enable_content_type_options() -> bool {
    true
}

// Security header content defaults
pub fn default_csp_content() -> Option<String> {
    Some("default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline' https://fonts.googleapis.com; img-src 'self' data: https:; font-src 'self' https://fonts.gstatic.com; connect-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self';".to_string())
}

pub fn default_hsts_content() -> Option<String> {
    Some("max-age=31536000; includeSubDomains".to_string())
}

pub fn default_frame_options_content() -> Option<String> {
    Some("DENY".to_string())
}

pub fn default_referrer_policy() -> Option<String> {
    Some("strict-origin-when-cross-origin".to_string())
}

pub fn default_permissions_policy() -> Option<String> {
    Some("geolocation=(), camera=(), microphone=()".to_string())
}

// Path defaults
pub fn default_sites_directory(project_root: &std::path::Path) -> String {
    project_root.join("sites").to_string_lossy().to_string()
}

pub fn default_templates_directory(project_root: &std::path::Path) -> String {
    project_root.join("templates").to_string_lossy().to_string()
}

// Cache defaults
pub fn default_static_files_max_age() -> u64 {
    31_536_000 // 1 year in seconds
}

// MCP defaults
pub fn default_mcp_oauth_token_expiry() -> u64 {
    3600 // 1 hour
}

// Database defaults
pub fn default_database_url() -> String {
    "sqlite:doxyde.db".to_string()
}

// Development mode default
pub fn default_development_mode() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_server_defaults() {
        assert_eq!(default_host(), "0.0.0.0");
        assert_eq!(default_port(), 3000);
    }

    #[test]
    fn test_session_defaults() {
        assert_eq!(default_session_timeout_minutes(), 1440);
        assert!(default_secure_cookies());
        assert!(default_session_secret().is_some());
        // Verify the secret is a valid UUID format (36 chars with hyphens)
        let secret = default_session_secret().unwrap();
        assert_eq!(secret.len(), 36);
        assert_eq!(secret.chars().filter(|&c| c == '-').count(), 4);
    }

    #[test]
    fn test_upload_defaults() {
        assert_eq!(default_max_upload_size(), 10_485_760);
        assert!(
            default_uploads_directory().contains(".doxyde")
                || default_uploads_directory().contains("/var/doxyde")
        );
        assert!(default_upload_allowed_types().is_none());
    }

    #[test]
    fn test_rate_limit_defaults() {
        assert_eq!(default_login_attempts_per_minute(), 5);
        assert_eq!(default_api_requests_per_minute(), 60);
    }

    #[test]
    fn test_csrf_defaults() {
        assert!(default_csrf_enabled());
        assert_eq!(default_csrf_token_expiry_hours(), 24);
        assert_eq!(default_csrf_token_length(), 32);
        assert_eq!(default_csrf_header_name(), "X-CSRF-Token");
    }

    #[test]
    fn test_security_headers_defaults() {
        assert!(default_enable_hsts());
        assert!(default_enable_csp());
        assert!(default_enable_frame_options());
        assert!(default_enable_content_type_options());
    }

    #[test]
    fn test_security_header_content_defaults() {
        let csp = default_csp_content().unwrap();
        assert!(csp.contains("default-src 'self'"));
        assert!(csp.contains("frame-ancestors 'none'"));

        let hsts = default_hsts_content().unwrap();
        assert!(hsts.contains("max-age=31536000"));
        assert!(hsts.contains("includeSubDomains"));

        let frame_options = default_frame_options_content().unwrap();
        assert_eq!(frame_options, "DENY");

        let referrer = default_referrer_policy().unwrap();
        assert_eq!(referrer, "strict-origin-when-cross-origin");

        let permissions = default_permissions_policy().unwrap();
        assert!(permissions.contains("geolocation=()"));
        assert!(permissions.contains("camera=()"));
        assert!(permissions.contains("microphone=()"));
    }

    #[test]
    fn test_path_defaults() {
        let project_root = Path::new("/tmp/project");
        assert_eq!(default_sites_directory(project_root), "/tmp/project/sites");
        assert_eq!(
            default_templates_directory(project_root),
            "/tmp/project/templates"
        );
    }

    #[test]
    fn test_cache_defaults() {
        assert_eq!(default_static_files_max_age(), 31_536_000);
    }

    #[test]
    fn test_mcp_defaults() {
        assert_eq!(default_mcp_oauth_token_expiry(), 3600);
    }

    #[test]
    fn test_database_defaults() {
        assert_eq!(default_database_url(), "sqlite:doxyde.db");
        assert!(!default_development_mode());
    }
}
