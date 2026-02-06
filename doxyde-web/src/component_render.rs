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

use doxyde_core::models::component::Component;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tera::{from_value, to_value, Context, Function as TeraFunction, Result as TeraResult, Value};

use crate::{markdown::markdown_to_html, path_security::validate_template_name};

/// Tera function to render a component with its template
pub struct RenderComponentFunction {
    pub templates_dir: String,
    pub site_component_templates: HashMap<String, String>,
}

impl TeraFunction for RenderComponentFunction {
    fn call(&self, args: &HashMap<String, Value>) -> TeraResult<Value> {
        // Get the component from arguments
        let component_value = args
            .get("component")
            .ok_or_else(|| tera::Error::msg("component parameter is required"))?;

        // Deserialize the component
        let component: Component = from_value(component_value.clone())?;

        // Validate component type and template names
        validate_template_name(&component.component_type)
            .map_err(|e| tera::Error::msg(format!("Invalid component type: {}", e)))?;
        validate_template_name(&component.template)
            .map_err(|e| tera::Error::msg(format!("Invalid template name: {}", e)))?;

        // Check site-specific DB overrides first, then filesystem
        let template_key = format!(
            "components/{}/{}.html",
            component.component_type, component.template
        );

        let template_content = if let Some(content) = self.site_component_templates.get(&template_key) {
            content.clone()
        } else {
            // Build the filesystem template path
            let template_path = format!("{}/{}", self.templates_dir, template_key);

            if Path::new(&template_path).exists() {
                fs::read_to_string(&template_path)
                    .map_err(|e| tera::Error::msg(format!("Failed to read template: {}", e)))?
            } else {
                // Try default template
                let default_path = format!(
                    "{}/components/{}/default.html",
                    self.templates_dir, component.component_type
                );

                if Path::new(&default_path).exists() {
                    fs::read_to_string(&default_path).map_err(|e| {
                        tera::Error::msg(format!("Failed to read default template: {}", e))
                    })?
                } else {
                    // Fallback to inline template for backward compatibility
                    return Ok(to_value(render_component_inline(&component)?)?);
                }
            }
        };

        // Create a Tera instance and render the template
        let mut tera = tera::Tera::default();
        tera.add_raw_template("component", &template_content)?;

        // Add filters
        tera.register_filter("markdown", crate::markdown::make_markdown_filter());

        // Create context with component data
        let mut context = Context::new();
        context.insert("component", &component);

        // Render the template
        let html = tera.render("component", &context)?;

        Ok(to_value(html)?)
    }
}

/// Render component inline (fallback for backward compatibility)
fn render_component_inline(component: &Component) -> TeraResult<String> {
    // Use the old renderer for backward compatibility
    let renderer = doxyde_core::models::component_factory::create_renderer(component);
    let mut html = renderer.render(&component.template);

    // Post-process markdown components
    if component.component_type == "markdown" {
        html = process_markdown_placeholders(&html);
    }

    Ok(html)
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
                let class_attr = div_content.find("class=\"").and_then(|class_start| {
                    let class_start = class_start + 7;
                    div_content[class_start..]
                        .find('"')
                        .map(|class_end| &div_content[class_start..class_start + class_end])
                });

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
pub struct GetComponentTemplatesFunction {
    pub templates_dir: String,
}

impl TeraFunction for GetComponentTemplatesFunction {
    fn call(&self, args: &HashMap<String, Value>) -> TeraResult<Value> {
        let component_type = args
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| tera::Error::msg("type parameter is required"))?;

        // Read templates from the filesystem
        let component_dir = format!("{}/components/{}", self.templates_dir, component_type);

        let templates = if let Ok(entries) = fs::read_dir(&component_dir) {
            let mut template_names = Vec::new();

            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_file() {
                        if let Some(file_name) = entry.file_name().to_str() {
                            if file_name.ends_with(".html") {
                                // Remove .html extension
                                let template_name = file_name.trim_end_matches(".html");
                                template_names.push(template_name.to_string());
                            }
                        }
                    }
                }
            }

            // Sort templates with "default" first
            template_names.sort_by(|a, b| {
                if a == "default" {
                    std::cmp::Ordering::Less
                } else if b == "default" {
                    std::cmp::Ordering::Greater
                } else {
                    a.cmp(b)
                }
            });

            template_names
        } else {
            // No template directory found for this component type
            vec!["default".to_string()]
        };

        Ok(to_value(templates)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_render_component_function() {
        let func = RenderComponentFunction {
            templates_dir: "templates".to_string(),
            site_component_templates: HashMap::new(),
        };
        let mut args = HashMap::new();

        let component = Component::new(1, "text".to_string(), 0, json!({"text": "Hello, world!"}));

        args.insert("component".to_string(), to_value(&component).unwrap());

        // This test will use the inline renderer as a fallback
        let result = func.call(&args).unwrap();
        let html = result.as_str().unwrap();

        assert!(html.contains("text-component"));
        assert!(html.contains("Hello, world!"));
    }

    #[test]
    fn test_get_component_templates_function() {
        let func = GetComponentTemplatesFunction {
            templates_dir: "templates".to_string(),
        };
        let mut args = HashMap::new();

        args.insert("type".to_string(), to_value("text").unwrap());

        let result = func.call(&args).unwrap();
        let templates = result.as_array().unwrap();

        // Will use fallback templates in test environment
        assert!(!templates.is_empty());
        assert!(templates.contains(&to_value("default").unwrap()));
    }
}
