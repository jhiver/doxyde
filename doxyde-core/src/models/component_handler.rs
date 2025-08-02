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

use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

/// Trait for handling component-specific logic
pub trait ComponentHandler: Send + Sync {
    /// Get the component type this handler manages
    fn component_type(&self) -> &'static str;

    /// Parse content from a form submission into JSON
    fn parse_content(&self, raw_content: &str) -> Result<Value, String>;

    /// Get default content for a new component
    fn default_content(&self) -> Value;

    /// Validate the content before saving
    fn validate_content(&self, _content: &Value) -> Result<(), String> {
        // Default implementation accepts all content
        Ok(())
    }

    /// Get available templates for this component type
    fn available_templates(&self) -> Vec<&'static str> {
        vec!["default"]
    }

    /// Compare content of two components for equality
    /// This should ignore metadata and focus on actual content
    fn content_equals(&self, content1: &Value, content2: &Value) -> bool {
        // Default implementation does full comparison
        content1 == content2
    }
}

/// Registry for component handlers
pub struct ComponentRegistry {
    handlers: HashMap<String, Arc<dyn ComponentHandler>>,
}

impl ComponentRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Register a component handler
    pub fn register<H: ComponentHandler + 'static>(&mut self, handler: H) {
        let component_type = handler.component_type().to_string();
        self.handlers.insert(component_type, Arc::new(handler));
    }

    /// Get a handler for a component type
    pub fn get_handler(&self, component_type: &str) -> Option<Arc<dyn ComponentHandler>> {
        self.handlers.get(component_type).cloned()
    }

    /// Parse content for a component type
    pub fn parse_content(&self, component_type: &str, raw_content: &str) -> Result<Value, String> {
        if let Some(handler) = self.get_handler(component_type) {
            handler.parse_content(raw_content)
        } else {
            // Fallback for unknown types
            Ok(json!({
                "content": raw_content
            }))
        }
    }

    /// Get default content for a component type
    pub fn default_content(&self, component_type: &str) -> Value {
        if let Some(handler) = self.get_handler(component_type) {
            handler.default_content()
        } else {
            // Fallback for unknown types
            json!({
                "content": ""
            })
        }
    }

    /// Compare content of two components
    pub fn content_equals(&self, component_type: &str, content1: &Value, content2: &Value) -> bool {
        if let Some(handler) = self.get_handler(component_type) {
            handler.content_equals(content1, content2)
        } else {
            // Fallback for unknown types
            content1 == content2
        }
    }
}

impl Default for ComponentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Component-specific handlers

/// Handler for text components
pub struct TextComponentHandler;

impl ComponentHandler for TextComponentHandler {
    fn component_type(&self) -> &'static str {
        "text"
    }

    fn parse_content(&self, raw_content: &str) -> Result<Value, String> {
        Ok(json!({
            "text": raw_content
        }))
    }

    fn default_content(&self) -> Value {
        json!({
            "text": ""
        })
    }

    fn available_templates(&self) -> Vec<&'static str> {
        vec![
            "default",
            "with_title",
            "card",
            "highlight",
            "quote",
            "hidden",
        ]
    }

    fn content_equals(&self, content1: &Value, content2: &Value) -> bool {
        // For text components, only compare the text field
        content1.get("text") == content2.get("text")
    }
}

/// Handler for markdown components
pub struct MarkdownComponentHandler;

impl ComponentHandler for MarkdownComponentHandler {
    fn component_type(&self) -> &'static str {
        "markdown"
    }

    fn parse_content(&self, raw_content: &str) -> Result<Value, String> {
        Ok(json!({
            "text": raw_content
        }))
    }

    fn default_content(&self) -> Value {
        json!({
            "text": ""
        })
    }

    fn available_templates(&self) -> Vec<&'static str> {
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

    fn content_equals(&self, content1: &Value, content2: &Value) -> bool {
        // For markdown components, only compare the text field
        content1.get("text") == content2.get("text")
    }
}

/// Handler for HTML components
pub struct HtmlComponentHandler;

impl ComponentHandler for HtmlComponentHandler {
    fn component_type(&self) -> &'static str {
        "html"
    }

    fn parse_content(&self, raw_content: &str) -> Result<Value, String> {
        Ok(json!({
            "html": raw_content
        }))
    }

    fn default_content(&self) -> Value {
        json!({
            "html": ""
        })
    }

    fn available_templates(&self) -> Vec<&'static str> {
        vec!["default"]
    }

    fn content_equals(&self, content1: &Value, content2: &Value) -> bool {
        // For HTML components, only compare the html field
        content1.get("html") == content2.get("html")
    }
}

/// Handler for code components
pub struct CodeComponentHandler;

impl ComponentHandler for CodeComponentHandler {
    fn component_type(&self) -> &'static str {
        "code"
    }

    fn parse_content(&self, raw_content: &str) -> Result<Value, String> {
        Ok(json!({
            "code": raw_content,
            "language": "plaintext"
        }))
    }

    fn default_content(&self) -> Value {
        json!({
            "code": "",
            "language": "plaintext"
        })
    }

    fn available_templates(&self) -> Vec<&'static str> {
        vec!["default", "with_title"]
    }

    fn content_equals(&self, content1: &Value, content2: &Value) -> bool {
        // For code components, compare both code and language fields
        content1.get("code") == content2.get("code")
            && content1.get("language") == content2.get("language")
    }
}

