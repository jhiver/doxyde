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

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    env,
    path::{Path, PathBuf},
};

pub mod defaults;
pub mod parser;

/// Main configuration structure containing all sub-configurations
///
/// # Example
///
/// ```rust
/// use doxyde_web::configuration::Configuration;
///
/// // Load configuration from environment variables
/// let config = Configuration::load().expect("Failed to load configuration");
///
/// // Access various configuration sections
/// println!("Server running on: {}", config.bind_addr());
/// println!("Upload max size: {} bytes", config.upload.max_size);
/// println!("Session timeout: {} minutes", config.session.timeout_minutes);
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Configuration {
    pub server: ServerConfig,
    pub session: SessionConfig,
    pub upload: UploadConfig,
    pub rate_limit: RateLimitConfig,
    pub security: SecurityConfig,
    pub path: PathConfig,
    pub cache: CacheConfig,
    pub mcp: McpConfig,
    pub database_url: String,
    pub development_mode: bool,
    pub multi_site_mode: bool,
}

/// Server configuration for host and port settings
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

/// Session management configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SessionConfig {
    pub timeout_minutes: i64,
    pub secure_cookies: bool,
    pub secret: Option<String>,
}

/// File upload configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UploadConfig {
    pub max_size: usize,
    pub directory: String,
    pub allowed_types: Option<Vec<String>>,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RateLimitConfig {
    pub login_attempts_per_minute: u32,
    pub api_requests_per_minute: u32,
}

/// Security configuration containing CSRF and headers settings
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SecurityConfig {
    pub csrf: CsrfConfig,
    pub headers: HeadersConfig,
}

/// CSRF protection configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CsrfConfig {
    pub enabled: bool,
    pub token_expiry_hours: u64,
    pub token_length: usize,
    pub header_name: String,
}

/// Security headers configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HeadersConfig {
    pub enable_hsts: bool,
    pub enable_csp: bool,
    pub enable_frame_options: bool,
    pub enable_content_type_options: bool,
    pub csp_content: Option<String>,
    pub hsts_content: Option<String>,
    pub frame_options_content: Option<String>,
    pub referrer_policy: Option<String>,
    pub permissions_policy: Option<String>,
}

/// Path configuration for various directories
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PathConfig {
    pub sites: String,
    pub templates: String,
}

/// Caching configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CacheConfig {
    pub static_files_max_age: u64,
}

/// MCP (Model Context Protocol) configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpConfig {
    pub oauth_token_expiry: u64,
}

impl Configuration {
    /// Load configuration from environment variables and optional configuration files
    ///
    /// Configuration loading order (later sources override earlier ones):
    /// 1. Default values
    /// 2. /etc/doxyde.conf (if exists)
    /// 3. ~/.doxyde.conf (if exists)
    /// 4. Environment variables
    pub fn load() -> Result<Self> {
        let project_root = Self::find_project_root()?;

        // Load and merge TOML configuration files
        let toml_config = Self::load_toml_config()?;

        let server = ServerConfig::load(&toml_config)?;
        let session = SessionConfig::load(&toml_config)?;
        let upload = UploadConfig::load(&project_root, &toml_config)?;
        let rate_limit = RateLimitConfig::load(&toml_config)?;
        let security = SecurityConfig::load(&toml_config)?;
        let path = PathConfig::load(&project_root, &toml_config)?;
        let cache = CacheConfig::load(&toml_config)?;
        let mcp = McpConfig::load(&toml_config)?;

        let database_url = env::var("DATABASE_URL")
            .or_else(|_| {
                toml_config
                    .database_url
                    .clone()
                    .ok_or_else(|| env::VarError::NotPresent)
            })
            .unwrap_or_else(|_| defaults::default_database_url());

        let development_mode = env::var("DEVELOPMENT_MODE")
            .or_else(|_| {
                toml_config
                    .development_mode
                    .map(|b| b.to_string())
                    .ok_or_else(|| env::VarError::NotPresent)
            })
            .unwrap_or_else(|_| defaults::default_development_mode().to_string())
            .parse()
            .unwrap_or_else(|_| defaults::default_development_mode());

        let multi_site_mode = env::var("MULTI_SITE_MODE")
            .or_else(|_| {
                toml_config
                    .multi_site_mode
                    .map(|b| b.to_string())
                    .ok_or_else(|| env::VarError::NotPresent)
            })
            .unwrap_or_else(|_| defaults::default_multi_site_mode().to_string())
            .parse()
            .unwrap_or_else(|_| defaults::default_multi_site_mode());

        Ok(Self {
            server,
            session,
            upload,
            rate_limit,
            security,
            path,
            cache,
            mcp,
            database_url,
            development_mode,
            multi_site_mode,
        })
    }

