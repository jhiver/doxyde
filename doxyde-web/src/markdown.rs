use pulldown_cmark::{html, Options, Parser};

/// Convert Markdown text to safe HTML
pub fn markdown_to_html(markdown: &str) -> String {
    // Configure parser options
    let mut options = Options::empty();
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_TASKLISTS);

    // Parse markdown
    let parser = Parser::new_ext(markdown, options);

    // Convert to HTML
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    // Sanitize HTML to prevent XSS
    ammonia::clean(&html_output)
}

/// Create a Tera filter for Markdown conversion
pub fn make_markdown_filter() -> impl tera::Filter {
    |value: &tera::Value, _: &std::collections::HashMap<String, tera::Value>| match value.as_str() {
        Some(text) => Ok(tera::Value::String(markdown_to_html(text))),
        None => Err(tera::Error::msg("markdown filter expects a string")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_to_html_basic() {
        let markdown = "# Hello\n\nThis is a **test**.";
        let html = markdown_to_html(markdown);
        assert!(html.contains("<h1>Hello</h1>"));
        assert!(html.contains("<strong>test</strong>"));
    }

    #[test]
    fn test_markdown_to_html_strikethrough() {
        let markdown = "This is ~~deleted~~ text.";
        let html = markdown_to_html(markdown);
        assert!(html.contains("<del>deleted</del>"));
    }

    #[test]
    fn test_markdown_to_html_table() {
        let markdown = "| Header 1 | Header 2 |\n|----------|----------|\n| Cell 1   | Cell 2   |";
        let html = markdown_to_html(markdown);
        assert!(html.contains("<table>"));
        assert!(html.contains("<th>Header 1</th>"));
    }

    #[test]
    fn test_markdown_to_html_xss_prevention() {
        let markdown = "Hello <script>alert('xss')</script> world!";
        let html = markdown_to_html(markdown);
        assert!(!html.contains("<script>"));
        assert!(!html.contains("alert"));
    }

    #[test]
    fn test_markdown_to_html_links() {
        let markdown = "[Click here](https://example.com)";
        let html = markdown_to_html(markdown);
        assert!(html.contains(r#"<a href="https://example.com""#));
        assert!(html.contains("Click here</a>"));
    }
}
