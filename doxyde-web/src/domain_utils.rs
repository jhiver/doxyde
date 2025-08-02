use sha2::{Digest, Sha256};
use std::path::PathBuf;

/// Extracts the base domain from a full domain name.
///
/// This function extracts the base domain by removing subdomains.
/// For example:
/// - "example.com" → "example.com"
/// - "site1.example.com" → "example.com"
/// - "sse.example.com" → "example.com"
/// - "www.example.com" → "example.com"
/// - "example.com:8080" → "example.com"
/// - "site1.example.com:8080" → "example.com"
///
/// Note: This is a simplified implementation that assumes standard TLD structure.
/// For more complex cases (e.g., .co.uk), a proper public suffix list would be needed.
pub fn extract_base_domain(domain: &str) -> String {
    // Remove port if present
    let domain_no_port = domain.split(':').next().unwrap_or(domain);
    
    // Handle email-like domains by taking everything after @
    let domain_no_prefix = if let Some(at_pos) = domain_no_port.rfind('@') {
        &domain_no_port[at_pos + 1..]
    } else {
        domain_no_port
    };
    
    // Split by dots
    let parts: Vec<&str> = domain_no_prefix.split('.').collect();
    
    // If we have at least 2 parts, take the last 2 as the base domain
    // This handles most common cases like example.com, example.org, etc.
    if parts.len() >= 2 {
        let base = format!("{}.{}", parts[parts.len() - 2], parts[parts.len() - 1]);
        base
    } else {
        // For single-part domains (like "localhost"), return as-is
        domain_no_prefix.to_string()
    }
}

/// Resolves a site directory path from a base path and domain name.
///
/// This function takes a domain name and:
/// - Sanitizes it for filesystem usage (replaces . and : with -)
/// - Appends a hash suffix to prevent collisions
/// - Returns the full path to the site directory
///
/// # Examples
///
/// ```
/// use std::path::PathBuf;
/// use doxyde_web::domain_utils::resolve_site_directory;
///
/// let base = PathBuf::from("/sites");
/// let path = resolve_site_directory(&base, "example.com");
/// // Returns something like: /sites/example-com-a1b2c3d4/
/// ```
pub fn resolve_site_directory(base_path: &PathBuf, domain: &str) -> PathBuf {
    // Extract base domain to ensure subdomains share the same directory
    let base_domain = extract_base_domain(domain);
    
    // Sanitize the base domain for filesystem usage
    let sanitized = base_domain.replace('.', "-").replace(':', "-");

    // Generate hash suffix from the base domain
    let mut hasher = Sha256::new();
    hasher.update(base_domain.as_bytes());
    let hash_result = hasher.finalize();

    // Take first 8 characters of the hex hash
    let hash_suffix = format!("{:x}", hash_result)
        .chars()
        .take(8)
        .collect::<String>();

    // Combine sanitized domain with hash suffix
    let directory_name = format!("{}-{}", sanitized, hash_suffix);

    // Build the full path
    base_path.join(&directory_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_base_domain() {
        // Basic domains
        assert_eq!(extract_base_domain("example.com"), "example.com");
        assert_eq!(extract_base_domain("example.org"), "example.org");
        
        // Subdomains
        assert_eq!(extract_base_domain("www.example.com"), "example.com");
        assert_eq!(extract_base_domain("site1.example.com"), "example.com");
        assert_eq!(extract_base_domain("sse.example.com"), "example.com");
        assert_eq!(extract_base_domain("api.v2.example.com"), "example.com");
        
        // With ports
        assert_eq!(extract_base_domain("example.com:8080"), "example.com");
        assert_eq!(extract_base_domain("site1.example.com:3000"), "example.com");
        
        // Single part (localhost)
        assert_eq!(extract_base_domain("localhost"), "localhost");
        assert_eq!(extract_base_domain("localhost:3000"), "localhost");
    }

    #[test]
    fn test_basic_domain() {
        let base = PathBuf::from("/sites");
        let result = resolve_site_directory(&base, "example.com");

        // Should start with base path
        assert!(result.starts_with(&base));

        // Should contain sanitized domain
        let dir_name = result.file_name().unwrap().to_str().unwrap();
        assert!(dir_name.starts_with("example-com-"));

        // Should have hash suffix (8 chars after the last dash)
        let parts: Vec<&str> = dir_name.split('-').collect();
        assert_eq!(parts.last().unwrap().len(), 8);
    }

    #[test]
    fn test_domain_with_port() {
        let base = PathBuf::from("/sites");
        let result = resolve_site_directory(&base, "example.com:8080");
        let result_no_port = resolve_site_directory(&base, "example.com");

        // Port should be stripped, so both should resolve to same directory
        assert_eq!(result, result_no_port);
        
        let dir_name = result.file_name().unwrap().to_str().unwrap();
        assert!(dir_name.starts_with("example-com-"));
    }

    #[test]
    fn test_subdomain_handling() {
        let base = PathBuf::from("/sites");
        let result1 = resolve_site_directory(&base, "sub.example.com");
        let result2 = resolve_site_directory(&base, "example.com");
        let result3 = resolve_site_directory(&base, "www.example.com");

        // All subdomains should resolve to the same directory
        assert_eq!(result1, result2);
        assert_eq!(result2, result3);
        
        let dir_name = result1.file_name().unwrap().to_str().unwrap();
        assert!(dir_name.starts_with("example-com-"));
    }

    #[test]
    fn test_consistent_hash() {
        let base = PathBuf::from("/sites");
        let result1 = resolve_site_directory(&base, "example.com");
        let result2 = resolve_site_directory(&base, "example.com");

        // Same domain should produce same path
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_different_domains_different_paths() {
        let base = PathBuf::from("/sites");
        let result1 = resolve_site_directory(&base, "example.com");
        let result2 = resolve_site_directory(&base, "example.org");

        // Different domains should produce different paths
        assert_ne!(result1, result2);
    }

    #[test]
    fn test_empty_domain() {
        let base = PathBuf::from("/sites");
        let result = resolve_site_directory(&base, "");

        // Should still produce a valid path with hash
        let dir_name = result.file_name().unwrap().to_str().unwrap();
        assert!(dir_name.starts_with("-")); // empty domain becomes just "-hash"
    }

    #[test]
    fn test_long_domain_name() {
        let base = PathBuf::from("/sites");
        let long_domain = "very.long.subdomain.example.com:8080";
        let result = resolve_site_directory(&base, long_domain);

        // Should resolve to base domain "example.com"
        let dir_name = result.file_name().unwrap().to_str().unwrap();
        assert!(dir_name.starts_with("example-com-"));
    }

    #[test]
    fn test_unicode_domain() {
        let base = PathBuf::from("/sites");
        let unicode_domain = "münchen.example.com";
        let result = resolve_site_directory(&base, unicode_domain);

        // Should resolve to base domain "example.com"
        assert!(result.starts_with(&base));
        let dir_name = result.file_name().unwrap().to_str().unwrap();
        assert!(dir_name.starts_with("example-com-"));
    }

    #[test]
    fn test_special_characters() {
        let base = PathBuf::from("/sites");
        let special_domain = "test@example.com";
        let result = resolve_site_directory(&base, special_domain);

        // @ in subdomain part is ignored, resolves to base domain
        let dir_name = result.file_name().unwrap().to_str().unwrap();
        assert!(dir_name.starts_with("example-com-"));
    }
}
