# Path Traversal Security Audit - Doxyde Web

## Executive Summary

I've conducted a security audit of the Doxyde web application focusing on path traversal vulnerabilities. While the codebase shows good security practices in many areas, I've identified several potential vulnerabilities and areas for improvement.

## Critical Findings

### 1. **Image Serving Handler - No Path Validation** ⚠️ CRITICAL

**Location**: `/src/handlers/image_serve.rs` (line 98-104)

```rust
async fn serve_image_file(file_path: &str, format: &str) -> Result<Response, StatusCode> {
    let path = PathBuf::from(file_path);
    
    // Ensure the file exists
    if !path.exists() {
        tracing::warn!("Image file not found: {}", file_path);
        return Err(StatusCode::NOT_FOUND);
    }
```

**Vulnerability**: The `file_path` comes from the database (component content) and is used directly without any validation. If an attacker can control the database content, they could potentially read any file on the system.

**Risk**: An attacker could potentially:
- Read sensitive files like `/etc/passwd` or application config files
- Access files outside the uploads directory
- Traverse directories using `../` sequences

**Recommendation**: 
1. Validate that the path is within the uploads directory
2. Use `canonicalize()` to resolve the path and check it's within bounds
3. Reject any paths containing `..` components

### 2. **Component Template Path Construction** ⚠️ HIGH

**Location**: `/src/component_render.rs` (lines 41-48)

```rust
let template_path = format!(
    "{}/components/{}/{}.html",
    self.templates_dir, component.component_type, component.template
);

let template_content = if Path::new(&template_path).exists() {
    fs::read_to_string(&template_path)
```

**Vulnerability**: The `component.template` value comes from user input (via forms or MCP API) and is directly interpolated into a file path without validation.

**Risk**: An attacker could potentially:
- Use `../` sequences in template names to read files outside the templates directory
- Access sensitive configuration files
- Read source code or other protected files

**Recommendation**:
1. Validate template names to only allow alphanumeric characters, hyphens, and underscores
2. Reject any template names containing path separators or `..`
3. Use a whitelist of allowed template names

### 3. **Static File Serving** ✅ SAFE

**Location**: `/src/routes.rs` (line 36)

```rust
.nest_service("/.static", ServeDir::new("static"))
```

**Assessment**: The use of `tower_http::services::ServeDir` is safe as it includes built-in path traversal protection.

### 4. **Upload Directory Creation** ✅ MOSTLY SAFE

**Location**: `/src/uploads.rs` (lines 98-109)

```rust
pub fn create_upload_directory(base_path: &Path, date: DateTime<Utc>) -> Result<PathBuf> {
    let year = date.format("%Y").to_string();
    let month = date.format("%m").to_string();
    let day = date.format("%d").to_string();
    
    let dir_path = base_path.join(year).join(month).join(day);
```

**Assessment**: This is safe as it only uses date components which cannot contain path traversal sequences.

### 5. **Filename Generation** ✅ SAFE

**Location**: `/src/uploads.rs` (lines 112-120)

```rust
pub fn generate_unique_filename(original_name: &str) -> String {
    let uuid = Uuid::new_v4();
    let extension = Path::new(original_name)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("bin");
    
    format!("{}.{}", uuid, extension)
}
```

**Assessment**: Using UUID for filenames is safe and prevents path traversal attacks.

## Additional Security Concerns

### 1. **Missing Input Validation for Component Types**

Component types and templates are accepted from user input without strict validation. This could lead to:
- Directory traversal when looking for template files
- Attempts to load non-existent or unauthorized templates

### 2. **Database Content Trust**

The application trusts file paths stored in the database. If the database is compromised or if there's an SQL injection vulnerability elsewhere, attackers could insert malicious paths.

## Recommendations

### Immediate Actions (Critical)

1. **Implement Path Validation for Image Serving**:
```rust
use std::path::{Path, PathBuf};

async fn serve_image_file(file_path: &str, format: &str, uploads_dir: &str) -> Result<Response, StatusCode> {
    let uploads_base = PathBuf::from(uploads_dir).canonicalize()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let requested_path = PathBuf::from(file_path);
    
    // Reject paths with .. components
    if requested_path.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
        tracing::error!("Path traversal attempt detected: {}", file_path);
        return Err(StatusCode::FORBIDDEN);
    }
    
    // Resolve and validate the path
    let canonical_path = requested_path.canonicalize()
        .map_err(|_| StatusCode::NOT_FOUND)?;
    
    // Ensure the path is within uploads directory
    if !canonical_path.starts_with(&uploads_base) {
        tracing::error!("Access attempt outside uploads directory: {}", file_path);
        return Err(StatusCode::FORBIDDEN);
    }
    
    // Continue with file serving...
}
```

2. **Validate Component Template Names**:
```rust
fn validate_template_name(template: &str) -> Result<(), &'static str> {
    // Only allow alphanumeric, dash, and underscore
    if !template.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err("Invalid template name");
    }
    
    // Reject if contains path separators or parent directory references
    if template.contains('/') || template.contains('\\') || template.contains("..") {
        return Err("Template name cannot contain path separators");
    }
    
    // Length limits
    if template.is_empty() || template.len() > 50 {
        return Err("Template name must be between 1 and 50 characters");
    }
    
    Ok(())
}
```

3. **Add Validation to Component Creation/Updates**:
- Validate component types against a whitelist
- Validate template names before saving
- Sanitize all user inputs that could become file paths

### Medium Priority

1. **Implement Content Security Policy Headers**
2. **Add rate limiting for file access**
3. **Log all file access attempts for monitoring**
4. **Consider moving uploaded files outside the web root**
5. **Implement virus scanning for uploaded files**

### Long Term

1. **Consider using object storage (S3, etc.) instead of local filesystem**
2. **Implement a CDN for serving static assets**
3. **Add file access auditing and monitoring**
4. **Implement principle of least privilege for file access**

## Testing Recommendations

Add security tests for:
```rust
#[cfg(test)]
mod security_tests {
    #[test]
    fn test_path_traversal_image_serve() {
        // Test various path traversal attempts
        let attacks = vec![
            "../../../etc/passwd",
            "..\\..\\..\\windows\\system32\\config\\sam",
            "uploads/../../../etc/passwd",
            "uploads/2024/01/01/../../../../../../etc/passwd",
        ];
        
        for attack in attacks {
            // Should reject or sanitize these paths
        }
    }
    
    #[test]
    fn test_template_name_validation() {
        // Valid templates
        assert!(validate_template_name("default").is_ok());
        assert!(validate_template_name("my-template").is_ok());
        assert!(validate_template_name("template_123").is_ok());
        
        // Invalid templates
        assert!(validate_template_name("../etc/passwd").is_err());
        assert!(validate_template_name("../../templates/admin").is_err());
        assert!(validate_template_name("template/../../secret").is_err());
    }
}
```

## Conclusion

While Doxyde shows good security practices in many areas (using UUIDs for filenames, parameterized SQL queries, etc.), the identified path traversal vulnerabilities in image serving and template handling pose significant security risks. These should be addressed immediately before any production deployment.

The fixes are relatively straightforward to implement and would significantly improve the application's security posture.