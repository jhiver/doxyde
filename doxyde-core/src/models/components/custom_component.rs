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

use crate::models::component::Component;
use crate::models::component_trait::ComponentRenderer;

pub struct CustomComponent {
    pub id: Option<i64>,
    pub component_type: String,
    pub content: serde_json::Value,
    pub title: Option<String>,
}

impl CustomComponent {
    pub fn from_component(component: &Component) -> Self {
        Self {
            id: component.id,
            component_type: component.component_type.clone(),
            content: component.content.clone(),
            title: component.title.clone(),
        }
    }
}

impl ComponentRenderer for CustomComponent {
    fn render(&self, _template: &str) -> String {
        format!(
            r#"<div class="custom-component" data-type="{}">{}</div>"#,
            self.component_type,
            serde_json::to_string_pretty(&self.content).unwrap_or_else(|_| "{}".to_string())
        )
    }

    fn get_available_templates(&self) -> Vec<&'static str> {
        vec!["default"]
    }
}
