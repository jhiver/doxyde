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

const MAX_NAME_LENGTH: usize = 255;
const MAX_CONTENT_SIZE: usize = 512 * 1024; // 512KB
const MAX_PATH_DEPTH: usize = 5;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SiteTemplate {
    pub id: Option<i64>,
    pub template_name: String,
    pub content: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SiteTemplate {
    pub fn new(template_name: String, content: String) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            template_name,
            content,
            is_active: true,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn validate_name(name: &str) -> Result<(), String> {
        if name.is_empty() {
            return Err("Template name cannot be empty".to_string());
        }
        if name.len() > MAX_NAME_LENGTH {
            return Err(format!(
                "Template name cannot exceed {} characters",
                MAX_NAME_LENGTH
            ));
        }
        if name.contains("..") {
            return Err("Template name cannot contain '..'".to_string());
        }
        if name.starts_with('/') || name.starts_with('\\') {
            return Err("Template name cannot be an absolute path".to_string());
        }
        let valid_pattern =
            regex::Regex::new(r"^[a-zA-Z0-9_\-/\.]+$").map_err(|e| e.to_string())?;
        if !valid_pattern.is_match(name) {
            return Err(
                "Template name may only contain letters, digits, underscores, hyphens, slashes, and dots".to_string(),
            );
        }
        let depth = name.split('/').count();
        if depth > MAX_PATH_DEPTH {
            return Err(format!(
                "Template name path depth cannot exceed {}",
                MAX_PATH_DEPTH
            ));
        }
        Ok(())
    }

    pub fn validate_content(content: &str) -> Result<(), String> {
        if content.len() > MAX_CONTENT_SIZE {
            return Err(format!(
                "Template content cannot exceed {} bytes",
                MAX_CONTENT_SIZE
            ));
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), String> {
        Self::validate_name(&self.template_name)?;
        Self::validate_content(&self.content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_creates_template() {
        let t = SiteTemplate::new("base.html".to_string(), "<html></html>".to_string());
        assert_eq!(t.id, None);
        assert_eq!(t.template_name, "base.html");
        assert_eq!(t.content, "<html></html>");
        assert!(t.is_active);
    }

    #[test]
    fn test_validate_name_valid() {
        let valid = vec![
            "base.html",
            "page_templates/default.html",
            "page_templates/blog.html",
            "components/text/default.html",
            "styles.css",
        ];
        for name in valid {
            assert!(
                SiteTemplate::validate_name(name).is_ok(),
                "Expected '{}' to be valid",
                name
            );
        }
    }

    #[test]
    fn test_validate_name_empty() {
        let result = SiteTemplate::validate_name("");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn test_validate_name_too_long() {
        let name = "a".repeat(MAX_NAME_LENGTH + 1);
        let result = SiteTemplate::validate_name(&name);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("exceed"));
    }

    #[test]
    fn test_validate_name_path_traversal() {
        let result = SiteTemplate::validate_name("../etc/passwd");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains(".."));
    }

    #[test]
    fn test_validate_name_absolute_path() {
        let result = SiteTemplate::validate_name("/etc/passwd");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("absolute"));
    }

    #[test]
    fn test_validate_name_invalid_chars() {
        let invalid = vec!["base html", "base<html>", "base;html"];
        for name in invalid {
            assert!(
                SiteTemplate::validate_name(name).is_err(),
                "Expected '{}' to be invalid",
                name
            );
        }
    }

    #[test]
    fn test_validate_name_too_deep() {
        let name = "a/b/c/d/e/f/g.html";
        let result = SiteTemplate::validate_name(name);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("depth"));
    }

    #[test]
    fn test_validate_content_too_large() {
        let content = "x".repeat(MAX_CONTENT_SIZE + 1);
        let result = SiteTemplate::validate_content(&content);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("exceed"));
    }

    #[test]
    fn test_validate_content_ok() {
        assert!(SiteTemplate::validate_content("<html>hello</html>").is_ok());
    }

    #[test]
    fn test_validate_full() {
        let t = SiteTemplate::new("base.html".to_string(), "<html></html>".to_string());
        assert!(t.validate().is_ok());
    }

    #[test]
    fn test_validate_full_invalid_name() {
        let t = SiteTemplate::new("".to_string(), "<html></html>".to_string());
        assert!(t.validate().is_err());
    }
}