    /// Load and merge TOML configuration files from standard locations
    fn load_toml_config() -> Result<parser::TomlConfig> {
        let config_paths = parser::get_config_file_paths();
        let mut configs = Vec::new();

        for path in config_paths {
            match parser::parse_toml_file(&path) {
                Ok(config) => configs.push(config),
                Err(e) => {
                    // Log the error but continue - missing config files are OK
                    tracing::debug!("Could not load config file {}: {}", path.display(), e);
                }
            }
        }

        Ok(parser::merge_toml_configs(configs))
    }

    /// Convert the current configuration to TOML format
    pub fn to_toml(&self) -> Result<String> {
        let toml_config = parser::TomlConfig {
            server: Some(parser::TomlServerConfig {
                host: Some(self.server.host.clone()),
                port: Some(self.server.port),
            }),
            session: Some(parser::TomlSessionConfig {
                session_timeout_minutes: Some(self.session.timeout_minutes),
                secure_cookies: Some(self.session.secure_cookies),
                session_secret: self.session.secret.clone(),
            }),
            upload: Some(parser::TomlUploadConfig {
                max_upload_size: Some(self.upload.max_size),
                uploads_dir: Some(self.upload.directory.clone()),
                upload_allowed_types: self.upload.allowed_types.clone(),
            }),
            rate_limit: Some(parser::TomlRateLimitConfig {
                rate_limit_login_attempts: Some(self.rate_limit.login_attempts_per_minute),
                rate_limit_api_requests: Some(self.rate_limit.api_requests_per_minute),
            }),
            security: Some(parser::TomlSecurityConfig {
                csrf: Some(parser::TomlCsrfConfig {
                    csrf_enabled: Some(self.security.csrf.enabled),
                    csrf_token_expiry_hours: Some(self.security.csrf.token_expiry_hours),
                    csrf_token_length: Some(self.security.csrf.token_length),
                    csrf_header_name: Some(self.security.csrf.header_name.clone()),
                }),
                headers: Some(parser::TomlHeadersConfig {
                    security_headers_hsts: Some(self.security.headers.enable_hsts),
                    security_headers_csp: Some(self.security.headers.enable_csp),
                    security_headers_csp_content: self.security.headers.csp_content.clone(),
                    security_headers_frame_options: Some(
                        self.security.headers.enable_frame_options,
                    ),
                    security_headers_content_type_options: Some(
                        self.security.headers.enable_content_type_options,
                    ),
                    security_hsts_content: self.security.headers.hsts_content.clone(),
                    security_frame_options_content: self
                        .security
                        .headers
                        .frame_options_content
                        .clone(),
                    security_referrer_policy: self.security.headers.referrer_policy.clone(),
                    security_permissions_policy: self.security.headers.permissions_policy.clone(),
                }),
            }),
            path: Some(parser::TomlPathConfig {
                sites_dir: Some(self.path.sites.clone()),
                templates_dir: Some(self.path.templates.clone()),
            }),
            cache: Some(parser::TomlCacheConfig {
                cache_static_files_max_age: Some(self.cache.static_files_max_age),
            }),
            mcp: Some(parser::TomlMcpConfig {
                mcp_oauth_token_expiry: Some(self.mcp.oauth_token_expiry),
            }),
            database_url: Some(self.database_url.clone()),
            development_mode: Some(self.development_mode),
            multi_site_mode: Some(self.multi_site_mode),
        };

        toml::to_string_pretty(&toml_config).context("Failed to serialize configuration to TOML")
    }

