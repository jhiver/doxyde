use once_cell::sync::Lazy;
use regex::Regex;

static SLUG_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[^a-zA-Z0-9]+").expect("Failed to compile slug regex"));

/// Generate a URL-friendly slug from a title
pub fn generate_slug_from_title(title: &str) -> String {
    // Convert to lowercase and trim
    let mut slug = title.trim().to_lowercase();

    // Replace non-alphanumeric characters with hyphens
    slug = SLUG_REGEX.replace_all(&slug, "-").to_string();

    // Remove leading/trailing hyphens
    slug = slug.trim_matches('-').to_string();

    // Handle empty results
    if slug.is_empty() {
        slug = "untitled".to_string();
    }

    // Ensure slug doesn't exceed reasonable length (100 chars)
    if slug.len() > 100 {
        slug = slug
            .chars()
            .take(100)
            .collect::<String>()
            .trim_end_matches('-')
            .to_string();
    }

    slug
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_slug_basic() {
        assert_eq!(generate_slug_from_title("Hello World"), "hello-world");
        assert_eq!(generate_slug_from_title("About Us"), "about-us");
        assert_eq!(generate_slug_from_title("Contact"), "contact");
    }

    #[test]
    fn test_generate_slug_special_characters() {
        assert_eq!(generate_slug_from_title("Hello, World!"), "hello-world");
        assert_eq!(generate_slug_from_title("What's New?"), "what-s-new");
        assert_eq!(generate_slug_from_title("Price: $99.99"), "price-99-99");
        assert_eq!(
            generate_slug_from_title("Email@example.com"),
            "email-example-com"
        );
    }

    #[test]
    fn test_generate_slug_whitespace() {
        assert_eq!(generate_slug_from_title("  Hello  World  "), "hello-world");
        assert_eq!(
            generate_slug_from_title("Multiple   Spaces"),
            "multiple-spaces"
        );
        assert_eq!(
            generate_slug_from_title("\tTabs\tand\tSpaces\t"),
            "tabs-and-spaces"
        );
    }

    #[test]
    fn test_generate_slug_edge_cases() {
        assert_eq!(generate_slug_from_title(""), "untitled");
        assert_eq!(generate_slug_from_title("   "), "untitled");
        assert_eq!(generate_slug_from_title("!!!"), "untitled");
        assert_eq!(generate_slug_from_title("---"), "untitled");
    }

    #[test]
    fn test_generate_slug_numbers() {
        assert_eq!(generate_slug_from_title("Article 123"), "article-123");
        assert_eq!(generate_slug_from_title("2024 Review"), "2024-review");
        assert_eq!(generate_slug_from_title("Top 10 Tips"), "top-10-tips");
    }

    #[test]
    fn test_generate_slug_long_title() {
        let long_title = "This is a very long title that exceeds one hundred characters and should be truncated to ensure reasonable URL length for better usability";
        let slug = generate_slug_from_title(long_title);
        assert!(slug.len() <= 100);
        assert!(!slug.ends_with('-'));
    }

    #[test]
    fn test_generate_slug_unicode() {
        // Unicode characters are replaced with hyphens
        assert_eq!(generate_slug_from_title("Café René"), "caf-ren");
        assert_eq!(generate_slug_from_title("Hello 世界"), "hello");
        assert_eq!(generate_slug_from_title("Über uns"), "ber-uns");
    }

    #[test]
    fn test_generate_slug_consecutive_special_chars() {
        assert_eq!(generate_slug_from_title("Hello---World"), "hello-world");
        assert_eq!(generate_slug_from_title("Test___Case"), "test-case");
        assert_eq!(
            generate_slug_from_title("Multiple!!!Exclamations"),
            "multiple-exclamations"
        );
    }
}
