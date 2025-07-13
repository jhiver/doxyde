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

            // Find the corresponding content to replace
            if let Some(content_start) = result[attr_start + end..].find('>') {
                let content_start_abs = attr_start + end + content_start + 1;
                if let Some(content_end) = result[content_start_abs..].find("</div>") {
                    let content_end_abs = content_start_abs + content_end;
                    // Replace the content
                    result.replace_range(content_start_abs..content_end_abs, &html_content);
                }
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