    /// Find the project root by looking for the workspace Cargo.toml
    fn find_project_root() -> Result<PathBuf> {
        let mut current_dir = env::current_dir().context("Failed to get current directory")?;

        loop {
            let cargo_toml = current_dir.join("Cargo.toml");
            if cargo_toml.exists() {
                // Check if this is the workspace root
                let content =
                    std::fs::read_to_string(&cargo_toml).context("Failed to read Cargo.toml")?;
                if content.contains("[workspace]") {
                    return Ok(current_dir);
                }
            }

            // Move up one directory
            if !current_dir.pop() {
                // We've reached the root directory
                break;
            }
        }

        // If we can't find the workspace root, use current directory
        env::current_dir().context("Failed to determine project root")
    }

    /// Get the server bind address
    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.server.host, self.server.port)
    }
}

impl ServerConfig {
    fn load(toml_config: &parser::TomlConfig) -> Result<Self> {
        let host = env::var("HOST")
            .or_else(|_| {
                toml_config
                    .server
                    .as_ref()
                    .and_then(|s| s.host.clone())
                    .ok_or_else(|| env::VarError::NotPresent)
            })
            .unwrap_or_else(|_| defaults::default_host());

        let port = env::var("PORT")
            .or_else(|_| {
                toml_config
                    .server
                    .as_ref()
                    .and_then(|s| s.port.map(|p| p.to_string()))
                    .ok_or_else(|| env::VarError::NotPresent)
            })
            .unwrap_or_else(|_| defaults::default_port().to_string())
            .parse()
            .context("Invalid PORT environment variable")?;

        Ok(Self { host, port })
    }
}

impl SessionConfig {
    fn load(toml_config: &parser::TomlConfig) -> Result<Self> {
        let timeout_minutes = env::var("SESSION_TIMEOUT_MINUTES")
            .or_else(|_| {
                toml_config
                    .session
                    .as_ref()
                    .and_then(|s| s.session_timeout_minutes.map(|t| t.to_string()))
                    .ok_or_else(|| env::VarError::NotPresent)
            })
            .unwrap_or_else(|_| defaults::default_session_timeout_minutes().to_string())
            .parse()
            .context("Invalid SESSION_TIMEOUT_MINUTES environment variable")?;

        let secure_cookies = env::var("SECURE_COOKIES")
            .or_else(|_| {
                toml_config
                    .session
                    .as_ref()
                    .and_then(|s| s.secure_cookies.map(|c| c.to_string()))
                    .ok_or_else(|| env::VarError::NotPresent)
            })
            .unwrap_or_else(|_| defaults::default_secure_cookies().to_string())
            .parse()
            .context("Invalid SECURE_COOKIES environment variable")?;

        let secret = env::var("SESSION_SECRET")
            .ok()
            .or_else(|| {
                toml_config
                    .session
                    .as_ref()
                    .and_then(|s| s.session_secret.clone())
            })
            .or_else(|| defaults::default_session_secret());

        Ok(Self {
            timeout_minutes,
            secure_cookies,
            secret,
        })
    }
}

impl UploadConfig {
    fn load(_project_root: &Path, toml_config: &parser::TomlConfig) -> Result<Self> {
        let max_size = env::var("MAX_UPLOAD_SIZE")
            .or_else(|_| {
                toml_config
                    .upload
                    .as_ref()
                    .and_then(|u| u.max_upload_size.map(|s| s.to_string()))
                    .ok_or_else(|| env::VarError::NotPresent)
            })
            .unwrap_or_else(|_| defaults::default_max_upload_size().to_string())
            .parse()
            .context("Invalid MAX_UPLOAD_SIZE environment variable")?;

        let directory = env::var("UPLOADS_DIR")
            .or_else(|_| {
                toml_config
                    .upload
                    .as_ref()
                    .and_then(|u| u.uploads_dir.clone())
                    .ok_or_else(|| env::VarError::NotPresent)
            })
            .unwrap_or_else(|_| defaults::default_uploads_directory());

        let allowed_types = env::var("UPLOAD_ALLOWED_TYPES")
            .ok()
            .map(|types| types.split(',').map(|t| t.trim().to_string()).collect())
            .or_else(|| {
                toml_config
                    .upload
                    .as_ref()
                    .and_then(|u| u.upload_allowed_types.clone())
            })
            .or_else(|| defaults::default_upload_allowed_types());

        Ok(Self {
            max_size,
            directory,
            allowed_types,
        })
    }
}

