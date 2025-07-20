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

use doxyde_core::models::component_handler::{create_default_registry, ComponentRegistry};
use once_cell::sync::Lazy;
use std::sync::Arc;

/// Global component registry instance
pub static COMPONENT_REGISTRY: Lazy<Arc<ComponentRegistry>> =
    Lazy::new(|| Arc::new(create_default_registry()));

/// Get a reference to the global component registry
pub fn get_component_registry() -> Arc<ComponentRegistry> {
    COMPONENT_REGISTRY.clone()
}
