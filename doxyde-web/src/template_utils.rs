use std::path::Path;

/// Discovers available page templates by scanning the page_templates subdirectory
pub fn discover_page_templates(template_dir: &Path) -> Vec<String> {
    let page_templates_dir = template_dir.join("page_templates");

    // Try to read the page_templates directory
    if let Ok(entries) = std::fs::read_dir(page_templates_dir) {
        let mut found_templates: Vec<String> = Vec::new();

        for entry in entries.flatten() {
            if let Ok(file_name) = entry.file_name().into_string() {
                // Look for .html files
                if file_name.ends_with(".html") {
                    // Extract template name by removing .html extension
                    if let Some(template_name) = file_name.strip_suffix(".html") {
                        found_templates.push(template_name.to_string());
                    }
                }
            }
        }

        // Sort templates alphabetically
        found_templates.sort();

        // If we found templates, return them; otherwise return just default
        if !found_templates.is_empty() {
            found_templates
        } else {
            vec!["default".to_string()]
        }
    } else {
        // If directory doesn't exist or can't be read, return just default
        vec!["default".to_string()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_discover_page_templates_empty_dir() {
        let temp_dir = TempDir::new().unwrap();
        let templates = discover_page_templates(temp_dir.path());
        assert_eq!(templates, vec!["default"]);
    }

    #[test]
    fn test_discover_page_templates_with_files() {
        let temp_dir = TempDir::new().unwrap();
        let page_templates_dir = temp_dir.path().join("page_templates");
        fs::create_dir(&page_templates_dir).unwrap();

        // Create test template files in page_templates directory
        fs::write(page_templates_dir.join("default.html"), "").unwrap();
        fs::write(page_templates_dir.join("blog.html"), "").unwrap();
        fs::write(page_templates_dir.join("landing.html"), "").unwrap();
        fs::write(page_templates_dir.join("full_width.html"), "").unwrap();

        // Create files in main templates directory that should be ignored
        fs::write(temp_dir.path().join("page_edit.html"), "").unwrap();
        fs::write(temp_dir.path().join("not_a_template.html"), "").unwrap();

        let templates = discover_page_templates(temp_dir.path());

        assert_eq!(templates.len(), 4);
        assert_eq!(templates[0], "blog");
        assert_eq!(templates[1], "default");
        assert_eq!(templates[2], "full_width");
        assert_eq!(templates[3], "landing");
    }

    #[test]
    fn test_discover_page_templates_nonexistent_dir() {
        let templates = discover_page_templates(Path::new("/nonexistent/path"));
        assert_eq!(templates, vec!["default"]);
    }
}