/// Handler for image components
pub struct ImageComponentHandler;

impl ComponentHandler for ImageComponentHandler {
    fn component_type(&self) -> &'static str {
        "image"
    }

    fn parse_content(&self, raw_content: &str) -> Result<Value, String> {
        // Parse as JSON (must be new format)
        serde_json::from_str::<Value>(raw_content)
            .map_err(|e| format!("Failed to parse image content as JSON: {}", e))
    }

    fn default_content(&self) -> Value {
        json!({
            "slug": "",
            "format": "",
            "file_path": "",
            "title": "",
            "description": "",
            "alt_text": ""
        })
    }

    fn available_templates(&self) -> Vec<&'static str> {
        vec![
            "default",
            "figure",
            "hero",
            "gallery",
            "thumbnail",
            "responsive",
            "hidden",
            "inline",
            "float_left",
            "float_right",
        ]
    }

    fn content_equals(&self, content1: &Value, content2: &Value) -> bool {
        // Compare essential fields for image components
        content1.get("slug") == content2.get("slug")
            && content1.get("format") == content2.get("format")
            && content1.get("title") == content2.get("title")
            && content1.get("description") == content2.get("description")
            && content1.get("alt_text") == content2.get("alt_text")
    }
}

/// Handler for blog summary components
pub struct BlogSummaryComponentHandler;

impl ComponentHandler for BlogSummaryComponentHandler {
    fn component_type(&self) -> &'static str {
        "blog_summary"
    }

    fn parse_content(&self, raw_content: &str) -> Result<Value, String> {
        // Parse the JSON configuration
        match serde_json::from_str::<Value>(raw_content) {
            Ok(json_content) => Ok(json_content),
            Err(_) => {
                // Return error message so the web layer can log it
                Err(format!(
                    "Failed to parse blog_summary content as JSON: {}",
                    raw_content
                ))
            }
        }
    }

    fn default_content(&self) -> Value {
        json!({
            "parent_page_id": null,
            "display_title": "Latest Posts",
            "item_count": 5,
            "show_descriptions": true,
            "sort_order": "created_at_desc",
            "template": "cards"
        })
    }

    fn validate_content(&self, content: &Value) -> Result<(), String> {
        // Validate item_count is reasonable
        if let Some(count) = content.get("item_count").and_then(|v| v.as_i64()) {
            if !(1..=50).contains(&count) {
                return Err("Item count must be between 1 and 50".to_string());
            }
        }

        // Validate sort_order
        if let Some(sort_order) = content.get("sort_order").and_then(|v| v.as_str()) {
            match sort_order {
                "created_at_desc" | "created_at_asc" | "title_asc" | "title_desc" => {}
                _ => return Err("Invalid sort order".to_string()),
            }
        }

        Ok(())
    }

    fn available_templates(&self) -> Vec<&'static str> {
        vec![
            "cards",
            "list",
            "definition",
            "compact",
            "timeline",
            "featured",
        ]
    }
}

/// Create a registry with all built-in component handlers
pub fn create_default_registry() -> ComponentRegistry {
    let mut registry = ComponentRegistry::new();

    // Register all built-in handlers
    registry.register(TextComponentHandler);
    registry.register(MarkdownComponentHandler);
    registry.register(HtmlComponentHandler);
    registry.register(CodeComponentHandler);
    registry.register(ImageComponentHandler);
    registry.register(BlogSummaryComponentHandler);

    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_handler() {
        let handler = TextComponentHandler;
        assert_eq!(handler.component_type(), "text");

        let content = handler.parse_content("Hello world").unwrap();
        assert_eq!(content["text"], "Hello world");
    }

    #[test]
    fn test_blog_summary_handler() {
        let handler = BlogSummaryComponentHandler;
        assert_eq!(handler.component_type(), "blog_summary");

        // Test valid JSON
        let json_str = r#"{"parent_page_id": 5, "item_count": 10}"#;
        let content = handler.parse_content(json_str).unwrap();
        assert_eq!(content["parent_page_id"], 5);
        assert_eq!(content["item_count"], 10);

        // Test invalid JSON returns error
        let result = handler.parse_content("invalid json");
        assert!(result.is_err());
    }

    #[test]
    fn test_registry() {
        let registry = create_default_registry();

        // Test getting known handler
        let handler = registry.get_handler("text");
        assert!(handler.is_some());

        // Test parsing with registry
        let content = registry.parse_content("text", "Hello").unwrap();
        assert_eq!(content["text"], "Hello");

        // Test unknown type fallback
        let content = registry.parse_content("unknown", "data").unwrap();
        assert_eq!(content["content"], "data");
    }

    #[test]
    fn test_blog_summary_validation() {
        let handler = BlogSummaryComponentHandler;

        // Valid content
        let valid = json!({
            "item_count": 10,
            "sort_order": "created_at_desc"
        });
        assert!(handler.validate_content(&valid).is_ok());

        // Invalid item count
        let invalid_count = json!({
            "item_count": 100
        });
        assert!(handler.validate_content(&invalid_count).is_err());

        // Invalid sort order
        let invalid_sort = json!({
            "sort_order": "invalid"
        });
        assert!(handler.validate_content(&invalid_sort).is_err());
    }
}
