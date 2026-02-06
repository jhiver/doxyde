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

const MAX_NAME_LENGTH: usize = 100;
const MAX_CSS_SIZE: usize = 1024 * 1024; // 1MB

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SiteStyle {
    pub id: Option<i64>,
    pub name: String,
    pub css_content: String,
    pub is_active: bool,
    pub priority: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SiteStyle {
    pub fn new(name: String, css_content: String) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            name,
            css_content,
            is_active: true,
            priority: 0,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn validate_name(name: &str) -> Result<(), String> {
        if name.is_empty() {
            return Err("Style name cannot be empty".to_string());
        }
        if name.len() > MAX_NAME_LENGTH {
            return Err(format!(
                "Style name cannot exceed {} characters",
                MAX_NAME_LENGTH
            ));
        }
        Ok(())
    }

    pub fn validate_css(css: &str) -> Result<(), String> {
        if css.len() > MAX_CSS_SIZE {
            return Err(format!("CSS content cannot exceed {} bytes", MAX_CSS_SIZE));
        }
        let lower = css.to_lowercase();
        let forbidden = ["expression(", "javascript:", "-moz-binding"];
        for pattern in &forbidden {
            if lower.contains(pattern) {
                return Err(format!(
                    "CSS content contains forbidden pattern: {}",
                    pattern
                ));
            }
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), String> {
        Self::validate_name(&self.name)?;
        Self::validate_css(&self.css_content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_creates_style() {
        let s = SiteStyle::new("main".to_string(), "body { color: red; }".to_string());
        assert_eq!(s.id, None);
        assert_eq!(s.name, "main");
        assert_eq!(s.css_content, "body { color: red; }");
        assert!(s.is_active);
        assert_eq!(s.priority, 0);
    }

    #[test]
    fn test_validate_name_valid() {
        assert!(SiteStyle::validate_name("main").is_ok());
        assert!(SiteStyle::validate_name("theme-colors").is_ok());
        assert!(SiteStyle::validate_name("custom_layout").is_ok());
    }

    #[test]
    fn test_validate_name_empty() {
        let result = SiteStyle::validate_name("");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty"));
    }

    #[test]
    fn test_validate_name_too_long() {
        let name = "a".repeat(MAX_NAME_LENGTH + 1);
        assert!(SiteStyle::validate_name(&name).is_err());
    }

    #[test]
    fn test_validate_css_valid() {
        assert!(SiteStyle::validate_css("body { background: #f0f0f0; }").is_ok());
    }

    #[test]
    fn test_validate_css_too_large() {
        let css = "x".repeat(MAX_CSS_SIZE + 1);
        assert!(SiteStyle::validate_css(&css).is_err());
    }

    #[test]
    fn test_validate_css_forbidden_expression() {
        let result = SiteStyle::validate_css("body { width: expression(100); }");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("expression("));
    }

    #[test]
    fn test_validate_css_forbidden_javascript() {
        let result = SiteStyle::validate_css("body { background: url(javascript:alert(1)); }");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("javascript:"));
    }

    #[test]
    fn test_validate_css_forbidden_moz_binding() {
        let result = SiteStyle::validate_css("body { -moz-binding: url('evil.xml'); }");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("-moz-binding"));
    }

    #[test]
    fn test_validate_full() {
        let s = SiteStyle::new("main".to_string(), "body { color: red; }".to_string());
        assert!(s.validate().is_ok());
    }

    #[test]
    fn test_validate_full_invalid() {
        let s = SiteStyle::new("".to_string(), "body { color: red; }".to_string());
        assert!(s.validate().is_err());
    }
}
