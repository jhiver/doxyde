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

pub mod auth;
pub mod autoreload_templates;
pub mod component_render;
pub mod config;
pub mod content;
pub mod db;
pub mod draft;
pub mod error;
pub mod handlers;
pub mod logo;
pub mod markdown;
pub mod routes;
pub mod state;
pub mod template_context;
pub mod template_utils;
pub mod templates;
pub mod uploads;

#[cfg(test)]
pub mod test_helpers;


pub use config::Config;
pub use state::AppState;
