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

//! TOML configuration parser for Doxyde
//!
//! This module provides functionality to read and parse TOML configuration files
//! from standard locations and merge them with environment variables.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Raw TOML configuration structure that mirrors the main Configuration
/// but with all fields optional to support partial configuration files
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TomlConfig {
    #[serde(flatten)]
    pub server: Option<TomlServerConfig>,
    #[serde(flatten)]
    pub session: Option<TomlSessionConfig>,
    #[serde(flatten)]
    pub upload: Option<TomlUploadConfig>,
    #[serde(flatten)]
    pub rate_limit: Option<TomlRateLimitConfig>,
    #[serde(flatten)]
    pub security: Option<TomlSecurityConfig>,
    #[serde(flatten)]
    pub path: Option<TomlPathConfig>,
    #[serde(flatten)]
    pub cache: Option<TomlCacheConfig>,
    #[serde(flatten)]
    pub mcp: Option<TomlMcpConfig>,
    pub database_url: Option<String>,
    pub development_mode: Option<bool>,
    pub multi_site_mode: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TomlServerConfig {
    pub host: Option<String>,
    pub port: Option<u16>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TomlSessionConfig {
    pub session_timeout_minutes: Option<i64>,
    pub secure_cookies: Option<bool>,
    pub session_secret: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TomlUploadConfig {
    pub max_upload_size: Option<usize>,
    pub uploads_dir: Option<String>,
    pub upload_allowed_types: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TomlRateLimitConfig {
    pub rate_limit_login_attempts: Option<u32>,
    pub rate_limit_api_requests: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TomlSecurityConfig {
    #[serde(flatten)]
    pub csrf: Option<TomlCsrfConfig>,
    #[serde(flatten)]
    pub headers: Option<TomlHeadersConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TomlCsrfConfig {
    pub csrf_enabled: Option<bool>,
    pub csrf_token_expiry_hours: Option<u64>,
    pub csrf_token_length: Option<usize>,
    pub csrf_header_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TomlHeadersConfig {
    pub security_headers_hsts: Option<bool>,
    pub security_headers_csp: Option<bool>,
    pub security_headers_csp_content: Option<String>,
    pub security_headers_frame_options: Option<bool>,
    pub security_headers_content_type_options: Option<bool>,
    pub security_hsts_content: Option<String>,
    pub security_frame_options_content: Option<String>,
    pub security_referrer_policy: Option<String>,
    pub security_permissions_policy: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TomlPathConfig {
    pub sites_dir: Option<String>,
    pub templates_dir: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TomlCacheConfig {
    pub cache_static_files_max_age: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TomlMcpConfig {
    pub mcp_oauth_token_expiry: Option<u64>,
}

impl Default for TomlConfig {
    fn default() -> Self {
        Self {
            server: None,
            session: None,
            upload: None,
            rate_limit: None,
            security: None,
            path: None,
            cache: None,
            mcp: None,
            database_url: None,
            development_mode: None,
            multi_site_mode: None,
        }
    }
}

/// Parse a TOML configuration file if it exists
pub fn parse_toml_file<P: AsRef<Path>>(path: P) -> Result<TomlConfig> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(TomlConfig::default());
    }

    let content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read configuration file: {}", path.display()))?;

    let config: TomlConfig = toml::from_str(&content).with_context(|| {
        format!(
            "Failed to parse TOML configuration file: {}",
            path.display()
        )
    })?;

    Ok(config)
}

/// Get standard configuration file paths in order of precedence (lowest to highest)
pub fn get_config_file_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // System-wide configuration
    paths.push(PathBuf::from("/etc/doxyde.conf"));

    // User-specific configuration
    if let Ok(home) = std::env::var("HOME") {
        paths.push(PathBuf::from(home).join(".doxyde.conf"));
    }

    // Current directory configuration (highest precedence)
    paths.push(PathBuf::from("./doxyde.toml"));

    paths
}

/// Merge multiple TOML configurations, with later configs taking precedence
pub fn merge_toml_configs(configs: Vec<TomlConfig>) -> TomlConfig {
    let mut merged = TomlConfig::default();

    for config in configs {
        // Merge server config
        if let Some(server) = config.server {
            let mut merged_server = merged.server.unwrap_or_default();
            if server.host.is_some() {
                merged_server.host = server.host;
            }
            if server.port.is_some() {
                merged_server.port = server.port;
            }
            merged.server = Some(merged_server);
        }

        // Merge session config
        if let Some(session) = config.session {
            let mut merged_session = merged.session.unwrap_or_default();
            if session.session_timeout_minutes.is_some() {
                merged_session.session_timeout_minutes = session.session_timeout_minutes;
            }
            if session.secure_cookies.is_some() {
                merged_session.secure_cookies = session.secure_cookies;
            }
            if session.session_secret.is_some() {
                merged_session.session_secret = session.session_secret;
            }
            merged.session = Some(merged_session);
        }

        // Merge upload config
        if let Some(upload) = config.upload {
            let mut merged_upload = merged.upload.unwrap_or_default();
            if upload.max_upload_size.is_some() {
                merged_upload.max_upload_size = upload.max_upload_size;
            }
            if upload.uploads_dir.is_some() {
                merged_upload.uploads_dir = upload.uploads_dir;
            }
            if upload.upload_allowed_types.is_some() {
                merged_upload.upload_allowed_types = upload.upload_allowed_types;
            }
            merged.upload = Some(merged_upload);
        }

        // Merge rate limit config
        if let Some(rate_limit) = config.rate_limit {
            let mut merged_rate_limit = merged.rate_limit.unwrap_or_default();
            if rate_limit.rate_limit_login_attempts.is_some() {
                merged_rate_limit.rate_limit_login_attempts = rate_limit.rate_limit_login_attempts;
            }
            if rate_limit.rate_limit_api_requests.is_some() {
                merged_rate_limit.rate_limit_api_requests = rate_limit.rate_limit_api_requests;
            }
            merged.rate_limit = Some(merged_rate_limit);
        }

        // Merge security config
        if let Some(security) = config.security {
            let mut merged_security = merged.security.unwrap_or_default();

            if let Some(csrf) = security.csrf {
                let mut merged_csrf = merged_security.csrf.unwrap_or_default();
                if csrf.csrf_enabled.is_some() {
                    merged_csrf.csrf_enabled = csrf.csrf_enabled;
                }
                if csrf.csrf_token_expiry_hours.is_some() {
                    merged_csrf.csrf_token_expiry_hours = csrf.csrf_token_expiry_hours;
                }
                if csrf.csrf_token_length.is_some() {
                    merged_csrf.csrf_token_length = csrf.csrf_token_length;
                }
                if csrf.csrf_header_name.is_some() {
                    merged_csrf.csrf_header_name = csrf.csrf_header_name;
                }
                merged_security.csrf = Some(merged_csrf);
            }

            if let Some(headers) = security.headers {
                let mut merged_headers = merged_security.headers.unwrap_or_default();
                if headers.security_headers_hsts.is_some() {
                    merged_headers.security_headers_hsts = headers.security_headers_hsts;
                }
                if headers.security_headers_csp.is_some() {
                    merged_headers.security_headers_csp = headers.security_headers_csp;
                }
                if headers.security_headers_csp_content.is_some() {
                    merged_headers.security_headers_csp_content =
                        headers.security_headers_csp_content;
                }
                if headers.security_headers_frame_options.is_some() {
                    merged_headers.security_headers_frame_options =
                        headers.security_headers_frame_options;
                }
                if headers.security_headers_content_type_options.is_some() {
                    merged_headers.security_headers_content_type_options =
                        headers.security_headers_content_type_options;
                }
                if headers.security_hsts_content.is_some() {
                    merged_headers.security_hsts_content = headers.security_hsts_content;
                }
                if headers.security_frame_options_content.is_some() {
                    merged_headers.security_frame_options_content =
                        headers.security_frame_options_content;
                }
                if headers.security_referrer_policy.is_some() {
                    merged_headers.security_referrer_policy = headers.security_referrer_policy;
                }
                if headers.security_permissions_policy.is_some() {
                    merged_headers.security_permissions_policy =
                        headers.security_permissions_policy;
                }
                merged_security.headers = Some(merged_headers);
            }

            merged.security = Some(merged_security);
        }

        // Merge path config
        if let Some(path) = config.path {
            let mut merged_path = merged.path.unwrap_or_default();
            if path.sites_dir.is_some() {
                merged_path.sites_dir = path.sites_dir;
            }
            if path.templates_dir.is_some() {
                merged_path.templates_dir = path.templates_dir;
            }
            merged.path = Some(merged_path);
        }

        // Merge cache config
        if let Some(cache) = config.cache {
            let mut merged_cache = merged.cache.unwrap_or_default();
            if cache.cache_static_files_max_age.is_some() {
                merged_cache.cache_static_files_max_age = cache.cache_static_files_max_age;
            }
            merged.cache = Some(merged_cache);
        }

        // Merge MCP config
        if let Some(mcp) = config.mcp {
            let mut merged_mcp = merged.mcp.unwrap_or_default();
            if mcp.mcp_oauth_token_expiry.is_some() {
                merged_mcp.mcp_oauth_token_expiry = mcp.mcp_oauth_token_expiry;
            }
            merged.mcp = Some(merged_mcp);
        }

        // Merge top-level fields
        if config.database_url.is_some() {
            merged.database_url = config.database_url;
        }
        if config.development_mode.is_some() {
            merged.development_mode = config.development_mode;
        }
        if config.multi_site_mode.is_some() {
            merged.multi_site_mode = config.multi_site_mode;
        }
    }

    merged
}

impl Default for TomlServerConfig {
    fn default() -> Self {
        Self {
            host: None,
            port: None,
        }
    }
}

impl Default for TomlSessionConfig {
    fn default() -> Self {
        Self {
            session_timeout_minutes: None,
            secure_cookies: None,
            session_secret: None,
        }
    }
}

impl Default for TomlUploadConfig {
    fn default() -> Self {
        Self {
            max_upload_size: None,
            uploads_dir: None,
            upload_allowed_types: None,
        }
    }
}

impl Default for TomlRateLimitConfig {
    fn default() -> Self {
        Self {
            rate_limit_login_attempts: None,
            rate_limit_api_requests: None,
        }
    }
}

impl Default for TomlSecurityConfig {
    fn default() -> Self {
        Self {
            csrf: None,
            headers: None,
        }
    }
}

impl Default for TomlCsrfConfig {
    fn default() -> Self {
        Self {
            csrf_enabled: None,
            csrf_token_expiry_hours: None,
            csrf_token_length: None,
            csrf_header_name: None,
        }
    }
}

impl Default for TomlHeadersConfig {
    fn default() -> Self {
        Self {
            security_headers_hsts: None,
            security_headers_csp: None,
            security_headers_csp_content: None,
            security_headers_frame_options: None,
            security_headers_content_type_options: None,
            security_hsts_content: None,
            security_frame_options_content: None,
            security_referrer_policy: None,
            security_permissions_policy: None,
        }
    }
}

impl Default for TomlPathConfig {
    fn default() -> Self {
        Self {
            sites_dir: None,
            templates_dir: None,
        }
    }
}

impl Default for TomlCacheConfig {
    fn default() -> Self {
        Self {
            cache_static_files_max_age: None,
        }
    }
}

impl Default for TomlMcpConfig {
    fn default() -> Self {
        Self {
            mcp_oauth_token_expiry: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_empty_toml_file() -> Result<()> {
        let mut file = NamedTempFile::new()?;
        writeln!(file, "")?;

        let config = parse_toml_file(file.path())?;
        assert!(config.database_url.is_none());
        assert!(config.development_mode.is_none());

        Ok(())
    }

    #[test]
    fn test_parse_basic_toml_config() -> Result<()> {
        let mut file = NamedTempFile::new()?;
        writeln!(
            file,
            r#"
database_url = "sqlite:test.db"
development_mode = true
host = "127.0.0.1"
port = 8080
session_timeout_minutes = 720
"#
        )?;

        let config = parse_toml_file(file.path())?;
        assert_eq!(config.database_url, Some("sqlite:test.db".to_string()));
        assert_eq!(config.development_mode, Some(true));

        let server = config.server.unwrap();
        assert_eq!(server.host, Some("127.0.0.1".to_string()));
        assert_eq!(server.port, Some(8080));

        let session = config.session.unwrap();
        assert_eq!(session.session_timeout_minutes, Some(720));

        Ok(())
    }

    #[test]
    fn test_parse_security_headers_config() -> Result<()> {
        let mut file = NamedTempFile::new()?;
        writeln!(
            file,
            r#"
security_headers_csp = true
security_headers_csp_content = "default-src 'self'; script-src 'self' 'unsafe-inline'"
security_headers_hsts = false
security_hsts_content = "max-age=63072000"
"#
        )?;

        let config = parse_toml_file(file.path())?;
        let security = config.security.unwrap();
        let headers = security.headers.unwrap();

        assert_eq!(headers.security_headers_csp, Some(true));
        assert_eq!(
            headers.security_headers_csp_content,
            Some("default-src 'self'; script-src 'self' 'unsafe-inline'".to_string())
        );
        assert_eq!(headers.security_headers_hsts, Some(false));
        assert_eq!(
            headers.security_hsts_content,
            Some("max-age=63072000".to_string())
        );

        Ok(())
    }

    #[test]
    fn test_parse_nonexistent_file() -> Result<()> {
        let config = parse_toml_file("/nonexistent/path.toml")?;
        assert!(config.database_url.is_none());
        assert!(config.development_mode.is_none());

        Ok(())
    }

    #[test]
    fn test_parse_invalid_toml() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "invalid toml [[[").unwrap();

        let result = parse_toml_file(file.path());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to parse TOML"));
    }

    #[test]
    fn test_get_config_file_paths() {
        let paths = get_config_file_paths();
        assert!(!paths.is_empty());
        assert!(paths.iter().any(|p| p.ends_with("/etc/doxyde.conf")));
    }

    #[test]
    fn test_merge_toml_configs() {
        let config1 = TomlConfig {
            database_url: Some("sqlite:db1.db".to_string()),
            server: Some(TomlServerConfig {
                host: Some("127.0.0.1".to_string()),
                port: Some(3000),
            }),
            session: Some(TomlSessionConfig {
                session_timeout_minutes: Some(1440),
                secure_cookies: Some(true),
                session_secret: None,
            }),
            ..Default::default()
        };

        let config2 = TomlConfig {
            database_url: Some("sqlite:db2.db".to_string()),
            server: Some(TomlServerConfig {
                host: None,
                port: Some(8080),
            }),
            session: Some(TomlSessionConfig {
                session_timeout_minutes: None,
                secure_cookies: None,
                session_secret: Some("secret".to_string()),
            }),
            development_mode: Some(true),
            ..Default::default()
        };

        let merged = merge_toml_configs(vec![config1, config2]);

        // Later config should override
        assert_eq!(merged.database_url, Some("sqlite:db2.db".to_string()));
        assert_eq!(merged.development_mode, Some(true));

        // Server config should merge individual fields
        let server = merged.server.unwrap();
        assert_eq!(server.host, Some("127.0.0.1".to_string())); // from config1
        assert_eq!(server.port, Some(8080)); // from config2 (override)

        // Session config should merge individual fields
        let session = merged.session.unwrap();
        assert_eq!(session.session_timeout_minutes, Some(1440)); // from config1
        assert_eq!(session.secure_cookies, Some(true)); // from config1
        assert_eq!(session.session_secret, Some("secret".to_string())); // from config2
    }

    #[test]
    fn test_empty_merge() {
        let merged = merge_toml_configs(vec![]);
        assert!(merged.database_url.is_none());
        assert!(merged.server.is_none());
    }
}