impl RateLimitConfig {
    fn load(toml_config: &parser::TomlConfig) -> Result<Self> {
        let login_attempts_per_minute = env::var("RATE_LIMIT_LOGIN_ATTEMPTS")
            .or_else(|_| {
                toml_config
                    .rate_limit
                    .as_ref()
                    .and_then(|r| r.rate_limit_login_attempts.map(|a| a.to_string()))
                    .ok_or_else(|| env::VarError::NotPresent)
            })
            .unwrap_or_else(|_| defaults::default_login_attempts_per_minute().to_string())
            .parse()
            .context("Invalid RATE_LIMIT_LOGIN_ATTEMPTS environment variable")?;

        let api_requests_per_minute = env::var("RATE_LIMIT_API_REQUESTS")
            .or_else(|_| {
                toml_config
                    .rate_limit
                    .as_ref()
                    .and_then(|r| r.rate_limit_api_requests.map(|a| a.to_string()))
                    .ok_or_else(|| env::VarError::NotPresent)
            })
            .unwrap_or_else(|_| defaults::default_api_requests_per_minute().to_string())
            .parse()
            .context("Invalid RATE_LIMIT_API_REQUESTS environment variable")?;

        Ok(Self {
            login_attempts_per_minute,
            api_requests_per_minute,
        })
    }
}

impl SecurityConfig {
    fn load(toml_config: &parser::TomlConfig) -> Result<Self> {
        let csrf = CsrfConfig::load(toml_config)?;
        let headers = HeadersConfig::load(toml_config)?;

        Ok(Self { csrf, headers })
    }
}

impl CsrfConfig {
    fn load(toml_config: &parser::TomlConfig) -> Result<Self> {
        let enabled = env::var("CSRF_ENABLED")
            .or_else(|_| {
                toml_config
                    .security
                    .as_ref()
                    .and_then(|s| s.csrf.as_ref())
                    .and_then(|c| c.csrf_enabled.map(|e| e.to_string()))
                    .ok_or_else(|| env::VarError::NotPresent)
            })
            .unwrap_or_else(|_| defaults::default_csrf_enabled().to_string())
            .parse()
            .context("Invalid CSRF_ENABLED environment variable")?;

        let token_expiry_hours = env::var("CSRF_TOKEN_EXPIRY_HOURS")
            .or_else(|_| {
                toml_config
                    .security
                    .as_ref()
                    .and_then(|s| s.csrf.as_ref())
                    .and_then(|c| c.csrf_token_expiry_hours.map(|h| h.to_string()))
                    .ok_or_else(|| env::VarError::NotPresent)
            })
            .unwrap_or_else(|_| defaults::default_csrf_token_expiry_hours().to_string())
            .parse()
            .context("Invalid CSRF_TOKEN_EXPIRY_HOURS environment variable")?;

        let token_length = env::var("CSRF_TOKEN_LENGTH")
            .or_else(|_| {
                toml_config
                    .security
                    .as_ref()
                    .and_then(|s| s.csrf.as_ref())
                    .and_then(|c| c.csrf_token_length.map(|l| l.to_string()))
                    .ok_or_else(|| env::VarError::NotPresent)
            })
            .unwrap_or_else(|_| defaults::default_csrf_token_length().to_string())
            .parse()
            .context("Invalid CSRF_TOKEN_LENGTH environment variable")?;

        let header_name = env::var("CSRF_HEADER_NAME")
            .or_else(|_| {
                toml_config
                    .security
                    .as_ref()
                    .and_then(|s| s.csrf.as_ref())
                    .and_then(|c| c.csrf_header_name.clone())
                    .ok_or_else(|| env::VarError::NotPresent)
            })
            .unwrap_or_else(|_| defaults::default_csrf_header_name());

        Ok(Self {
            enabled,
            token_expiry_hours,
            token_length,
            header_name,
        })
    }
}

