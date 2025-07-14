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
use crate::models::components::{
    CodeComponent, CustomComponent, HtmlComponent, ImageComponent, MarkdownComponent, TextComponent,
};

/// Create a renderer for the given component based on its type
pub fn create_renderer(component: &Component) -> Box<dyn ComponentRenderer> {
    match component.component_type.as_str() {
        "text" => Box::new(TextComponent::from_component(component)),
        "markdown" => Box::new(MarkdownComponent::from_component(component)),
        "html" => Box::new(HtmlComponent::from_component(component)),
        "code" => Box::new(CodeComponent::from_component(component)),
        "image" => Box::new(ImageComponent::from_component(component)),
        _ => Box::new(CustomComponent::from_component(component)),
    }
}

/// Get available templates for a given component type
pub fn get_templates_for_type(component_type: &str) -> Vec<&'static str> {
    match component_type {
        "text" => vec![
            "default",
            "with_title",
            "card",
            "highlight",
            "quote",
            "hidden",
        ],
        "markdown" => vec![
            "default",
            "with_title",
            "card",
            "highlight",
            "quote",
            "hidden",
            "hero",
        ],
        "html" => vec!["default"],
        "code" => vec!["default", "with_title"],
        "image" => vec![
            "default",
            "figure",
            "hero",
            "gallery",
            "thumbnail",
            "responsive",
            "hidden",
        ],
        _ => vec!["default"],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_create_text_renderer() {
        let component = Component::new(1, "text".to_string(), 0, json!({"text": "Hello"}));
        let renderer = create_renderer(&component);
        let html = renderer.render("default");
        assert!(html.contains("text-component"));
        assert!(html.contains("Hello"));
    }

    #[test]
    fn test_create_unknown_renderer() {
        let component = Component::new(1, "unknown".to_string(), 0, json!({"data": "test"}));
        let renderer = create_renderer(&component);
        let html = renderer.render("default");
        assert!(html.contains("custom-component"));
        assert!(html.contains("unknown"));
    }

    #[test]
    fn test_get_templates_for_text() {
        let templates = get_templates_for_type("text");
        assert_eq!(templates.len(), 6);
        assert!(templates.contains(&"default"));
        assert!(templates.contains(&"card"));
    }

    #[test]
    fn test_get_templates_for_unknown() {
        let templates = get_templates_for_type("unknown");
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0], "default");
    }
}
