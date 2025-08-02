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

// Server defaults
pub const DEFAULT_HOST: &str = "0.0.0.0";
pub const DEFAULT_PORT: u16 = 3000;

// Database defaults
pub const DEFAULT_DATABASE_URL: &str = "sqlite:doxyde.db";

// Directory defaults
pub const DEFAULT_SITES_DIRECTORY: &str = "./sites";
pub const DEFAULT_TEMPLATES_DIR: &str = "templates";
pub const DEFAULT_UPLOADS_DIR: &str = ".doxyde/uploads";

// Upload defaults
pub const DEFAULT_MAX_UPLOAD_SIZE: usize = 10_485_760; // 10MB

// Session defaults
pub const DEFAULT_SESSION_TIMEOUT_MINUTES: i64 = 1440; // 24 hours
pub const DEFAULT_SECURE_COOKIES: bool = true;

// Development defaults
pub const DEFAULT_DEVELOPMENT_MODE: bool = false;

// Configuration file paths
pub const SYSTEM_CONFIG_PATH: &str = "/etc/doxyde.conf";
pub const USER_CONFIG_PATH: &str = ".doxyde.conf";