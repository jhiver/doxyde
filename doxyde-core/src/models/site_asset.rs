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

const MAX_PATH_LENGTH: usize = 255;
const MAX_CONTENT_SIZE: usize = 10 * 1024 * 1024; // 10MB
const MAX_PATH_DEPTH: usize = 5;

const ALLOWED_MIME_TYPES: &[&str] = &[
    "text/javascript",
    "application/javascript",
    "text/css",
    "font/woff",
    "font/woff2",
    "font/ttf",
    "font/otf",
    "image/svg+xml",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SiteAsset {
    pub id: Option<i64>,
    pub path: String,
    pub content: Vec<u8>,
    pub mime_type: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SiteAssetMeta {
    pub id: i64,
    pub path: String,
    pub mime_type: String,
    pub content_length: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SiteAsset {
    pub fn new(path: String, content: Vec<u8>, mime_type: String) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            path,
            content,
            mime_type,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn validate_path(path: &str) -> Result<(), String> {
        if path.is_empty() {
            return Err("Asset path cannot be empty".to_string());
        }
        if path.len() > MAX_PATH_LENGTH {
            return Err(format!(
                "Asset path cannot exceed {} characters",
                MAX_PATH_LENGTH
            ));
        }
        if path.contains("..") {
            return Err("Asset path cannot contain '..'".to_string());
        }
        if path.starts_with('/') || path.starts_with('\\') {
            return Err("Asset path cannot be absolute".to_string());
        }
        let valid_pattern =
            regex::Regex::new(r"^[a-zA-Z0-9_\-/\.]+$").map_err(|e| e.to_string())?;
        if !valid_pattern.is_match(path) {
            return Err(
                "Asset path may only contain letters, digits, underscores, hyphens, slashes, and dots".to_string(),
            );
        }
        let depth = path.split('/').count();
        if depth > MAX_PATH_DEPTH {
            return Err(format!("Asset path depth cannot exceed {}", MAX_PATH_DEPTH));
        }
        Ok(())
    }

    pub fn validate_content(content: &[u8]) -> Result<(), String> {
        if content.len() > MAX_CONTENT_SIZE {
            return Err(format!(
                "Asset content cannot exceed {} bytes",
                MAX_CONTENT_SIZE
            ));
        }
        Ok(())
    }

    pub fn validate_mime_type(mime_type: &str) -> Result<(), String> {
        if ALLOWED_MIME_TYPES.contains(&mime_type) {
            return Ok(());
        }
        if mime_type.starts_with("font/") {
            return Ok(());
        }
        Err(format!("Mime type '{}' is not allowed", mime_type))
    }

    pub fn validate(&self) -> Result<(), String> {
        Self::validate_path(&self.path)?;
        Self::validate_content(&self.content)?;
        Self::validate_mime_type(&self.mime_type)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_creates_asset() {
        let a = SiteAsset::new(
            "js/custom.js".to_string(),
            b"console.log('hi');".to_vec(),
            "text/javascript".to_string(),
        );
        assert_eq!(a.id, None);
        assert_eq!(a.path, "js/custom.js");
        assert_eq!(a.mime_type, "text/javascript");
    }

    #[test]
    fn test_validate_path_valid() {
        let valid = vec![
            "js/custom.js",
            "fonts/brand.woff2",
            "css/theme.css",
            "icon.svg",
        ];
        for path in valid {
            assert!(
                SiteAsset::validate_path(path).is_ok(),
                "Expected '{}' to be valid",
                path
            );
        }
    }

    #[test]
    fn test_validate_path_empty() {
        assert!(SiteAsset::validate_path("").is_err());
    }

    #[test]
    fn test_validate_path_traversal() {
        assert!(SiteAsset::validate_path("../etc/passwd").is_err());
    }

    #[test]
    fn test_validate_path_absolute() {
        assert!(SiteAsset::validate_path("/etc/passwd").is_err());
    }

    #[test]
    fn test_validate_path_too_deep() {
        assert!(SiteAsset::validate_path("a/b/c/d/e/f.js").is_err());
    }

    #[test]
    fn test_validate_content_too_large() {
        let content = vec![0u8; MAX_CONTENT_SIZE + 1];
        assert!(SiteAsset::validate_content(&content).is_err());
    }

    #[test]
    fn test_validate_mime_type_valid() {
        assert!(SiteAsset::validate_mime_type("text/javascript").is_ok());
        assert!(SiteAsset::validate_mime_type("font/woff2").is_ok());
        assert!(SiteAsset::validate_mime_type("text/css").is_ok());
        assert!(SiteAsset::validate_mime_type("image/svg+xml").is_ok());
    }

    #[test]
    fn test_validate_mime_type_invalid() {
        assert!(SiteAsset::validate_mime_type("text/html").is_err());
        assert!(SiteAsset::validate_mime_type("application/pdf").is_err());
    }

    #[test]
    fn test_validate_full() {
        let a = SiteAsset::new(
            "js/custom.js".to_string(),
            b"console.log('hi');".to_vec(),
            "text/javascript".to_string(),
        );
        assert!(a.validate().is_ok());
    }

    #[test]
    fn test_validate_full_invalid_mime() {
        let a = SiteAsset::new(
            "file.html".to_string(),
            b"<html></html>".to_vec(),
            "text/html".to_string(),
        );
        assert!(a.validate().is_err());
    }

    #[test]
    fn test_asset_meta() {
        let meta = SiteAssetMeta {
            id: 1,
            path: "js/custom.js".to_string(),
            mime_type: "text/javascript".to_string(),
            content_length: 42,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        assert_eq!(meta.id, 1);
        assert_eq!(meta.content_length, 42);
    }
}
