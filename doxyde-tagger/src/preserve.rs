use crate::{HtmlTagger, Result, TagExpression};
use regex::Regex;

/// A tagger that preserves certain tags from being tagged
///
/// This is useful to prevent double-tagging of elements like links.
/// For example, if you're auto-linking terms but don't want to link
/// text that's already inside an <a> tag.
#[derive(Debug)]
pub struct PreserveTagger {
    /// Tags to preserve (e.g., ["a", "abbr"])
    preserve_tags: Vec<String>,
    /// The underlying tagger
    tagger: HtmlTagger,
}

impl PreserveTagger {
    /// Creates a new PreserveTagger
    pub fn new(preserve_tags: Vec<String>) -> Self {
        Self {
            preserve_tags,
            tagger: HtmlTagger::new(),
        }
    }

    /// Adds an expression to be tagged
    pub fn add_expression(&mut self, expr: TagExpression) -> Result<()> {
        self.tagger.add_expression(expr)
    }

    /// Creates a PreserveTagger with expressions
    pub fn with_expressions(
        preserve_tags: Vec<String>,
        expressions: Vec<TagExpression>,
    ) -> Result<Self> {
        let tagger = HtmlTagger::with_expressions(expressions)?;
        Ok(Self {
            preserve_tags,
            tagger,
        })
    }

    /// Processes HTML while preserving certain tags
    pub fn process(&self, html: &str) -> Result<String> {
        if self.preserve_tags.is_empty() {
            return self.tagger.process(html);
        }

        let mut preserved_sections = Vec::new();
        let mut working_html = html.to_string();

        // Extract and replace preserved tags with placeholders
        for tag in &self.preserve_tags {
            let pattern = format!(
                r"(?s)<{tag}(?:\s[^>]*)?>.*?</{tag}>",
                tag = regex::escape(tag)
            );
            let regex = Regex::new(&pattern)?;

            // Collect all matches first to avoid offset issues
            let matches: Vec<_> = regex
                .find_iter(&working_html)
                .map(|m| (m.start(), m.end(), m.as_str().to_string()))
                .collect();

            // Replace from end to start to maintain positions
            for (start, end, content) in matches.into_iter().rev() {
                let placeholder = format!("__PRESERVE_{}__", preserved_sections.len());
                preserved_sections.push(content);
                working_html.replace_range(start..end, &placeholder);
            }
        }

        // Process the HTML with placeholders
        let processed = self.tagger.process(&working_html)?;

        // Restore preserved sections
        let mut result = processed;
        for (i, section) in preserved_sections.iter().enumerate() {
            let placeholder = format!("__PRESERVE_{}__", i);
            result = result.replace(&placeholder, section);
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preserve_tagger_new() {
        let tagger = PreserveTagger::new(vec!["a".to_string()]);
        assert_eq!(tagger.preserve_tags.len(), 1);
        assert_eq!(tagger.preserve_tags[0], "a");
    }

    #[test]
    fn test_preserve_simple() {
        let mut tagger = PreserveTagger::new(vec!["a".to_string()]);
        tagger
            .add_expression(TagExpression::new("example", "strong"))
            .unwrap();

        let html = r#"Visit <a href="test">example</a> for an example"#;
        let result = tagger.process(html).unwrap();

        // "example" inside <a> should not be tagged
        assert_eq!(
            result,
            r#"Visit <a href="test">example</a> for an <strong>example</strong>"#
        );
    }

    #[test]
    fn test_preserve_multiple_tags() {
        let expressions = vec![TagExpression::new("test", "em")];
        let tagger = PreserveTagger::with_expressions(
            vec!["a".to_string(), "strong".to_string()],
            expressions,
        )
        .unwrap();

        let html = r##"This is a <a href="#">test</a> and <strong>test</strong> and test"##;
        let result = tagger.process(html).unwrap();

        // Only the last "test" should be tagged
        assert_eq!(
            result,
            r##"This is a <a href="#">test</a> and <strong>test</strong> and <em>test</em>"##
        );
    }

    #[test]
    fn test_preserve_nested() {
        let mut tagger = PreserveTagger::new(vec!["div".to_string()]);
        tagger
            .add_expression(TagExpression::new("content", "span"))
            .unwrap();

        let html = r#"<div>Some content here</div> and more content"#;
        let result = tagger.process(html).unwrap();

        // "content" inside div should not be tagged
        assert_eq!(
            result,
            r#"<div>Some content here</div> and more <span>content</span>"#
        );
    }

    #[test]
    fn test_preserve_empty_list() {
        let mut tagger = PreserveTagger::new(vec![]);
        tagger
            .add_expression(TagExpression::new("test", "em"))
            .unwrap();

        let html = r##"<a href="#">test</a> and test"##;
        let result = tagger.process(html).unwrap();

        // Both "test" should be tagged since no preservation
        assert_eq!(
            result,
            r##"<a href="#"><em>test</em></a> and <em>test</em>"##
        );
    }
}
