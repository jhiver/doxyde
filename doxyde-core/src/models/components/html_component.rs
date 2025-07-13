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
use crate::models::component_trait::{extract_text, ComponentRenderer};

pub struct HtmlComponent {
    pub id: Option<i64>,
    pub html: String,
    pub title: Option<String>,
}

impl HtmlComponent {
    pub fn from_component(component: &Component) -> Self {
        Self {
            id: component.id,
            html: extract_text(&component.content, "html"),
            title: component.title.clone(),
        }
    }
}

impl ComponentRenderer for HtmlComponent {
    fn render(&self, template: &str) -> String {
        match template {
            "default" => {
                format!(r#"<div class="html-component">{}</div>"#, self.html)
            }
            _ => self.render("default"),
        }
    }

    fn get_available_templates(&self) -> Vec<&'static str> {
        vec!["default"]
    }
}
