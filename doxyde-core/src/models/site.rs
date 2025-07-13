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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Site {
    pub id: Option<i64>,
    pub domain: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Site {
    pub fn new(domain: String, title: String) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            domain,
            title,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn validate_domain(&self) -> Result<(), String> {
        if self.domain.is_empty() {
            return Err("Domain cannot be empty".to_string());
        }

        if self.domain.len() > 255 {
            return Err("Domain cannot exceed 255 characters".to_string());
        }

        // Extract hostname without port
        let hostname = if let Some(colon_pos) = self.domain.find(':') {
            &self.domain[..colon_pos]
        } else {
            &self.domain
        };

        // Allow localhost as special case
        if hostname == "localhost" {
            return Ok(());
        }

        // Basic domain validation - must contain at least one dot
        if !self.domain.contains('.') {
            return Err("Domain must contain at least one dot".to_string());
        }

        // Check for invalid characters
        if self.domain.contains(' ') {
            return Err("Domain cannot contain spaces".to_string());
        }

        // Check for double dots
        if self.domain.contains("..") {
            return Err("Domain cannot contain consecutive dots".to_string());
        }

        // Cannot start or end with a dot
        if self.domain.starts_with('.') || self.domain.ends_with('.') {
            return Err("Domain cannot start or end with a dot".to_string());
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

    pub fn is_valid(&self) -> Result<(), String> {
        self.validate_domain()?;
        self.validate_title()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_new_creates_site_with_correct_fields() {
        let domain = "example.com".to_string();
        let title = "Example Site".to_string();

        let before_creation = Utc::now();
        let site = Site::new(domain.clone(), title.clone());
        let after_creation = Utc::now();

        // Check basic fields
        assert_eq!(site.id, None);
        assert_eq!(site.domain, domain);
        assert_eq!(site.title, title);

        // Check timestamps are set correctly (within a reasonable time window)
        assert!(site.created_at >= before_creation);
        assert!(site.created_at <= after_creation);
        assert!(site.updated_at >= before_creation);
        assert!(site.updated_at <= after_creation);
        assert_eq!(site.created_at, site.updated_at);
    }

    #[test]
    fn test_new_with_empty_strings() {
        let site = Site::new(String::new(), String::new());

        assert_eq!(site.id, None);
        assert_eq!(site.domain, "");
        assert_eq!(site.title, "");

        // Timestamps should still be set
        let time_diff = Utc::now() - site.created_at;
        assert!(time_diff < Duration::seconds(1));
    }

    #[test]
    fn test_new_with_unicode_strings() {
        let domain = "例え.jp".to_string();
        let title = "日本語のサイト".to_string();

        let site = Site::new(domain.clone(), title.clone());

        assert_eq!(site.domain, domain);
        assert_eq!(site.title, title);
    }

    #[test]
    fn test_validate_domain_valid_cases() {
        let test_cases = vec![
            "example.com",
            "sub.example.com",
            "sub.sub.example.com",
            "example.co.uk",
            "例え.jp",
            "test-site.com",
            "my_site.org",
            "123.456.789.012",
            "a.b",
            "localhost",
            "localhost:3000",
            "localhost:8080",
        ];

        for domain in test_cases {
            let site = Site::new(domain.to_string(), "Test".to_string());
            assert!(
                site.validate_domain().is_ok(),
                "Domain '{}' should be valid",
                domain
            );
        }
    }

    #[test]
    fn test_validate_domain_empty() {
        let site = Site::new(String::new(), "Test".to_string());
        let result = site.validate_domain();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Domain cannot be empty");
    }

    #[test]
    fn test_validate_domain_too_long() {
        let domain = "a".repeat(256);
        let site = Site::new(domain, "Test".to_string());
        let result = site.validate_domain();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Domain cannot exceed 255 characters");
    }

    #[test]
    fn test_validate_domain_no_dot() {
        let site = Site::new("mydomain".to_string(), "Test".to_string());
        let result = site.validate_domain();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Domain must contain at least one dot");
    }

    #[test]
    fn test_validate_domain_with_spaces() {
        let site = Site::new("example .com".to_string(), "Test".to_string());
        let result = site.validate_domain();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Domain cannot contain spaces");
    }

    #[test]
    fn test_validate_domain_consecutive_dots() {
        let site = Site::new("example..com".to_string(), "Test".to_string());
        let result = site.validate_domain();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Domain cannot contain consecutive dots"
        );
    }

    #[test]
    fn test_validate_domain_starts_with_dot() {
        let site = Site::new(".example.com".to_string(), "Test".to_string());
        let result = site.validate_domain();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Domain cannot start or end with a dot");
    }

    #[test]
    fn test_validate_domain_ends_with_dot() {
        let site = Site::new("example.com.".to_string(), "Test".to_string());
        let result = site.validate_domain();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Domain cannot start or end with a dot");
    }

    #[test]
    fn test_validate_title_valid_cases() {
        let test_cases = vec![
            "My Website",
            "Example Site",
            "日本語のサイト",
            "Site with numbers 123",
            "Site with symbols !@#$%",
            "A",
            "Very Long Title That Is Still Under The Maximum Character Limit But Contains Many Words",
        ];

        for title in test_cases {
            let site = Site::new("example.com".to_string(), title.to_string());
            assert!(
                site.validate_title().is_ok(),
                "Title '{}' should be valid",
                title
            );
        }
    }

    #[test]
    fn test_validate_title_empty() {
        let site = Site::new("example.com".to_string(), String::new());
        let result = site.validate_title();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Title cannot be empty");
    }

    #[test]
    fn test_validate_title_too_long() {
        let title = "a".repeat(256);
        let site = Site::new("example.com".to_string(), title);
        let result = site.validate_title();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Title cannot exceed 255 characters");
    }

    #[test]
    fn test_validate_title_only_whitespace() {
        let test_cases = vec![" ", "  ", "\t", "\n", "   \t\n  "];

        for title in test_cases {
            let site = Site::new("example.com".to_string(), title.to_string());
            let result = site.validate_title();
            assert!(result.is_err());
            assert_eq!(result.unwrap_err(), "Title cannot be only whitespace");
        }
    }

    #[test]
    fn test_validate_title_with_leading_trailing_spaces() {
        let site = Site::new("example.com".to_string(), "  Valid Title  ".to_string());
        assert!(site.validate_title().is_ok());
    }

    #[test]
    fn test_is_valid_with_valid_site() {
        let site = Site::new("example.com".to_string(), "Example Site".to_string());
        assert!(site.is_valid().is_ok());
    }

    #[test]
    fn test_is_valid_with_invalid_domain() {
        let site = Site::new("invalid domain.com".to_string(), "Valid Title".to_string());
        let result = site.is_valid();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Domain cannot contain spaces");
    }

    #[test]
    fn test_is_valid_with_invalid_title() {
        let site = Site::new("example.com".to_string(), "".to_string());
        let result = site.is_valid();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Title cannot be empty");
    }

    #[test]
    fn test_is_valid_with_both_invalid() {
        let site = Site::new("".to_string(), "".to_string());
        let result = site.is_valid();
        assert!(result.is_err());
        // Should fail on domain validation first
        assert_eq!(result.unwrap_err(), "Domain cannot be empty");
    }

    #[test]
    fn test_is_valid_multiple_valid_sites() {
        let test_cases = vec![
            ("example.com", "Example Site"),
            ("sub.example.com", "Subdomain Site"),
            ("例え.jp", "日本語のサイト"),
            ("test-site.org", "Test Site with Dash"),
            ("my.long.domain.example.com", "Site with Long Domain"),
        ];

        for (domain, title) in test_cases {
            let site = Site::new(domain.to_string(), title.to_string());
            assert!(
                site.is_valid().is_ok(),
                "Site with domain '{}' and title '{}' should be valid",
                domain,
                title
            );
        }
    }
}
