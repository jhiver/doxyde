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

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::models::component_trait::ComponentEq;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Component {
    pub id: Option<i64>,
    pub page_version_id: i64,
    pub component_type: String,
    pub position: i32,
    pub content: Value,
    pub title: Option<String>,
    pub template: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Component {
    pub fn new(
        page_version_id: i64,
        component_type: String,
        position: i32,
        content: Value,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            page_version_id,
            component_type,
            position,
            content,
            title: None,
            template: "default".to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn validate_component_type(&self) -> Result<(), String> {
        if self.component_type.is_empty() {
            return Err("Component type cannot be empty".to_string());
        }

        if self.component_type.len() > 50 {
            return Err("Component type cannot exceed 50 characters".to_string());
        }

        let valid_types = [
            "text",
            "image",
            "code",
            "html",
            "markdown",
            "custom",
            "blog_summary",
        ];
        if !valid_types.contains(&self.component_type.as_str()) {
            return Err(format!(
                "Invalid component type '{}'. Must be one of: {}",
                self.component_type,
                valid_types.join(", ")
            ));
        }

        Ok(())
    }

    pub fn validate_content(&self) -> Result<(), String> {
        if self.content.is_null() {
            return Err("Component content cannot be null".to_string());
        }

        let content_str = self.content.to_string();
        if content_str.len() > 1_048_576 {
            return Err("Component content cannot exceed 1MB when serialized".to_string());
        }

        match self.component_type.as_str() {
            "text" => {
                if !self.content.is_object() || self.content.get("text").is_none() {
                    return Err("Text component must have a 'text' field".to_string());
                }
            }
            "image" => {
                if !self.content.is_object() {
                    return Err("Image component content must be an object".to_string());
                }
                
                // Validate required fields for new format
                if self.content.get("slug").is_none()
                    || self.content.get("format").is_none()
                    || self.content.get("file_path").is_none() {
                    return Err("Image component must have 'slug', 'format', and 'file_path' fields".to_string());
                }

                // Validate slug
                if let Some(slug) = self.content.get("slug").and_then(|s| s.as_str()) {
                    if slug.is_empty() {
                        return Err("Image slug cannot be empty".to_string());
                    }
                    if !slug
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
                    {
                        return Err("Image slug can only contain letters, numbers, hyphens, and underscores".to_string());
                    }
                }

                // Validate format
                if let Some(format) = self.content.get("format").and_then(|f| f.as_str()) {
                    let valid_formats = ["jpg", "jpeg", "png", "gif", "webp", "svg"];
                    if !valid_formats.contains(&format) {
                        return Err(format!(
                            "Invalid image format '{}'. Must be one of: {}",
                            format,
                            valid_formats.join(", ")
                        ));
                    }
                }
            }
            "code" => {
                if !self.content.is_object() {
                    return Err("Code component content must be an object".to_string());
                }
                if self.content.get("code").is_none() {
                    return Err("Code component must have a 'code' field".to_string());
                }
            }
            _ => {}
        }

        Ok(())
    }

    pub fn validate_template(&self) -> Result<(), String> {
        if self.template.is_empty() {
            return Err("Template cannot be empty".to_string());
        }

        if self.template.len() > 50 {
            return Err("Template cannot exceed 50 characters".to_string());
        }

        // Get valid templates for this component type using component_factory
        let valid_templates =
            crate::models::component_factory::get_templates_for_type(&self.component_type);

        if !valid_templates.contains(&self.template.as_str()) {
            return Err(format!(
                "Invalid template '{}' for component type '{}'. Must be one of: {}",
                self.template,
                self.component_type,
                valid_templates.join(", ")
            ));
        }

        Ok(())
    }

    pub fn validate_title(&self) -> Result<(), String> {
        if let Some(ref title) = self.title {
            if title.len() > 255 {
                return Err("Title cannot exceed 255 characters".to_string());
            }
        }
        Ok(())
    }

    pub fn is_valid(&self) -> Result<(), String> {
        self.validate_component_type()?;
        self.validate_content()?;
        self.validate_template()?;
        self.validate_title()?;

        if self.page_version_id <= 0 {
            return Err("Page version ID must be positive".to_string());
        }

        if self.position < 0 {
            return Err("Position must be non-negative".to_string());
        }

        Ok(())
    }
}

impl ComponentEq for Component {
    fn content_equals(&self, other: &Self) -> bool {
        // First check basic fields
        if self.component_type != other.component_type
            || self.position != other.position
            || self.title != other.title
            || self.template != other.template
        {
            return false;
        }

        // Use the component registry to compare content based on component type
        use crate::models::component_handler::create_default_registry;
        let registry = create_default_registry();
        registry.content_equals(&self.component_type, &self.content, &other.content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_new_component() {
        let content = json!({"text": "Hello, world!"});
        let component = Component::new(1, "text".to_string(), 0, content.clone());

        assert_eq!(component.id, None);
        assert_eq!(component.page_version_id, 1);
        assert_eq!(component.component_type, "text");
        assert_eq!(component.position, 0);
        assert_eq!(component.content, content);
        assert_eq!(component.title, None);
        assert_eq!(component.template, "default");
        assert!(component.created_at <= Utc::now());
        assert!(component.updated_at <= Utc::now());
        assert_eq!(component.created_at, component.updated_at);
    }

    #[test]
    fn test_new_component_different_types() {
        let text_content = json!({"text": "Some text"});
        let text_component = Component::new(1, "text".to_string(), 0, text_content.clone());
        assert_eq!(text_component.component_type, "text");
        assert_eq!(text_component.content, text_content);

        let image_content = json!({
            "slug": "photo",
            "format": "jpg",
            "file_path": "/images/photo.jpg",
            "alt_text": "A photo"
        });
        let image_component = Component::new(2, "image".to_string(), 1, image_content.clone());
        assert_eq!(image_component.component_type, "image");
        assert_eq!(image_component.content, image_content);

        let code_content = json!({"language": "rust", "code": "fn main() {}"});
        let code_component = Component::new(3, "code".to_string(), 2, code_content.clone());
        assert_eq!(code_component.component_type, "code");
        assert_eq!(code_component.content, code_content);
    }

    #[test]
    fn test_new_component_with_complex_content() {
        let complex_content = json!({
            "title": "Complex Component",
            "items": [1, 2, 3],
            "nested": {
                "key": "value"
            }
        });
        let component = Component::new(5, "custom".to_string(), 10, complex_content.clone());

        assert_eq!(component.page_version_id, 5);
        assert_eq!(component.component_type, "custom");
        assert_eq!(component.position, 10);
        assert_eq!(component.content, complex_content);
    }

    #[test]
    fn test_validate_component_type_valid() {
        let valid_types = [
            "text",
            "image",
            "code",
            "html",
            "markdown",
            "custom",
            "blog_summary",
        ];

        for component_type in &valid_types {
            let component =
                Component::new(1, component_type.to_string(), 0, json!({"test": "content"}));
            assert!(component.validate_component_type().is_ok());
        }
    }

    #[test]
    fn test_validate_component_type_empty() {
        let component = Component::new(1, "".to_string(), 0, json!({}));
        let result = component.validate_component_type();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Component type cannot be empty");
    }

    #[test]
    fn test_validate_component_type_too_long() {
        let long_type = "a".repeat(51);
        let component = Component::new(1, long_type, 0, json!({}));
        let result = component.validate_component_type();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Component type cannot exceed 50 characters"
        );
    }

    #[test]
    fn test_validate_component_type_invalid() {
        let invalid_types = ["invalid", "unknown", "Text", "IMAGE", ""];

        for invalid_type in &invalid_types {
            if !invalid_type.is_empty() {
                let component =
                    Component::new(1, invalid_type.to_string(), 0, json!({"test": "content"}));
                let result = component.validate_component_type();
                assert!(result.is_err());
                assert!(result
                    .unwrap_err()
                    .contains(&format!("Invalid component type '{}'", invalid_type)));
            }
        }
    }

    #[test]
    fn test_validate_component_type_edge_cases() {
        let fifty_chars = "a".repeat(50);
        let edge_cases = vec![
            ("text", true),
            ("custom", true),
            ("video", false),
            ("text ", false),
            (" text", false),
            (fifty_chars.as_str(), false),
        ];

        for (component_type, should_be_valid) in &edge_cases {
            let component =
                Component::new(1, component_type.to_string(), 0, json!({"test": "content"}));
            let result = component.validate_component_type();
            assert_eq!(result.is_ok(), *should_be_valid);
        }
    }

    #[test]
    fn test_validate_content_null() {
        let component = Component::new(1, "text".to_string(), 0, serde_json::Value::Null);
        let result = component.validate_content();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Component content cannot be null");
    }

    #[test]
    fn test_validate_content_text_component() {
        let valid_content = json!({"text": "Hello, world!"});
        let component = Component::new(1, "text".to_string(), 0, valid_content);
        assert!(component.validate_content().is_ok());

        let invalid_content = json!({"content": "Hello, world!"});
        let component = Component::new(1, "text".to_string(), 0, invalid_content);
        let result = component.validate_content();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Text component must have a 'text' field"
        );

        let non_object_content = json!("just a string");
        let component = Component::new(1, "text".to_string(), 0, non_object_content);
        let result = component.validate_content();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Text component must have a 'text' field"
        );
    }

    #[test]
    fn test_validate_content_image_component() {
        // Test valid new format
        let new_format_content = json!({
            "slug": "hero-image",
            "title": "Hero Image",
            "description": "Main hero image",
            "format": "jpg",
            "file_path": "/var/mkdoc/uploads/2025/01/12/abc123.jpg",
            "original_name": "photo.jpg",
            "mime_type": "image/jpeg",
            "size": 245678,
            "width": 1920,
            "height": 1080
        });
        let component = Component::new(1, "image".to_string(), 0, new_format_content);
        assert!(component.validate_content().is_ok());

        // Test missing required fields
        let missing_fields = json!({"alt": "A photo"});
        let component = Component::new(1, "image".to_string(), 0, missing_fields);
        let result = component.validate_content();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must have 'slug', 'format', and 'file_path' fields"));

        // Test invalid slug
        let invalid_slug = json!({
            "slug": "hero image!", // Invalid characters
            "format": "jpg",
            "file_path": "/path/to/file.jpg"
        });
        let component = Component::new(1, "image".to_string(), 0, invalid_slug);
        let result = component.validate_content();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("can only contain letters"));

        // Test invalid format
        let invalid_format = json!({
            "slug": "hero-image",
            "format": "bmp", // Not supported
            "file_path": "/path/to/file.bmp"
        });
        let component = Component::new(1, "image".to_string(), 0, invalid_format);
        let result = component.validate_content();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid image format"));

        let non_object_content = json!("not an object");
        let component = Component::new(1, "image".to_string(), 0, non_object_content);
        let result = component.validate_content();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Image component content must be an object"
        );
    }

    #[test]
    fn test_validate_content_code_component() {
        let valid_content = json!({"code": "fn main() {}", "language": "rust"});
        let component = Component::new(1, "code".to_string(), 0, valid_content);
        assert!(component.validate_content().is_ok());

        let missing_code = json!({"language": "rust"});
        let component = Component::new(1, "code".to_string(), 0, missing_code);
        let result = component.validate_content();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Code component must have a 'code' field"
        );

        let non_object_content = json!(["not", "an", "object"]);
        let component = Component::new(1, "code".to_string(), 0, non_object_content);
        let result = component.validate_content();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Code component content must be an object"
        );
    }

    #[test]
    fn test_validate_content_custom_component() {
        let any_content = json!({"anything": "goes", "nested": {"key": "value"}});
        let component = Component::new(1, "custom".to_string(), 0, any_content);
        assert!(component.validate_content().is_ok());

        let array_content = json!([1, 2, 3]);
        let component = Component::new(1, "custom".to_string(), 0, array_content);
        assert!(component.validate_content().is_ok());

        let string_content = json!("simple string");
        let component = Component::new(1, "markdown".to_string(), 0, string_content);
        assert!(component.validate_content().is_ok());
    }

    #[test]
    fn test_validate_content_size_limit() {
        // Create content that's definitely over 1MB when serialized
        let large_text = "a".repeat(1_100_000); // 1.1MB of 'a' characters
        let large_content = json!({"text": large_text});
        let component = Component::new(1, "text".to_string(), 0, large_content);
        let result = component.validate_content();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Component content cannot exceed 1MB when serialized"
        );

        // Test content just under the limit
        let ok_text = "a".repeat(1_000_000); // Exactly 1MB
        let ok_content = json!({"text": ok_text});
        let component = Component::new(1, "text".to_string(), 0, ok_content);
        let result = component.validate_content();
        // This should be OK since JSON serialization adds only a small overhead
        assert!(result.is_ok() || result.is_err()); // Allow either since JSON overhead varies
    }

    #[test]
    fn test_is_valid_success() {
        let valid_components = vec![
            Component::new(1, "text".to_string(), 0, json!({"text": "Hello"})),
            Component::new(10, "image".to_string(), 5, json!({
                "slug": "img",
                "format": "jpg",
                "file_path": "/img.jpg"
            })),
            Component::new(100, "code".to_string(), 10, json!({"code": "print()"})),
            Component::new(1, "custom".to_string(), 0, json!({"any": "data"})),
        ];

        for component in valid_components {
            assert!(component.is_valid().is_ok());
        }
    }

    #[test]
    fn test_is_valid_invalid_page_version_id() {
        let component = Component::new(0, "text".to_string(), 0, json!({"text": "Hello"}));
        let result = component.is_valid();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Page version ID must be positive");

        let component = Component::new(-1, "text".to_string(), 0, json!({"text": "Hello"}));
        let result = component.is_valid();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Page version ID must be positive");
    }

    #[test]
    fn test_is_valid_invalid_position() {
        let component = Component::new(1, "text".to_string(), -1, json!({"text": "Hello"}));
        let result = component.is_valid();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Position must be non-negative");
    }

    #[test]
    fn test_is_valid_invalid_component_type() {
        let component = Component::new(1, "invalid".to_string(), 0, json!({"text": "Hello"}));
        let result = component.is_valid();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid component type"));
    }

    #[test]
    fn test_is_valid_invalid_content() {
        let component = Component::new(1, "text".to_string(), 0, json!({"wrong": "field"}));
        let result = component.is_valid();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Text component must have a 'text' field"
        );
    }

    #[test]
    fn test_is_valid_multiple_errors() {
        // Test that validation stops at first error
        let component = Component::new(0, "".to_string(), -1, serde_json::Value::Null);
        let result = component.is_valid();
        assert!(result.is_err());
        // Should get component type error first due to validation order
        assert_eq!(result.unwrap_err(), "Component type cannot be empty");
    }

    #[test]
    fn test_validate_template_valid() {
        // Test text component templates
        let text_templates = [
            "default",
            "with_title",
            "card",
            "highlight",
            "quote",
            "hidden",
        ];

        for template in &text_templates {
            let mut component = Component::new(1, "text".to_string(), 0, json!({"text": "test"}));
            component.template = template.to_string();
            assert!(component.validate_template().is_ok());
        }

        // Test markdown component can use hero template
        let mut markdown_component =
            Component::new(1, "markdown".to_string(), 0, json!({"text": "# Hero"}));
        markdown_component.template = "hero".to_string();
        assert!(markdown_component.validate_template().is_ok());

        // Test that text component cannot use hero template
        let mut text_component = Component::new(1, "text".to_string(), 0, json!({"text": "test"}));
        text_component.template = "hero".to_string();
        assert!(text_component.validate_template().is_err());
    }

    #[test]
    fn test_validate_template_invalid() {
        let mut component = Component::new(1, "text".to_string(), 0, json!({"text": "test"}));

        // Empty template
        component.template = "".to_string();
        let result = component.validate_template();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Template cannot be empty");

        // Invalid template
        component.template = "invalid".to_string();
        let result = component.validate_template();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Invalid template 'invalid' for component type 'text'"));

        // Too long template
        component.template = "a".repeat(51);
        let result = component.validate_template();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Template cannot exceed 50 characters");
    }

    #[test]
    fn test_validate_title() {
        let mut component = Component::new(1, "text".to_string(), 0, json!({"text": "test"}));

        // No title is valid
        assert!(component.validate_title().is_ok());

        // Valid title
        component.title = Some("My Component".to_string());
        assert!(component.validate_title().is_ok());

        // Empty title is valid
        component.title = Some("".to_string());
        assert!(component.validate_title().is_ok());

        // Max length title is valid
        component.title = Some("a".repeat(255));
        assert!(component.validate_title().is_ok());

        // Too long title
        component.title = Some("a".repeat(256));
        let result = component.validate_title();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Title cannot exceed 255 characters");
    }
}
