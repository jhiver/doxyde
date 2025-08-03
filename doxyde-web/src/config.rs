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

use crate::configuration::Configuration;
use anyhow::Result;
use uuid;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub host: String,
    pub port: u16,
    pub templates_dir: String,
    pub session_secret: String,
    pub development_mode: bool,
    pub uploads_dir: String,
    pub max_upload_size: usize,
    pub secure_cookies: bool,
    pub session_timeout_minutes: i64,
    pub login_attempts_per_minute: u32,
    pub api_requests_per_minute: u32,
    pub csrf_enabled: bool,
    pub csrf_token_expiry_hours: u64,
    pub csrf_token_length: usize,
    pub csrf_header_name: String,
    pub static_files_max_age: u64,
    pub oauth_token_expiry: u64,
    pub sites_directory: String,
    pub multi_site_mode: bool,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        // Load the new Configuration system
        let config = Configuration::load()?;

        // Map values from the new Configuration to the old Config struct
        Ok(Self {
            database_url: config.database_url,
            host: config.server.host,
            port: config.server.port,
            templates_dir: config.path.templates,
            session_secret: config.session.secret.unwrap_or_else(|| {
                // Generate a random secret for development if none was provided
                uuid::Uuid::new_v4().to_string()
            }),
            development_mode: config.development_mode,
            uploads_dir: config.upload.directory,
            max_upload_size: config.upload.max_size,
            secure_cookies: config.session.secure_cookies,
            session_timeout_minutes: config.session.timeout_minutes,
            login_attempts_per_minute: config.rate_limit.login_attempts_per_minute,
            api_requests_per_minute: config.rate_limit.api_requests_per_minute,
            csrf_enabled: config.security.csrf.enabled,
            csrf_token_expiry_hours: config.security.csrf.token_expiry_hours,
            csrf_token_length: config.security.csrf.token_length,
            csrf_header_name: config.security.csrf.header_name,
            static_files_max_age: config.cache.static_files_max_age,
            oauth_token_expiry: config.mcp.oauth_token_expiry,
            sites_directory: config.path.sites,
            multi_site_mode: config.multi_site_mode,
        })
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    pub fn get_sites_directory(&self) -> Result<std::path::PathBuf> {
        Ok(std::path::PathBuf::from(&self.sites_directory))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    #[test]
    #[serial]
    fn test_config_from_env_uses_new_configuration_system() {
        // Save current environment state
        let env_vars = [
            "DATABASE_URL",
            "HOST",
            "PORT",
            "DEVELOPMENT_MODE",
            "SESSION_SECRET",
            "SESSION_TIMEOUT_MINUTES",
            "SECURE_COOKIES",
            "MAX_UPLOAD_SIZE",
            "UPLOADS_DIR",
            "TEMPLATES_DIR",
        ];

        let saved_vars: Vec<(String, Option<String>)> = env_vars
            .iter()
            .map(|var| (var.to_string(), env::var(var).ok()))
            .collect();

        // Clear all environment variables
        for var in &env_vars {
            env::remove_var(var);
        }

        // Test loading with defaults
        let config = Config::from_env().expect("Should load config with defaults");

        // Verify that values come from the new Configuration system defaults
        assert_eq!(config.host, "0.0.0.0"); // Should match Configuration defaults
        assert_eq!(config.port, 3000);
        assert!(!config.development_mode);
        assert_eq!(config.session_timeout_minutes, 1440);
        assert!(config.secure_cookies);
        assert_eq!(config.max_upload_size, 10_485_760);
        assert_eq!(config.login_attempts_per_minute, 5);
        assert_eq!(config.api_requests_per_minute, 60);

        // Verify CSRF configuration defaults
        assert!(config.csrf_enabled);
        assert_eq!(config.csrf_token_expiry_hours, 24);
        assert_eq!(config.csrf_token_length, 32);
        assert_eq!(config.csrf_header_name, "X-CSRF-Token");

        // Verify DATABASE_URL handling remains the same (backward compatibility)
        assert_eq!(config.database_url, "sqlite:doxyde.db");

        // Test bind_addr method still works
        assert_eq!(config.bind_addr(), "0.0.0.0:3000");

        // Restore environment state
        for (var, value) in saved_vars {
            if let Some(val) = value {
                env::set_var(&var, val);
            }
        }
    }

    #[test]
    #[serial]
    fn test_config_database_url_backward_compatibility() {
        // Save original state
        let original_database_url = env::var("DATABASE_URL").ok();

        // Set DATABASE_URL environment variable
        env::set_var("DATABASE_URL", "sqlite:test.db");

        let config = Config::from_env().expect("Should load config");

        // DATABASE_URL should be passed through from environment variable
        assert_eq!(config.database_url, "sqlite:test.db");

        // Restore original state
        if let Some(url) = original_database_url {
            env::set_var("DATABASE_URL", url);
        } else {
            env::remove_var("DATABASE_URL");
        }
    }

    #[test]
    #[serial]
    fn test_config_env_override_still_works() {
        // Save original state
        let original_host = env::var("HOST").ok();
        let original_port = env::var("PORT").ok();
        let original_dev_mode = env::var("DEVELOPMENT_MODE").ok();

        // Set some environment variables that should override defaults
        env::set_var("HOST", "127.0.0.1");
        env::set_var("PORT", "8080");
        env::set_var("DEVELOPMENT_MODE", "true");

        let config = Config::from_env().expect("Should load config with env overrides");

        // These should come from the new Configuration system which respects env vars
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);
        assert!(config.development_mode);
        assert_eq!(config.bind_addr(), "127.0.0.1:8080");

        // CSRF should still use defaults even when other env vars are set
        assert!(config.csrf_enabled);
        assert_eq!(config.csrf_token_length, 32);
        assert_eq!(config.csrf_header_name, "X-CSRF-Token");

        // Restore original state
        if let Some(host) = original_host {
            env::set_var("HOST", host);
        } else {
            env::remove_var("HOST");
        }

        if let Some(port) = original_port {
            env::set_var("PORT", port);
        } else {
            env::remove_var("PORT");
        }

        if let Some(dev_mode) = original_dev_mode {
            env::set_var("DEVELOPMENT_MODE", dev_mode);
        } else {
            env::remove_var("DEVELOPMENT_MODE");
        }
    }

    #[test]
    #[serial]
    fn test_config_csrf_env_overrides() {
        // Save original state
        let original_csrf_enabled = env::var("CSRF_ENABLED").ok();
        let original_csrf_token_length = env::var("CSRF_TOKEN_LENGTH").ok();
        let original_csrf_header_name = env::var("CSRF_HEADER_NAME").ok();

        // Set CSRF environment variables
        env::set_var("CSRF_ENABLED", "false");
        env::set_var("CSRF_TOKEN_LENGTH", "64");
        env::set_var("CSRF_HEADER_NAME", "X-Custom-CSRF-Token");

        let config = Config::from_env().expect("Should load config with CSRF env overrides");

        // CSRF config should come from environment variables
        assert!(!config.csrf_enabled);
        assert_eq!(config.csrf_token_length, 64);
        assert_eq!(config.csrf_header_name, "X-Custom-CSRF-Token");

        // Restore original state
        if let Some(enabled) = original_csrf_enabled {
            env::set_var("CSRF_ENABLED", enabled);
        } else {
            env::remove_var("CSRF_ENABLED");
        }

        if let Some(length) = original_csrf_token_length {
            env::set_var("CSRF_TOKEN_LENGTH", length);
        } else {
            env::remove_var("CSRF_TOKEN_LENGTH");
        }

        if let Some(header_name) = original_csrf_header_name {
            env::set_var("CSRF_HEADER_NAME", header_name);
        } else {
            env::remove_var("CSRF_HEADER_NAME");
        }
    }
}