impl HeadersConfig {
    fn load(toml_config: &parser::TomlConfig) -> Result<Self> {
        let headers_ref = toml_config
            .security
            .as_ref()
            .and_then(|s| s.headers.as_ref());

        let enable_hsts = env::var("SECURITY_HEADERS_HSTS")
            .or_else(|_| {
                headers_ref
                    .and_then(|h| h.security_headers_hsts.map(|b| b.to_string()))
                    .ok_or_else(|| env::VarError::NotPresent)
            })
            .unwrap_or_else(|_| defaults::default_enable_hsts().to_string())
            .parse()
            .context("Invalid SECURITY_HEADERS_HSTS environment variable")?;

        let enable_csp = env::var("SECURITY_HEADERS_CSP")
            .or_else(|_| {
                headers_ref
                    .and_then(|h| h.security_headers_csp.map(|b| b.to_string()))
                    .ok_or_else(|| env::VarError::NotPresent)
            })
            .unwrap_or_else(|_| defaults::default_enable_csp().to_string())
            .parse()
            .context("Invalid SECURITY_HEADERS_CSP environment variable")?;

        let enable_frame_options = env::var("SECURITY_HEADERS_FRAME_OPTIONS")
            .or_else(|_| {
                headers_ref
                    .and_then(|h| h.security_headers_frame_options.map(|b| b.to_string()))
                    .ok_or_else(|| env::VarError::NotPresent)
            })
            .unwrap_or_else(|_| defaults::default_enable_frame_options().to_string())
            .parse()
            .context("Invalid SECURITY_HEADERS_FRAME_OPTIONS environment variable")?;

        let enable_content_type_options = env::var("SECURITY_HEADERS_CONTENT_TYPE_OPTIONS")
            .or_else(|_| {
                headers_ref
                    .and_then(|h| {
                        h.security_headers_content_type_options
                            .map(|b| b.to_string())
                    })
                    .ok_or_else(|| env::VarError::NotPresent)
            })
            .unwrap_or_else(|_| defaults::default_enable_content_type_options().to_string())
            .parse()
            .context("Invalid SECURITY_HEADERS_CONTENT_TYPE_OPTIONS environment variable")?;

        let csp_content = env::var("SECURITY_HEADERS_CSP_CONTENT")
            .ok()
            .or_else(|| headers_ref.and_then(|h| h.security_headers_csp_content.clone()))
            .or_else(|| defaults::default_csp_content());

        let hsts_content = env::var("SECURITY_HSTS_CONTENT")
            .ok()
            .or_else(|| headers_ref.and_then(|h| h.security_hsts_content.clone()))
            .or_else(|| defaults::default_hsts_content());

        let frame_options_content = env::var("SECURITY_FRAME_OPTIONS_CONTENT")
            .ok()
            .or_else(|| headers_ref.and_then(|h| h.security_frame_options_content.clone()))
            .or_else(|| defaults::default_frame_options_content());

        let referrer_policy = env::var("SECURITY_REFERRER_POLICY")
            .ok()
            .or_else(|| headers_ref.and_then(|h| h.security_referrer_policy.clone()))
            .or_else(|| defaults::default_referrer_policy());

        let permissions_policy = env::var("SECURITY_PERMISSIONS_POLICY")
            .ok()
            .or_else(|| headers_ref.and_then(|h| h.security_permissions_policy.clone()))
            .or_else(|| defaults::default_permissions_policy());

        Ok(Self {
            enable_hsts,
            enable_csp,
            enable_frame_options,
            enable_content_type_options,
            csp_content,
            hsts_content,
            frame_options_content,
            referrer_policy,
            permissions_policy,
        })
    }
}

impl PathConfig {
    fn load(project_root: &Path, toml_config: &parser::TomlConfig) -> Result<Self> {
        let sites = env::var("SITES_DIR")
            .or_else(|_| {
                toml_config
                    .path
                    .as_ref()
                    .and_then(|p| p.sites_dir.clone())
                    .ok_or_else(|| env::VarError::NotPresent)
            })
            .unwrap_or_else(|_| defaults::default_sites_directory(project_root));

        let templates = env::var("TEMPLATES_DIR")
            .or_else(|_| {
                toml_config
                    .path
                    .as_ref()
                    .and_then(|p| p.templates_dir.clone())
                    .ok_or_else(|| env::VarError::NotPresent)
            })
            .unwrap_or_else(|_| defaults::default_templates_directory(project_root));

        Ok(Self { sites, templates })
    }
}

