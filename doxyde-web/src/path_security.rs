use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

/// Validates that a path is safe and within allowed directories
pub fn validate_safe_path(path: &str, allowed_base: &Path) -> Result<PathBuf> {
    // Reject empty paths
    if path.is_empty() {
        return Err(anyhow!("Empty path provided"));
    }

    // Reject paths with null bytes
    if path.contains('\0') {
        return Err(anyhow!("Path contains null bytes"));
    }

    // Reject paths with suspicious patterns
    // Check for parent directory traversal
    if path.contains("../") || path.contains("..\\") || path.ends_with("..") {
        return Err(anyhow!("Path contains directory traversal patterns"));
    }

    // Check for current directory markers that could be suspicious
    if path.contains("/./")
        || path.contains("\\.\\")
        || path.contains("/../")
        || path.contains("\\..\\")
    {
        return Err(anyhow!("Path contains directory traversal patterns"));
    }

    let path_buf = PathBuf::from(path);

    // If path exists, canonicalize it
    let canonical = if path_buf.exists() {
        path_buf
            .canonicalize()
            .map_err(|e| anyhow!("Failed to canonicalize path: {}", e))?
    } else {
        // For non-existent paths, manually resolve and check
        let mut normalized = allowed_base.to_path_buf();

        // Get relative path from the base
        if path_buf.is_absolute() {
            // For absolute paths, we still need to validate
            path_buf
        } else {
            // Append relative path components
            for component in path_buf.components() {
                match component {
                    std::path::Component::Normal(c) => normalized.push(c),
                    std::path::Component::ParentDir => {
                        return Err(anyhow!("Parent directory references not allowed"));
                    }
                    std::path::Component::RootDir => {
                        return Err(anyhow!("Root directory references not allowed"));
                    }
                    _ => {}
                }
            }
            normalized
        }
    };

    // Get canonical allowed base
    let canonical_base = allowed_base
        .canonicalize()
        .map_err(|e| anyhow!("Failed to canonicalize base path: {}", e))?;

    // Ensure the path is within the allowed base directory
    if !canonical.starts_with(&canonical_base) {
        return Err(anyhow!(
            "Path traversal attempt detected: path is outside allowed directory"
        ));
    }

    Ok(canonical)
}

/// Validates a template name to ensure it's safe
pub fn validate_template_name(name: &str) -> Result<()> {
    // Reject empty names
    if name.is_empty() {
        return Err(anyhow!("Empty template name"));
    }

    // Only allow alphanumeric, underscore, and hyphen
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        return Err(anyhow!("Template name contains invalid characters"));
    }

    // Reject names that could be directory traversal
    if name.contains("..") || name.starts_with('.') || name.starts_with('/') {
        return Err(anyhow!("Template name contains suspicious patterns"));
    }

    Ok(())
}

/// Sanitizes a filename for safe storage
pub fn sanitize_filename(filename: &str) -> String {
    // Remove any path components
    let name = Path::new(filename)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unnamed");

    // Replace any non-alphanumeric characters (except dot and hyphen) with underscore
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_validate_safe_path_success() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Create a test file
        let test_file = base_path.join("test.txt");
        fs::write(&test_file, "test").unwrap();

        // Valid path should succeed
        let result = validate_safe_path(test_file.to_str().unwrap(), base_path);
        match &result {
            Err(e) => println!("Error: {}", e),
            Ok(p) => println!("Success: {:?}", p),
        }
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_safe_path_traversal() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        // Path traversal attempts should fail
        let traversal_attempts = vec![
            "../etc/passwd",
            "./../../etc/passwd",
            "subdir/../../etc/passwd",
            "subdir/../../../etc/passwd",
        ];

        for attempt in traversal_attempts {
            let full_path = base_path.join(attempt).to_string_lossy().to_string();
            let result = validate_safe_path(&full_path, base_path);
            assert!(result.is_err(), "Path traversal not caught: {}", attempt);
        }
    }

    #[test]
    fn test_validate_safe_path_null_bytes() {
        let temp_dir = TempDir::new().unwrap();
        let base_path = temp_dir.path();

        let path_with_null = "test\0.txt";
        let result = validate_safe_path(path_with_null, base_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_template_name() {
        // Valid names
        assert!(validate_template_name("default").is_ok());
        assert!(validate_template_name("with_underscore").is_ok());
        assert!(validate_template_name("with-hyphen").is_ok());
        assert!(validate_template_name("alphanumeric123").is_ok());

        // Invalid names
        assert!(validate_template_name("").is_err());
        assert!(validate_template_name("../traversal").is_err());
        assert!(validate_template_name(".hidden").is_err());
        assert!(validate_template_name("/absolute").is_err());
        assert!(validate_template_name("with spaces").is_err());
        assert!(validate_template_name("with/slash").is_err());
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("normal.txt"), "normal.txt");
        assert_eq!(sanitize_filename("with spaces.txt"), "with_spaces.txt");
        assert_eq!(sanitize_filename("../../../etc/passwd"), "passwd");
        assert_eq!(sanitize_filename("/etc/passwd"), "passwd");
        assert_eq!(sanitize_filename("file<>:|?.txt"), "file_____.txt");
        assert_eq!(sanitize_filename(""), "unnamed");
    }
}
