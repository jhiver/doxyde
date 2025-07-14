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

use doxyde_core::models::{component::Component, component_factory::create_renderer};
use std::collections::HashMap;
use tera::{from_value, to_value, Function as TeraFunction, Result as TeraResult, Value};

use crate::markdown::markdown_to_html;

/// Tera function to render a component with its template
pub struct RenderComponentFunction;

impl TeraFunction for RenderComponentFunction {
    fn call(&self, args: &HashMap<String, Value>) -> TeraResult<Value> {
        // Get the component from arguments
        let component_value = args
            .get("component")
            .ok_or_else(|| tera::Error::msg("component parameter is required"))?;

        // Deserialize the component
        let component: Component = from_value(component_value.clone())?;

        // Create the appropriate renderer
        let renderer = create_renderer(&component);

        // Render the component
        let mut html = renderer.render(&component.template);

        // Post-process markdown components
        if component.component_type == "markdown" {
            // Replace markdown placeholders with actual rendered markdown
            html = process_markdown_placeholders(&html);
        }

        Ok(to_value(html)?)
    }
}

/// Process markdown placeholders in the HTML
fn process_markdown_placeholders(html: &str) -> String {
    // This is a simple implementation - in a real app, you might want to use regex
    let mut result = html.to_string();

    // Find all data-markdown attributes and replace their content
    while let Some(start) = result.find("data-markdown=\"") {
        let attr_start = start + 15; // length of 'data-markdown="'
        if let Some(end) = result[attr_start..].find('"') {
            let markdown_escaped = &result[attr_start..attr_start + end];
            // Unescape the markdown
            let markdown = markdown_escaped
                .replace("&quot;", "\"")
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&amp;", "&")
                .replace("&#39;", "'");

            // Convert to HTML
            let html_content = markdown_to_html(&markdown);

            // Find the entire div element to replace
            // We need to find where the div starts (before data-markdown)
            let div_start = result[..start].rfind("<div").unwrap_or(0);
            
            // Find the corresponding closing div
            if let Some(div_end_offset) = result[attr_start + end..].find("</div>") {
                let div_end = attr_start + end + div_end_offset + 6; // +6 for "</div>"
                
                // Extract the div's classes if any
                let div_content = &result[div_start..start];
                let class_attr = if let Some(class_start) = div_content.find("class=\"") {
                    let class_start = class_start + 7;
                    if let Some(class_end) = div_content[class_start..].find('"') {
                        Some(&div_content[class_start..class_start + class_end])
                    } else {
                        None
                    }
                } else {
                    None
                };
                
                // Build the replacement div without data-markdown attribute
                let replacement = if let Some(classes) = class_attr {
                    format!(r#"<div class="{}">{}</div>"#, classes, html_content)
                } else {
                    format!(r#"<div>{}</div>"#, html_content)
                };
                
                // Replace the entire div
                result.replace_range(div_start..div_end, &replacement);
            } else {
                // If we can't find the closing div, break to avoid infinite loop
                break;
            }
        } else {
            break;
        }
    }

    result
}

/// Tera function to get available templates for a component type
pub struct GetComponentTemplatesFunction;

impl TeraFunction for GetComponentTemplatesFunction {
    fn call(&self, args: &HashMap<String, Value>) -> TeraResult<Value> {
        let component_type = args
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| tera::Error::msg("type parameter is required"))?;

        let templates =
            doxyde_core::models::component_factory::get_templates_for_type(component_type);

        Ok(to_value(templates)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_render_component_function() {
        let func = RenderComponentFunction;
        let mut args = HashMap::new();

        let component = Component::new(1, "text".to_string(), 0, json!({"text": "Hello, world!"}));

        args.insert("component".to_string(), to_value(&component).unwrap());

        let result = func.call(&args).unwrap();
        let html = result.as_str().unwrap();

        assert!(html.contains("text-component"));
        assert!(html.contains("Hello, world!"));
    }

    #[test]
    fn test_get_component_templates_function() {
        let func = GetComponentTemplatesFunction;
        let mut args = HashMap::new();

        args.insert("type".to_string(), to_value("text").unwrap());

        let result = func.call(&args).unwrap();
        let templates = result.as_array().unwrap();

        assert_eq!(templates.len(), 6);
        assert!(templates.contains(&to_value("default").unwrap()));
        assert!(templates.contains(&to_value("card").unwrap()));
    }
}