impl CacheConfig {
    fn load(toml_config: &parser::TomlConfig) -> Result<Self> {
        let static_files_max_age = env::var("CACHE_STATIC_FILES_MAX_AGE")
            .or_else(|_| {
                toml_config
                    .cache
                    .as_ref()
                    .and_then(|c| c.cache_static_files_max_age.map(|a| a.to_string()))
                    .ok_or_else(|| env::VarError::NotPresent)
            })
            .unwrap_or_else(|_| defaults::default_static_files_max_age().to_string())
            .parse()
            .context("Invalid CACHE_STATIC_FILES_MAX_AGE environment variable")?;

        Ok(Self {
            static_files_max_age,
        })
    }
}

impl McpConfig {
    fn load(toml_config: &parser::TomlConfig) -> Result<Self> {
        let oauth_token_expiry = env::var("MCP_OAUTH_TOKEN_EXPIRY")
            .or_else(|_| {
                toml_config
                    .mcp
                    .as_ref()
                    .and_then(|m| m.mcp_oauth_token_expiry.map(|e| e.to_string()))
                    .ok_or_else(|| env::VarError::NotPresent)
            })
            .unwrap_or_else(|_| defaults::default_mcp_oauth_token_expiry().to_string())
            .parse()
            .context("Invalid MCP_OAUTH_TOKEN_EXPIRY environment variable")?;

        Ok(Self { oauth_token_expiry })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    #[test]
    #[serial]
    fn test_server_config_load_defaults() {
        // Clear environment variables that might affect the test
        env::remove_var("HOST");
        env::remove_var("PORT");

        let toml_config = parser::TomlConfig::default();
        let config = ServerConfig::load(&toml_config).expect("Should load default server config");
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 3000);
    }

    #[test]
    #[serial]
    fn test_server_config_load_from_env() {
        env::set_var("HOST", "127.0.0.1");
        env::set_var("PORT", "8080");

        let toml_config = parser::TomlConfig::default();
        let config = ServerConfig::load(&toml_config).expect("Should load server config from env");
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);

