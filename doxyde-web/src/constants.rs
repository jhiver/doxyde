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

use crate::configuration::defaults;

// Server defaults
pub fn default_host() -> String {
    defaults::default_host()
}

pub fn default_port() -> u16 {
    defaults::default_port()
}

// Database defaults - keep as-is per requirements
pub const DEFAULT_DATABASE_URL: &str = "sqlite:doxyde.db";

// Directory defaults - these need special handling since they take parameters
pub const DEFAULT_SITES_DIRECTORY: &str = "./sites";
pub const DEFAULT_TEMPLATES_DIR: &str = "templates";

// Upload defaults
pub fn default_uploads_directory() -> String {
    defaults::default_uploads_directory()
}

pub fn default_max_upload_size() -> usize {
    defaults::default_max_upload_size()
}

// Session defaults
pub fn default_session_timeout_minutes() -> i64 {
    defaults::default_session_timeout_minutes()
}

pub fn default_secure_cookies() -> bool {
    defaults::default_secure_cookies()
}

// Development defaults
pub fn default_development_mode() -> bool {
    defaults::default_development_mode()
}

// Configuration file paths
pub const SYSTEM_CONFIG_PATH: &str = "/etc/doxyde.conf";
pub const USER_CONFIG_PATH: &str = ".doxyde.conf";