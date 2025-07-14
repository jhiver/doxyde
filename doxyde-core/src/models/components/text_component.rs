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

pub struct TextComponent {
    pub id: Option<i64>,
    pub text: String,
    pub title: Option<String>,
}

impl TextComponent {
    pub fn from_component(component: &Component) -> Self {
        Self {
            id: component.id,
            text: extract_text(&component.content, "text"),
            title: component.title.clone(),
        }
    }
}

impl ComponentRenderer for TextComponent {
    fn render(&self, template: &str) -> String {
        let escaped_text = escape_html(&self.text);

        match template {
            "default" => {
                format!(
                    r#"<div class="text-component">{}</div>"#,
                    escaped_text
                )
            }
            "with_title" => {
                if let Some(ref title) = self.title {
                    format!(
                        r#"<div class="text-component with-title">
    <h3 class="component-title">{}</h3>
    <div class="component-content">{}</div>
</div>"#,
                        escape_html(title),
                        escaped_text
                    )
                } else {
                    format!(
                        r#"<div class="text-component with-title">
    <div class="component-content">{}</div>
</div>"#,
                        escaped_text
                    )
                }
            }
            "card" => {
                let mut html = format!(r#"<div class="text-component card">"#);
                if let Some(ref title) = self.title {
                    html.push_str(&format!(
                        r#"
    <div class="card-header">
        <h3 class="component-title">{}</h3>
    </div>"#,
                        escape_html(title)
                    ));
                }
                html.push_str(&format!(
                    r#"
    <div class="card-body">{}</div>
</div>"#,
                    escaped_text
                ));
                html
            }
            "highlight" => {
                let mut html = format!(r#"<div class="text-component highlight">"#);
                if let Some(ref title) = self.title {
                    html.push_str(&format!(
                        r#"
    <h3 class="component-title">{}</h3>"#,
                        escape_html(title)
                    ));
                }
                html.push_str(&format!(
                    r#"
    <div class="component-content">{}</div>
</div>"#,
                    escaped_text
                ));
                html
            }
            "quote" => {
                let mut html = format!(r#"<blockquote class="text-component quote">"#);
                if let Some(ref title) = self.title {
                    html.push_str(&format!(
                        r#"
    <h4 class="component-title">{}</h4>"#,
                        escape_html(title)
                    ));
                }
                html.push_str(&format!(
                    r#"
    <div class="component-content">{}</div>
</blockquote>"#,
                    escaped_text
                ));
                html
            }
            "hero" => {
                let mut html = format!(r#"<div class="text-component hero">"#);
                if let Some(ref title) = self.title {
                    html.push_str(&format!(
                        r#"
    <h1 class="hero-title">{}</h1>"#,
                        escape_html(title)
                    ));
                }
                html.push_str(&format!(
                    r#"
    <div class="hero-content">{}</div>
</div>"#,
                    escaped_text
                ));
                html
            }
            "hidden" => String::new(),
            _ => self.render("default"),
        }
    }

    fn get_available_templates(&self) -> Vec<&'static str> {
        vec![
            "default",
            "with_title",
            "card",
            "highlight",
            "quote",
            "hero",
            "hidden",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_text_component_from_component() {
        let component = Component::new(1, "text".to_string(), 0, json!({"text": "Hello, world!"}));
        let text_comp = TextComponent::from_component(&component);

        assert_eq!(text_comp.text, "Hello, world!");
        assert_eq!(text_comp.title, None);
    }

    #[test]
    fn test_text_component_render_default() {
        let text_comp = TextComponent {
            id: Some(1),
            text: "Hello, world!".to_string(),
            title: None,
        };

        let html = text_comp.render("default");
        assert_eq!(html, r#"<div class="text-component">Hello, world!</div>"#);
    }

    #[test]
    fn test_text_component_render_with_title() {
        let text_comp = TextComponent {
            id: Some(1),
            text: "Content here".to_string(),
            title: Some("My Title".to_string()),
        };

        let html = text_comp.render("with_title");
        assert!(html.contains("My Title"));
        assert!(html.contains("Content here"));
        assert!(html.contains("text-component with-title"));
    }

    #[test]
    fn test_text_component_escape_html() {
        let text_comp = TextComponent {
            id: Some(1),
            text: "Hello <script>alert('xss')</script>".to_string(),
            title: None,
        };

        let html = text_comp.render("default");
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn test_available_templates() {
        let text_comp = TextComponent {
            id: Some(1),
            text: "Test".to_string(),
            title: None,
        };

        let templates = text_comp.get_available_templates();
        assert_eq!(templates.len(), 7);
        assert!(templates.contains(&"default"));
        assert!(templates.contains(&"with_title"));
        assert!(templates.contains(&"hero"));
    }


    #[test]
    fn test_text_component_render_hero() {
        let text_comp = TextComponent {
            id: Some(1),
            text: "Hero content here".to_string(),
            title: Some("Welcome to Our Site".to_string()),
        };

        let html = text_comp.render("hero");
        assert!(html.contains("text-component hero"));
        assert!(html.contains(r#"<h1 class="hero-title">Welcome to Our Site</h1>"#));
        assert!(html.contains(r#"<div class="hero-content">Hero content here</div>"#));
    }

    #[test]
    fn test_text_component_render_hero_without_title() {
        let text_comp = TextComponent {
            id: Some(1),
            text: "Just hero content".to_string(),
            title: None,
        };

        let html = text_comp.render("hero");
        assert!(html.contains("text-component hero"));
        assert!(!html.contains("hero-title"));
        assert!(html.contains(r#"<div class="hero-content">Just hero content</div>"#));
    }
}