        // Clean up
        env::remove_var("HOST");
        env::remove_var("PORT");
    }

    #[test]
    #[serial]
    fn test_server_config_invalid_port() {
        env::set_var("PORT", "invalid");

        let toml_config = parser::TomlConfig::default();
        let result = ServerConfig::load(&toml_config);
        assert!(result.is_err());

        env::remove_var("PORT");
    }

    #[test]
    #[serial]
    fn test_session_config_load_defaults() {
        env::remove_var("SESSION_TIMEOUT_MINUTES");
        env::remove_var("SECURE_COOKIES");
        env::remove_var("SESSION_SECRET");

        let toml_config = parser::TomlConfig::default();
        let config = SessionConfig::load(&toml_config).expect("Should load default session config");
        assert_eq!(config.timeout_minutes, 1440);
        assert!(config.secure_cookies);
        assert!(config.secret.is_some());
    }

    #[test]
    #[serial]
    fn test_session_config_load_from_env() {
        env::set_var("SESSION_TIMEOUT_MINUTES", "720");
        env::set_var("SECURE_COOKIES", "false");
        env::set_var("SESSION_SECRET", "test-secret");

        let toml_config = parser::TomlConfig::default();
        let config =
            SessionConfig::load(&toml_config).expect("Should load session config from env");
        assert_eq!(config.timeout_minutes, 720);
        assert!(!config.secure_cookies);
        assert_eq!(config.secret.as_ref().unwrap(), "test-secret");

        // Clean up
        env::remove_var("SESSION_TIMEOUT_MINUTES");
        env::remove_var("SECURE_COOKIES");
        env::remove_var("SESSION_SECRET");
    }

    #[test]
    #[serial]
    fn test_upload_config_load_defaults() {
        // Save current state
        let original_max_size = env::var("MAX_UPLOAD_SIZE").ok();
        let original_uploads_dir = env::var("UPLOADS_DIR").ok();
        let original_allowed_types = env::var("UPLOAD_ALLOWED_TYPES").ok();

        // Clean up environment variables that might affect this test
        env::remove_var("MAX_UPLOAD_SIZE");
        env::remove_var("UPLOADS_DIR");
        env::remove_var("UPLOAD_ALLOWED_TYPES");

        let project_root = PathBuf::from("/tmp");
        let toml_config = parser::TomlConfig::default();
        let config = UploadConfig::load(&project_root, &toml_config)
            .expect("Should load default upload config");

        assert_eq!(config.max_size, 10_485_760);
        assert!(config.allowed_types.is_none());

        // Restore original state
        if let Some(val) = original_max_size {
            env::set_var("MAX_UPLOAD_SIZE", val);
        }
        if let Some(val) = original_uploads_dir {
            env::set_var("UPLOADS_DIR", val);
        }
        if let Some(val) = original_allowed_types {
            env::set_var("UPLOAD_ALLOWED_TYPES", val);
        }
    }

    #[test]
    #[serial]
    fn test_upload_config_with_allowed_types() {
        // Save current state
        let original_allowed_types = env::var("UPLOAD_ALLOWED_TYPES").ok();

        // Set up test environment
        env::set_var("UPLOAD_ALLOWED_TYPES", "jpg,png,gif,pdf");

        let project_root = PathBuf::from("/tmp");
        let toml_config = parser::TomlConfig::default();
        let config =
            UploadConfig::load(&project_root, &toml_config).expect("Should load upload config");

        let allowed_types = config.allowed_types.expect("Should have allowed types");
        assert_eq!(allowed_types, vec!["jpg", "png", "gif", "pdf"]);

        // Restore original state
        if let Some(val) = original_allowed_types {
            env::set_var("UPLOAD_ALLOWED_TYPES", val);
        } else {
            env::remove_var("UPLOAD_ALLOWED_TYPES");
        }
    }

    #[test]
    #[serial]
    fn test_rate_limit_config_load_defaults() {
        env::remove_var("RATE_LIMIT_LOGIN_ATTEMPTS");
        env::remove_var("RATE_LIMIT_API_REQUESTS");

        let toml_config = parser::TomlConfig::default();
        let config =
            RateLimitConfig::load(&toml_config).expect("Should load default rate limit config");
        assert_eq!(config.login_attempts_per_minute, 5);
        assert_eq!(config.api_requests_per_minute, 60);
    }

    #[test]
    #[serial]
    fn test_csrf_config_load_defaults() {
        env::remove_var("CSRF_ENABLED");
        env::remove_var("CSRF_TOKEN_EXPIRY_HOURS");

        let toml_config = parser::TomlConfig::default();
        let config = CsrfConfig::load(&toml_config).expect("Should load default CSRF config");
        assert!(config.enabled);
        assert_eq!(config.token_expiry_hours, 24);
    }

    #[test]
    #[serial]
    fn test_headers_config_load_defaults() {
        env::remove_var("SECURITY_HEADERS_HSTS");
        env::remove_var("SECURITY_HEADERS_CSP");
        env::remove_var("SECURITY_HEADERS_FRAME_OPTIONS");
        env::remove_var("SECURITY_HEADERS_CONTENT_TYPE_OPTIONS");

        let toml_config = parser::TomlConfig::default();
        let config = HeadersConfig::load(&toml_config).expect("Should load default headers config");
        assert!(config.enable_hsts);
        assert!(config.enable_csp);
        assert!(config.enable_frame_options);
        assert!(config.enable_content_type_options);
    }

    #[test]
    #[serial]
    fn test_cache_config_load_defaults() {
        env::remove_var("CACHE_STATIC_FILES_MAX_AGE");

        let toml_config = parser::TomlConfig::default();
        let config = CacheConfig::load(&toml_config).expect("Should load default cache config");
        assert_eq!(config.static_files_max_age, 31_536_000);
    }

    #[test]
    #[serial]
    fn test_mcp_config_load_defaults() {
        env::remove_var("MCP_OAUTH_TOKEN_EXPIRY");

        let toml_config = parser::TomlConfig::default();
        let config = McpConfig::load(&toml_config).expect("Should load default MCP config");
        assert_eq!(config.oauth_token_expiry, 3600);
    }

    #[test]
    fn test_configuration_bind_addr() {
        let config = Configuration {
            server: ServerConfig {
                host: "127.0.0.1".to_string(),
                port: 8080,
            },
            session: SessionConfig {
                timeout_minutes: 1440,
                secure_cookies: true,
                secret: Some("test".to_string()),
            },
            upload: UploadConfig {
                max_size: 10_485_760,
                directory: "/tmp".to_string(),
                allowed_types: None,
            },
            rate_limit: RateLimitConfig {
                login_attempts_per_minute: 5,
                api_requests_per_minute: 60,
            },
            security: SecurityConfig {
                csrf: CsrfConfig {
                    enabled: true,
                    token_expiry_hours: 24,
                    token_length: 32,
                    header_name: "X-CSRF-Token".to_string(),
                },
                headers: HeadersConfig {
                    enable_hsts: true,
                    enable_csp: true,
                    enable_frame_options: true,
                    enable_content_type_options: true,
                    csp_content: None,
                    hsts_content: None,
                    frame_options_content: None,
                    referrer_policy: None,
                    permissions_policy: None,
                },
            },
            path: PathConfig {
                sites: "/tmp/sites".to_string(),
                templates: "/tmp/templates".to_string(),
            },
            cache: CacheConfig {
                static_files_max_age: 3600,
            },
            mcp: McpConfig {
                oauth_token_expiry: 3600,
            },
            database_url: "sqlite:test.db".to_string(),
            development_mode: false,
            multi_site_mode: true,
        };

        assert_eq!(config.bind_addr(), "127.0.0.1:8080");
    }

    #[test]
    #[serial]
    fn test_configuration_load_integration() {
        // Save current environment state
        let env_vars = [
            "DATABASE_URL",
            "DEVELOPMENT_MODE",
            "HOST",
            "PORT",
            "SESSION_TIMEOUT_MINUTES",
            "SECURE_COOKIES",
            "SESSION_SECRET",
            "MAX_UPLOAD_SIZE",
            "UPLOADS_DIR",
            "UPLOAD_ALLOWED_TYPES",
            "RATE_LIMIT_LOGIN_ATTEMPTS",
            "RATE_LIMIT_API_REQUESTS",
            "CSRF_ENABLED",
            "CSRF_TOKEN_EXPIRY_HOURS",
            "SECURITY_HEADERS_HSTS",
            "SECURITY_HEADERS_CSP",
            "SECURITY_HEADERS_FRAME_OPTIONS",
            "SECURITY_HEADERS_CONTENT_TYPE_OPTIONS",
            "SECURITY_HEADERS_CSP_CONTENT",
            "SECURITY_HSTS_CONTENT",
            "SECURITY_FRAME_OPTIONS_CONTENT",
            "SECURITY_REFERRER_POLICY",
            "SECURITY_PERMISSIONS_POLICY",
            "SITES_DIR",
            "TEMPLATES_DIR",
            "CACHE_STATIC_FILES_MAX_AGE",
            "MCP_OAUTH_TOKEN_EXPIRY",
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
        let config = Configuration::load().expect("Should load configuration with defaults");

        // Verify defaults are set correctly
        assert_eq!(config.database_url, "sqlite:doxyde.db");
        assert!(!config.development_mode);
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 3000);
        assert_eq!(config.session.timeout_minutes, 1440);
        assert!(config.session.secure_cookies);
        assert!(config.session.secret.is_some());
        assert_eq!(config.upload.max_size, 10_485_760);
        assert!(config.upload.allowed_types.is_none());
        assert_eq!(config.rate_limit.login_attempts_per_minute, 5);
        assert_eq!(config.rate_limit.api_requests_per_minute, 60);
        assert!(config.security.csrf.enabled);
        assert_eq!(config.security.csrf.token_expiry_hours, 24);
        assert!(config.security.headers.enable_hsts);
        assert!(config.security.headers.enable_csp);
        assert_eq!(config.cache.static_files_max_age, 31_536_000);
        assert_eq!(config.mcp.oauth_token_expiry, 3600);

        // Test bind_addr method
        assert_eq!(config.bind_addr(), "0.0.0.0:3000");

        // Restore environment state
        for (var, value) in saved_vars {
            if let Some(val) = value {
                env::set_var(&var, val);
            }
        }
    }
}
