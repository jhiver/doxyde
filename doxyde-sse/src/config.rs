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

use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_bind_addr")]
    pub bind_addr: String,

    #[serde(default = "default_database_url")]
    pub database_url: String,

    #[serde(default = "default_sse_path")]
    pub sse_path: String,

    #[serde(default = "default_post_path")]
    pub post_path: String,

    #[serde(default = "default_keep_alive_secs")]
    pub keep_alive_secs: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind_addr: default_bind_addr(),
            database_url: default_database_url(),
            sse_path: default_sse_path(),
            post_path: default_post_path(),
            keep_alive_secs: default_keep_alive_secs(),
        }
    }
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let config: Config = Figment::new()
            .merge(Toml::file("doxyde-sse.toml"))
            .merge(Env::prefixed("DOXYDE_SSE_"))
            .extract()?;

        Ok(config)
    }

    pub fn keep_alive_duration(&self) -> Duration {
        Duration::from_secs(self.keep_alive_secs)
    }
}

fn default_bind_addr() -> String {
    "127.0.0.1:3001".to_string()
}

fn default_database_url() -> String {
    "sqlite:doxyde.db".to_string()
}

fn default_sse_path() -> String {
    "/".to_string()
}

fn default_post_path() -> String {
    "/message".to_string()
}

fn default_keep_alive_secs() -> u64 {
    30
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.bind_addr, "127.0.0.1:3001");
        assert_eq!(config.database_url, "sqlite:doxyde.db");
        assert_eq!(config.sse_path, "/");
        assert_eq!(config.post_path, "/message");
        assert_eq!(config.keep_alive_secs, 30);
    }

    #[test]
    fn test_keep_alive_duration() {
        let config = Config::default();
        let duration = config.keep_alive_duration();
        assert_eq!(duration, Duration::from_secs(30));
    }

    #[test]
    fn test_custom_config() {
        let config = Config {
            bind_addr: "0.0.0.0:8080".to_string(),
            database_url: "sqlite:test.db".to_string(),
            sse_path: "/sse".to_string(),
            post_path: "/sse/post".to_string(),
            keep_alive_secs: 60,
        };
        assert_eq!(config.bind_addr, "0.0.0.0:8080");
        assert_eq!(config.keep_alive_duration(), Duration::from_secs(60));
    }

    #[test]
    fn test_from_env_with_defaults() {
        // This test will use defaults when no env vars or config file are present
        // In a real test, we might need to clear env vars first
        let result = Config::from_env();
        // Just check it doesn't panic - actual values depend on environment
        assert!(result.is_ok());
    }
}
