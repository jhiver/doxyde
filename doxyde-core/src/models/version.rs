use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PageVersion {
    pub id: Option<i64>,
    pub page_id: i64,
    pub version_number: i32,
    pub created_by: Option<String>,
    pub is_published: bool,
    pub created_at: DateTime<Utc>,
}

impl PageVersion {
    pub fn new(page_id: i64, version_number: i32, created_by: Option<String>) -> Self {
        Self {
            id: None,
            page_id,
            version_number,
            created_by,
            is_published: false,
            created_at: Utc::now(),
        }
    }

    pub fn is_valid(&self) -> Result<(), String> {
        if self.page_id <= 0 {
            return Err("Page ID must be positive".to_string());
        }

        if self.version_number <= 0 {
            return Err("Version number must be positive".to_string());
        }

        if let Some(ref created_by) = self.created_by {
            if created_by.is_empty() {
                return Err("Created by cannot be empty if provided".to_string());
            }
            if created_by.len() > 255 {
                return Err("Created by cannot exceed 255 characters".to_string());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_page_version() {
        let version = PageVersion::new(1, 1, Some("user@example.com".to_string()));

        assert_eq!(version.id, None);
        assert_eq!(version.page_id, 1);
        assert_eq!(version.version_number, 1);
        assert_eq!(version.created_by, Some("user@example.com".to_string()));
        assert_eq!(version.is_published, false);
        assert!(version.created_at <= Utc::now());
    }

    #[test]
    fn test_new_page_version_no_creator() {
        let version = PageVersion::new(5, 3, None);

        assert_eq!(version.id, None);
        assert_eq!(version.page_id, 5);
        assert_eq!(version.version_number, 3);
        assert_eq!(version.created_by, None);
        assert_eq!(version.is_published, false);
        assert!(version.created_at <= Utc::now());
    }

    #[test]
    fn test_new_page_version_different_values() {
        let version1 = PageVersion::new(10, 1, Some("admin".to_string()));
        assert_eq!(version1.page_id, 10);
        assert_eq!(version1.version_number, 1);
        assert_eq!(version1.created_by, Some("admin".to_string()));

        let version2 = PageVersion::new(20, 5, Some("editor".to_string()));
        assert_eq!(version2.page_id, 20);
        assert_eq!(version2.version_number, 5);
        assert_eq!(version2.created_by, Some("editor".to_string()));

        let version3 = PageVersion::new(100, 42, None);
        assert_eq!(version3.page_id, 100);
        assert_eq!(version3.version_number, 42);
        assert_eq!(version3.created_by, None);
    }

    #[test]
    fn test_is_valid_success() {
        let valid_versions = vec![
            PageVersion::new(1, 1, None),
            PageVersion::new(1, 1, Some("user".to_string())),
            PageVersion::new(100, 50, Some("admin@example.com".to_string())),
            PageVersion::new(i64::MAX, i32::MAX, Some("a".repeat(255))),
        ];

        for version in valid_versions {
            assert!(version.is_valid().is_ok());
        }
    }

    #[test]
    fn test_is_valid_invalid_page_id() {
        let version = PageVersion::new(0, 1, None);
        let result = version.is_valid();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Page ID must be positive");

        let version = PageVersion::new(-1, 1, None);
        let result = version.is_valid();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Page ID must be positive");
    }

    #[test]
    fn test_is_valid_invalid_version_number() {
        let version = PageVersion::new(1, 0, None);
        let result = version.is_valid();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Version number must be positive");

        let version = PageVersion::new(1, -1, None);
        let result = version.is_valid();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Version number must be positive");
    }

    #[test]
    fn test_is_valid_invalid_created_by() {
        let version = PageVersion::new(1, 1, Some("".to_string()));
        let result = version.is_valid();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Created by cannot be empty if provided"
        );

        let version = PageVersion::new(1, 1, Some("a".repeat(256)));
        let result = version.is_valid();
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "Created by cannot exceed 255 characters"
        );
    }

    #[test]
    fn test_is_valid_edge_cases() {
        // Test with minimal valid values
        let version = PageVersion::new(1, 1, Some("a".to_string()));
        assert!(version.is_valid().is_ok());

        // Test with maximum length created_by
        let version = PageVersion::new(1, 1, Some("a".repeat(255)));
        assert!(version.is_valid().is_ok());

        // Test with None created_by
        let version = PageVersion::new(1, 1, None);
        assert!(version.is_valid().is_ok());
    }
}
