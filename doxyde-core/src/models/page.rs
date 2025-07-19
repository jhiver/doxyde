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
use crate::utils::slug::generate_slug_from_title;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Page {
    pub id: Option<i64>,
    pub site_id: i64,
    pub parent_page_id: Option<i64>,
    pub slug: String,
    pub title: String,
    pub description: Option<String>,
    pub keywords: Option<String>,
    pub template: String,
    pub meta_robots: String,
    pub canonical_url: Option<String>,
    pub og_image_url: Option<String>,
    pub structured_data_type: String,
    pub position: i32,
    pub sort_mode: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Page {
    pub fn new(site_id: i64, slug: String, title: String) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            site_id,
            parent_page_id: None,
            slug,
            title,
            description: None,
            keywords: None,
            template: "default".to_string(),
            meta_robots: "index,follow".to_string(),
            canonical_url: None,
            og_image_url: None,
            structured_data_type: "WebPage".to_string(),
            position: 0,
            sort_mode: "created_at_asc".to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn new_with_parent(site_id: i64, parent_page_id: i64, slug: String, title: String) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            site_id,
            parent_page_id: Some(parent_page_id),
            slug,
            title,
            description: None,
            keywords: None,
            template: "default".to_string(),
            meta_robots: "index,follow".to_string(),
            canonical_url: None,
            og_image_url: None,
            structured_data_type: "WebPage".to_string(),
            position: 0,
            sort_mode: "created_at_asc".to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a new page with auto-generated slug from title
    pub fn new_with_title(site_id: i64, title: String) -> Self {
        let slug = generate_slug_from_title(&title);
        Self::new(site_id, slug, title)
    }

    /// Create a new child page with auto-generated slug from title
    pub fn new_with_parent_and_title(site_id: i64, parent_page_id: i64, title: String) -> Self {
        let slug = generate_slug_from_title(&title);
        Self::new_with_parent(site_id, parent_page_id, slug, title)
    }

    pub fn validate_slug(&self) -> Result<(), String> {
        // Empty slug is allowed only for root pages (pages with no parent)
        if self.slug.is_empty() {
            if self.parent_page_id.is_some() {
                return Err("Slug cannot be empty for non-root pages".to_string());
            }
            // Empty slug is valid for root pages
            return Ok(());
        }

        if self.slug.len() > 255 {
            return Err("Slug cannot exceed 255 characters".to_string());
        }

        // Slug should be URL-friendly
        if self.slug.contains(' ') {
            return Err("Slug cannot contain spaces".to_string());
        }

        // Check for invalid characters (only alphanumeric, hyphens, underscores, dots, and slashes allowed)
        let valid_chars =
            |c: char| c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '/';

        if !self.slug.chars().all(valid_chars) {
            return Err(
                "Slug can only contain letters, numbers, hyphens, underscores, dots, and slashes"
                    .to_string(),
            );
        }

        // Cannot start or end with a slash
        if self.slug.starts_with('/') || self.slug.ends_with('/') {
            return Err("Slug cannot start or end with a slash".to_string());
        }

        // Cannot have consecutive slashes
        if self.slug.contains("//") {
            return Err("Slug cannot contain consecutive slashes".to_string());
        }

        Ok(())
    }

    pub fn validate_title(&self) -> Result<(), String> {
        if self.title.is_empty() {
            return Err("Title cannot be empty".to_string());
        }

        if self.title.len() > 255 {
            return Err("Title cannot exceed 255 characters".to_string());
        }

        // Title should not be just whitespace
        if self.title.trim().is_empty() {
            return Err("Title cannot be only whitespace".to_string());
        }

        Ok(())
    }

    pub fn validate_template(&self) -> Result<(), String> {
        // Since templates are now dynamic (discovered from files),
        // we only validate that a template is specified
        if self.template.trim().is_empty() {
            return Err("Template cannot be empty".to_string());
        }

        Ok(())
    }

    pub fn validate_description(&self) -> Result<(), String> {
        if let Some(ref desc) = self.description {
            if desc.len() > 500 {
                return Err("Description cannot exceed 500 characters".to_string());
            }
        }
        Ok(())
    }

    pub fn validate_keywords(&self) -> Result<(), String> {
        if let Some(ref keywords) = self.keywords {
            if keywords.len() > 255 {
                return Err("Keywords cannot exceed 255 characters".to_string());
            }
        }
        Ok(())
    }

    pub fn validate_meta_robots(&self) -> Result<(), String> {
        // Validate that meta_robots contains valid directives
        const VALID_DIRECTIVES: &[&str] = &[
            "index",
            "noindex",
            "follow",
            "nofollow",
            "noarchive",
            "nosnippet",
            "noimageindex",
        ];

        for directive in self.meta_robots.split(',') {
            let directive = directive.trim();
            if !directive.is_empty() && !VALID_DIRECTIVES.contains(&directive) {
                return Err(format!("Invalid robots directive: {}", directive));
            }
        }
        Ok(())
    }

    pub fn validate_canonical_url(&self) -> Result<(), String> {
        if let Some(ref url) = self.canonical_url {
            if url.len() > 500 {
                return Err("Canonical URL cannot exceed 500 characters".to_string());
            }
            // Basic URL validation - just check it's not empty or whitespace
            if url.trim().is_empty() {
                return Err("Canonical URL cannot be empty".to_string());
            }
        }
        Ok(())
    }

    pub fn validate_og_image_url(&self) -> Result<(), String> {
        if let Some(ref url) = self.og_image_url {
            if url.len() > 500 {
                return Err("OG image URL cannot exceed 500 characters".to_string());
            }
            // Basic URL validation - just check it's not empty or whitespace
            if url.trim().is_empty() {
                return Err("OG image URL cannot be empty".to_string());
            }
        }
        Ok(())
    }

    pub fn validate_structured_data_type(&self) -> Result<(), String> {
        const VALID_TYPES: &[&str] = &[
            "WebPage",
            "Article",
            "BlogPosting",
            "Product",
            "Organization",
            "Person",
            "Event",
            "FAQPage",
        ];

        if !VALID_TYPES.contains(&self.structured_data_type.as_str()) {
            return Err(format!(
                "Invalid structured data type '{}'. Valid types are: {}",
                self.structured_data_type,
                VALID_TYPES.join(", ")
            ));
        }
        Ok(())
    }

    pub fn validate_sort_mode(&self) -> Result<(), String> {
        const VALID_MODES: &[&str] = &[
            "created_at_asc",
            "created_at_desc",
            "title_asc",
            "title_desc",
            "manual",
        ];

        if !VALID_MODES.contains(&self.sort_mode.as_str()) {
            return Err(format!(
                "Invalid sort mode '{}'. Valid modes are: {}",
                self.sort_mode,
                VALID_MODES.join(", ")
            ));
        }
        Ok(())
    }

    pub fn is_valid(&self) -> Result<(), String> {
        self.validate_slug()?;
        self.validate_title()?;
        self.validate_template()?;
        self.validate_description()?;
        self.validate_keywords()?;
        self.validate_meta_robots()?;
        self.validate_canonical_url()?;
        self.validate_og_image_url()?;
        self.validate_structured_data_type()?;
        self.validate_sort_mode()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_new_creates_page_with_correct_fields() {
        let site_id = 42;
        let slug = "about-us".to_string();
        let title = "About Us".to_string();

        let before_creation = Utc::now();
        let page = Page::new(site_id, slug.clone(), title.clone());
        let after_creation = Utc::now();

        // Check basic fields
        assert_eq!(page.id, None);
        assert_eq!(page.site_id, site_id);
        assert_eq!(page.parent_page_id, None);
        assert_eq!(page.slug, slug);
        assert_eq!(page.title, title);
        assert_eq!(page.position, 0);
        assert_eq!(page.description, None);
        assert_eq!(page.keywords, None);
        assert_eq!(page.template, "default");
        assert_eq!(page.meta_robots, "index,follow");
        assert_eq!(page.canonical_url, None);
        assert_eq!(page.og_image_url, None);
        assert_eq!(page.structured_data_type, "WebPage");
        assert_eq!(page.sort_mode, "created_at_asc");

        // Check timestamps are set correctly
        assert!(page.created_at >= before_creation);
        assert!(page.created_at <= after_creation);
        assert!(page.updated_at >= before_creation);
        assert!(page.updated_at <= after_creation);
        assert_eq!(page.created_at, page.updated_at);
    }

    #[test]
    fn test_new_with_parent_creates_page_with_parent() {
        let site_id = 42;
        let parent_page_id = 10;
        let slug = "sub-page".to_string();
        let title = "Sub Page".to_string();

        let page = Page::new_with_parent(site_id, parent_page_id, slug.clone(), title.clone());

        assert_eq!(page.id, None);
        assert_eq!(page.site_id, site_id);
        assert_eq!(page.parent_page_id, Some(parent_page_id));
        assert_eq!(page.slug, slug);
        assert_eq!(page.title, title);
        assert_eq!(page.position, 0);
    }

    #[test]
    fn test_new_with_empty_strings() {
        let page = Page::new(1, String::new(), String::new());

        assert_eq!(page.id, None);
        assert_eq!(page.site_id, 1);
        assert_eq!(page.slug, "");
        assert_eq!(page.title, "");

        // Timestamps should still be set
        let time_diff = Utc::now() - page.created_at;
        assert!(time_diff < Duration::seconds(1));
    }

    #[test]
    fn test_new_with_various_site_ids() {
        let test_cases = vec![0, 1, -1, i64::MAX, i64::MIN];

        for site_id in test_cases {
            let page = Page::new(site_id, "test".to_string(), "Test".to_string());
            assert_eq!(page.site_id, site_id);
        }
    }

    #[test]
    fn test_new_with_unicode_strings() {
        let slug = "プロフィール".to_string();
        let title = "私たちについて".to_string();

        let page = Page::new(1, slug.clone(), title.clone());

        assert_eq!(page.slug, slug);
        assert_eq!(page.title, title);
    }

    #[test]
    fn test_new_with_url_friendly_slugs() {
        let test_cases = vec![
            ("about-us", "About Us"),
            ("products_list", "Products List"),
            ("2024-blog-post", "2024 Blog Post"),
            ("page.html", "Page HTML"),
            ("deeply/nested/page", "Deeply Nested Page"),
        ];

        for (slug, title) in test_cases {
            let page = Page::new(1, slug.to_string(), title.to_string());
            assert_eq!(page.slug, slug);
            assert_eq!(page.title, title);
        }
    }

    #[test]
    fn test_validate_slug_valid_cases() {
        let test_cases = vec![
            "about",
            "about-us",
            "products_list",
            "2024-blog-post",
            "page.html",
            "nested/page",
            "deeply/nested/page",
            "file.txt",
            "123",
            "a",
            "日本語ページ",
        ];

        for slug in test_cases {
            let page = Page::new(1, slug.to_string(), "Test".to_string());
            assert!(
                page.validate_slug().is_ok(),
                "Slug '{}' should be valid",
                slug
            );
        }
    }

    #[test]
    fn test_validate_slug_empty() {
        // Empty slug is valid for root pages (no parent)
        let root_page = Page::new(1, String::new(), "Test".to_string());
        let result = root_page.validate_slug();
        assert!(result.is_ok());
        
        // Empty slug is invalid for child pages
        let child_page = Page::new_with_parent(1, 10, String::new(), "Test".to_string());
        let result = child_page.validate_slug();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Slug cannot be empty for non-root pages");
    }

    #[test]
    fn test_validate_slug_too_long() {
        let slug = "a".repeat(256);
        let page = Page::new(1, slug, "Test".to_string());
        let result = page.validate_slug();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Slug cannot exceed 255 characters");
    }

    #[test]
    fn test_validate_slug_with_spaces() {
        let page = Page::new(1, "about us".to_string(), "Test".to_string());
        let result = page.validate_slug();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Slug cannot contain spaces");
    }

    #[test]
    fn test_validate_slug_with_invalid_chars() {
        let test_cases = vec![
            (
                "about!",
                "Slug can only contain letters, numbers, hyphens, underscores, dots, and slashes",
            ),
            (
                "page@home",
                "Slug can only contain letters, numbers, hyphens, underscores, dots, and slashes",
            ),
            (
                "test#anchor",
                "Slug can only contain letters, numbers, hyphens, underscores, dots, and slashes",
            ),
            (
                "path?query",
                "Slug can only contain letters, numbers, hyphens, underscores, dots, and slashes",
            ),
            (
                "page&more",
                "Slug can only contain letters, numbers, hyphens, underscores, dots, and slashes",
            ),
        ];

        for (slug, expected_error) in test_cases {
            let page = Page::new(1, slug.to_string(), "Test".to_string());
            let result = page.validate_slug();
            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), expected_error);
        }
    }

    #[test]
    fn test_validate_slug_starts_with_slash() {
        let page = Page::new(1, "/about".to_string(), "Test".to_string());
        let result = page.validate_slug();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Slug cannot start or end with a slash");
    }

    #[test]
    fn test_validate_slug_ends_with_slash() {
        let page = Page::new(1, "about/".to_string(), "Test".to_string());
        let result = page.validate_slug();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Slug cannot start or end with a slash");
    }

    #[test]
    fn test_validate_slug_consecutive_slashes() {
        let page = Page::new(1, "about//us".to_string(), "Test".to_string());
        let result = page.validate_slug();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Slug cannot contain consecutive slashes"
        );
    }

    #[test]
    fn test_validate_title_valid_cases() {
        let test_cases = vec![
            "My Page",
            "About Us",
            "日本語のページ",
            "Page with numbers 123",
            "Page with symbols !@#$%",
            "A",
            "Very Long Title That Is Still Under The Maximum Character Limit But Contains Many Words",
        ];

        for title in test_cases {
            let page = Page::new(1, "test".to_string(), title.to_string());
            assert!(
                page.validate_title().is_ok(),
                "Title '{}' should be valid",
                title
            );
        }
    }

    #[test]
    fn test_validate_title_empty() {
        let page = Page::new(1, "test".to_string(), String::new());
        let result = page.validate_title();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Title cannot be empty");
    }

    #[test]
    fn test_validate_title_too_long() {
        let title = "a".repeat(256);
        let page = Page::new(1, "test".to_string(), title);
        let result = page.validate_title();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Title cannot exceed 255 characters");
    }

    #[test]
    fn test_validate_title_only_whitespace() {
        let test_cases = vec![" ", "  ", "\t", "\n", "   \t\n  "];

        for title in test_cases {
            let page = Page::new(1, "test".to_string(), title.to_string());
            let result = page.validate_title();
            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), "Title cannot be only whitespace");
        }
    }

    #[test]
    fn test_validate_title_with_leading_trailing_spaces() {
        let page = Page::new(1, "test".to_string(), "  Valid Title  ".to_string());
        assert!(page.validate_title().is_ok());
    }

    #[test]
    fn test_is_valid_with_valid_page() {
        let page = Page::new(1, "about-us".to_string(), "About Us".to_string());
        assert!(page.is_valid().is_ok());
    }

    #[test]
    fn test_is_valid_with_invalid_slug() {
        let page = Page::new(1, "invalid slug".to_string(), "Valid Title".to_string());
        let result = page.is_valid();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Slug cannot contain spaces");
    }

    #[test]
    fn test_is_valid_with_invalid_title() {
        let page = Page::new(1, "valid-slug".to_string(), "".to_string());
        let result = page.is_valid();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Title cannot be empty");
    }

    #[test]
    fn test_is_valid_with_both_invalid() {
        // Root page with empty slug is valid if title is provided
        let root_page = Page::new(1, "".to_string(), "Valid Title".to_string());
        let result = root_page.is_valid();
        assert!(result.is_ok());
        
        // Root page with empty slug and empty title should fail on title validation
        let page = Page::new(1, "".to_string(), "".to_string());
        let result = page.is_valid();
        assert!(result.is_err());
        // Should fail on title validation since empty slug is valid for root pages
        assert_eq!(result.unwrap_err(), "Title cannot be empty");
    }

    #[test]
    fn test_is_valid_multiple_valid_pages() {
        let test_cases = vec![
            ("about", "About"),
            ("products/list", "Products List"),
            ("2024-blog-post", "2024 Blog Post"),
            ("file.html", "HTML File"),
            ("deeply/nested/page", "Deeply Nested Page"),
        ];

        for (slug, title) in test_cases {
            let page = Page::new(1, slug.to_string(), title.to_string());
            assert!(
                page.is_valid().is_ok(),
                "Page with slug '{}' and title '{}' should be valid",
                slug,
                title
            );
        }
    }

    #[test]
    fn test_validate_template_valid() {
        let valid_templates = vec![
            "default",
            "full_width",
            "landing",
            "blog",
            "custom_template",
            "my_special_template",
        ];

        for template in valid_templates {
            let mut page = Page::new(1, "test".to_string(), "Test".to_string());
            page.template = template.to_string();
            assert!(
                page.validate_template().is_ok(),
                "Template '{}' should be valid",
                template
            );
        }
    }

    #[test]
    fn test_validate_template_invalid() {
        let mut page = Page::new(1, "test".to_string(), "Test".to_string());
        page.template = "".to_string();

        let result = page.validate_template();
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("Template cannot be empty"));

        // Test with whitespace only - should fail as it's trimmed
        page.template = "   ".to_string();
        let result = page.validate_template();
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.contains("Template cannot be empty"));
    }

    #[test]
    fn test_validate_description_valid() {
        let long_desc =
            "A longer description that is still under the 500 character limit. ".repeat(5);
        let test_cases = vec![
            None,
            Some(""),
            Some("A short description"),
            Some(long_desc.trim()),
        ];

        for desc in test_cases {
            let mut page = Page::new(1, "test".to_string(), "Test".to_string());
            page.description = desc.map(|s| s.to_string());
            assert!(page.validate_description().is_ok());
        }
    }

    #[test]
    fn test_validate_description_too_long() {
        let mut page = Page::new(1, "test".to_string(), "Test".to_string());
        page.description = Some("a".repeat(501));

        let result = page.validate_description();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Description cannot exceed 500 characters"
        );
    }

    #[test]
    fn test_validate_keywords_valid() {
        let test_cases = vec![
            None,
            Some(""),
            Some("rust, web, cms"),
            Some("keyword1, keyword2, keyword3, keyword4, keyword5"),
        ];

        for keywords in test_cases {
            let mut page = Page::new(1, "test".to_string(), "Test".to_string());
            page.keywords = keywords.map(|s| s.to_string());
            assert!(page.validate_keywords().is_ok());
        }
    }

    #[test]
    fn test_validate_keywords_too_long() {
        let mut page = Page::new(1, "test".to_string(), "Test".to_string());
        page.keywords = Some("a".repeat(256));

        let result = page.validate_keywords();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Keywords cannot exceed 255 characters");
    }

    #[test]
    fn test_validate_meta_robots_valid() {
        let test_cases = vec![
            "index,follow",
            "noindex,nofollow",
            "index,follow,noarchive",
            "nosnippet,noimageindex",
            "", // Empty is valid
        ];

        for robots in test_cases {
            let mut page = Page::new(1, "test".to_string(), "Test".to_string());
            page.meta_robots = robots.to_string();
            assert!(
                page.validate_meta_robots().is_ok(),
                "Robots '{}' should be valid",
                robots
            );
        }
    }

    #[test]
    fn test_validate_meta_robots_invalid() {
        let test_cases = vec!["index,invalid", "badrobot", "index,follow,badbot"];

        for robots in test_cases {
            let mut page = Page::new(1, "test".to_string(), "Test".to_string());
            page.meta_robots = robots.to_string();
            let result = page.validate_meta_robots();
            assert!(result.is_err(), "Robots '{}' should be invalid", robots);
        }
    }

    #[test]
    fn test_validate_structured_data_type_valid() {
        let test_cases = vec![
            "WebPage",
            "Article",
            "BlogPosting",
            "Product",
            "Organization",
            "Person",
            "Event",
            "FAQPage",
        ];

        for data_type in test_cases {
            let mut page = Page::new(1, "test".to_string(), "Test".to_string());
            page.structured_data_type = data_type.to_string();
            assert!(
                page.validate_structured_data_type().is_ok(),
                "Type '{}' should be valid",
                data_type
            );
        }
    }

    #[test]
    fn test_validate_structured_data_type_invalid() {
        let test_cases = vec!["InvalidType", "webpage", "article", ""];

        for data_type in test_cases {
            let mut page = Page::new(1, "test".to_string(), "Test".to_string());
            page.structured_data_type = data_type.to_string();
            let result = page.validate_structured_data_type();
            assert!(result.is_err(), "Type '{}' should be invalid", data_type);
        }
    }

    #[test]
    fn test_new_page_has_default_metadata() {
        let page = Page::new(1, "test".to_string(), "Test".to_string());

        assert_eq!(page.description, None);
        assert_eq!(page.keywords, None);
        assert_eq!(page.template, "default");
        assert_eq!(page.meta_robots, "index,follow");
        assert_eq!(page.canonical_url, None);
        assert_eq!(page.og_image_url, None);
        assert_eq!(page.structured_data_type, "WebPage");
        assert_eq!(page.sort_mode, "created_at_asc");
    }

    #[test]
    fn test_validate_sort_mode_valid() {
        let test_cases = vec![
            "created_at_asc",
            "created_at_desc",
            "title_asc",
            "title_desc",
            "manual",
        ];

        for mode in test_cases {
            let mut page = Page::new(1, "test".to_string(), "Test".to_string());
            page.sort_mode = mode.to_string();
            assert!(
                page.validate_sort_mode().is_ok(),
                "Sort mode '{}' should be valid",
                mode
            );
        }
    }

    #[test]
    fn test_validate_sort_mode_invalid() {
        let test_cases = vec!["invalid", "created_asc", "manual_sort", ""];

        for mode in test_cases {
            let mut page = Page::new(1, "test".to_string(), "Test".to_string());
            page.sort_mode = mode.to_string();
            let result = page.validate_sort_mode();
            assert!(result.is_err(), "Sort mode '{}' should be invalid", mode);
        }
    }

    #[test]
    fn test_is_valid_with_all_metadata() {
        let mut page = Page::new(1, "test".to_string(), "Test".to_string());
        page.description = Some("A test page description".to_string());
        page.keywords = Some("test, validation, metadata".to_string());
        page.template = "blog".to_string();
        page.meta_robots = "noindex,follow".to_string();
        page.canonical_url = Some("https://example.com/test".to_string());
        page.og_image_url = Some("https://example.com/image.jpg".to_string());
        page.structured_data_type = "Article".to_string();

        assert!(page.is_valid().is_ok());
    }

    #[test]
    fn test_new_with_title_generates_slug() {
        let test_cases = vec![
            ("About Us", "about-us"),
            ("Hello World", "hello-world"),
            ("Contact Page", "contact-page"),
            ("2024 Year in Review", "2024-year-in-review"),
            ("What's New?", "what-s-new"),
            ("Price: $99.99", "price-99-99"),
        ];

        for (title, expected_slug) in test_cases {
            let page = Page::new_with_title(1, title.to_string());
            assert_eq!(page.slug, expected_slug);
            assert_eq!(page.title, title);
            assert_eq!(page.site_id, 1);
            assert_eq!(page.parent_page_id, None);
        }
    }

    #[test]
    fn test_new_with_parent_and_title_generates_slug() {
        let test_cases = vec![
            ("Sub Page", "sub-page"),
            ("Child Page", "child-page"),
            ("Nested Content", "nested-content"),
        ];

        for (title, expected_slug) in test_cases {
            let page = Page::new_with_parent_and_title(1, 10, title.to_string());
            assert_eq!(page.slug, expected_slug);
            assert_eq!(page.title, title);
            assert_eq!(page.site_id, 1);
            assert_eq!(page.parent_page_id, Some(10));
        }
    }

    #[test]
    fn test_new_with_title_handles_edge_cases() {
        // Empty title should generate "untitled" slug
        let page = Page::new_with_title(1, "".to_string());
        assert_eq!(page.slug, "untitled");
        assert_eq!(page.title, "");

        // Only special characters should generate "untitled" slug
        let page = Page::new_with_title(1, "!!!".to_string());
        assert_eq!(page.slug, "untitled");
        assert_eq!(page.title, "!!!");

        // Very long title should be truncated
        let long_title = "This is a very long title that exceeds one hundred characters and should be truncated to ensure reasonable URL length for better usability and SEO";
        let page = Page::new_with_title(1, long_title.to_string());
        assert!(page.slug.len() <= 100);
        assert!(!page.slug.ends_with('-'));
        assert_eq!(page.title, long_title);
    }
}
