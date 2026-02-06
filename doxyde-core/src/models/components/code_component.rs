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
use crate::models::component_trait::{escape_html, extract_text, ComponentRenderer};

pub struct CodeComponent {
    pub id: Option<i64>,
    pub code: String,
    pub language: String,
    pub title: Option<String>,
}

impl CodeComponent {
    pub fn from_component(component: &Component) -> Self {
        Self {
            id: component.id,
            code: extract_text(&component.content, "code"),
            language: if extract_text(&component.content, "language").is_empty() {
                "plaintext".to_string()
            } else {
                extract_text(&component.content, "language")
            },
            title: component.title.clone(),
        }
    }
}

impl ComponentRenderer for CodeComponent {
    fn render(&self, template: &str) -> String {
        let escaped_code = escape_html(&self.code);

        match template {
            "default" => {
                format!(
                    r#"<div class="code-component">
    <pre><code class="language-{}">{}</code></pre>
</div>"#,
                    escape_html(&self.language),
                    escaped_code
                )
            }
            "with_title" => {
                let mut html = String::from(r#"<div class="code-component with-title">"#);
                if let Some(ref title) = self.title {
                    html.push_str(&format!(
                        r#"
    <h4 class="component-title">{}</h4>"#,
                        escape_html(title)
                    ));
                }
                html.push_str(&format!(
                    r#"
    <pre><code class="language-{}">{}</code></pre>
</div>"#,
                    escape_html(&self.language),
                    escaped_code
                ));
                html
            }
            _ => self.render("default"),
        }
    }

}
