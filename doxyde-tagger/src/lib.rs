//! HTML auto-tagging library for Doxyde CMS
//!
//! This library provides functionality to automatically add HTML markup to existing
//! content by matching text expressions. It's useful for auto-linking, glossary terms,
//! and wiki-style content enhancement.
//!
//! # Examples
//!
//! Basic usage:
//! ```
//! use doxyde_tagger::{HtmlTagger, TagExpression};
//!
//! let mut tagger = HtmlTagger::new();
//! tagger.add_expression(
//!     TagExpression::new("Rust", "a")
//!         .with_attribute("href", "https://rust-lang.org")
//! ).unwrap();
//!
//! let html = "<p>I love Rust programming!</p>";
//! let result = tagger.process(html).unwrap();
//! assert_eq!(result, r#"<p>I love <a href="https://rust-lang.org">Rust</a> programming!</p>"#);
//! ```
//!
//! Using PreserveTagger to avoid double-tagging:
//! ```
//! use doxyde_tagger::{PreserveTagger, TagExpression};
//!
//! let mut tagger = PreserveTagger::new(vec!["a".to_string()]);
//! tagger.add_expression(
//!     TagExpression::new("example", "strong")
//! ).unwrap();
//!
//! let html = r#"Visit <a href="test">example</a> for an example"#;
//! let result = tagger.process(html).unwrap();
//! // "example" inside <a> is preserved, only the second one is tagged
//! assert_eq!(result, r#"Visit <a href="test">example</a> for an <strong>example</strong>"#);
//! ```

pub mod error;
pub mod expression;
pub mod preserve;
pub mod replacer;
pub mod tokenizer;

pub use error::{Result, TaggerError};
pub use expression::TagExpression;
pub use preserve::PreserveTagger;
use replacer::{replace_expression_in_text, segregate_markup_from_text};
use tokenizer::tokenize;

/// Main HTML tagger that processes content and adds markup
#[derive(Debug, Default)]
pub struct HtmlTagger {
    expressions: Vec<TagExpression>,
}

impl HtmlTagger {
    /// Creates a new empty HtmlTagger
    pub fn new() -> Self {
        Self {
            expressions: Vec::new(),
        }
    }

    /// Adds a tag expression to be processed
    pub fn add_expression(&mut self, expr: TagExpression) -> Result<()> {
        expr.validate()?;
        self.expressions.push(expr);
        Ok(())
    }

    /// Creates a new HtmlTagger with the given expressions
    pub fn with_expressions(expressions: Vec<TagExpression>) -> Result<Self> {
        // Validate all expressions
        for expr in &expressions {
            expr.validate()?;
        }

        Ok(Self { expressions })
    }

    /// Processes HTML content, adding tags for matched expressions
    ///
    /// This method:
    /// 1. Tokenizes the HTML
    /// 2. Segregates text from markup
    /// 3. Applies expressions to the text (longest first)
    /// 4. Reconstructs the HTML with placeholders replaced
    pub fn process(&self, html: &str) -> Result<String> {
        if self.expressions.is_empty() {
            return Ok(html.to_string());
        }

        // Tokenize the HTML
        let tokens = tokenize(html)?;

        // Segregate markup from text
        let mut segregated = segregate_markup_from_text(&tokens)?;

        // Sort expressions by length (longest first)
        let mut sorted_expressions = self.expressions.clone();
        sorted_expressions.sort_by(|a, b| b.expression.len().cmp(&a.expression.len()));

        // Apply each expression to the text
        for expr in &sorted_expressions {
            segregated.text =
                replace_expression_in_text(&segregated.text, expr, &mut segregated.tags)?;
        }

        // Replace placeholders with actual tags
        let mut result = segregated.text;
        for (i, tag) in segregated.tags.iter().enumerate() {
            let placeholder = format!("&({})", i + 1);
            result = result.replace(&placeholder, tag);
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_tagger_new() {
        let tagger = HtmlTagger::new();
        assert_eq!(tagger.expressions.len(), 0);
    }

    #[test]
    fn test_add_expression() {
        let mut tagger = HtmlTagger::new();
        let expr = TagExpression::new("test", "strong");

        assert!(tagger.add_expression(expr).is_ok());
        assert_eq!(tagger.expressions.len(), 1);
    }

    #[test]
    fn test_add_invalid_expression() {
        let mut tagger = HtmlTagger::new();
        let expr = TagExpression::new("", "strong");

        assert!(tagger.add_expression(expr).is_err());
        assert_eq!(tagger.expressions.len(), 0);
    }

    #[test]
    fn test_with_expressions() {
        let expressions = vec![
            TagExpression::new("hello", "strong"),
            TagExpression::new("world", "em"),
        ];

        let tagger = HtmlTagger::with_expressions(expressions).unwrap();
        assert_eq!(tagger.expressions.len(), 2);
    }

    #[test]
    fn test_with_invalid_expressions() {
        let expressions = vec![
            TagExpression::new("hello", "strong"),
            TagExpression::new("", "em"), // Invalid
        ];

        assert!(HtmlTagger::with_expressions(expressions).is_err());
    }

    #[test]
    fn test_process_empty_tagger() {
        let tagger = HtmlTagger::new();
        let html = "<p>Hello world!</p>";
        let result = tagger.process(html).unwrap();
        assert_eq!(result, html);
    }

    #[test]
    fn test_process_simple() {
        let mut tagger = HtmlTagger::new();
        tagger
            .add_expression(TagExpression::new("world", "strong"))
            .unwrap();

        let result = tagger.process("<p>Hello world!</p>").unwrap();
        assert_eq!(result, "<p>Hello <strong>world</strong>!</p>");
    }

    #[test]
    fn test_process_with_attributes() {
        let expr = TagExpression::new("example", "a").with_attribute("href", "https://example.com");
        let tagger = HtmlTagger::with_expressions(vec![expr]).unwrap();

        let result = tagger.process("<p>Visit example site</p>").unwrap();
        assert_eq!(
            result,
            r#"<p>Visit <a href="https://example.com">example</a> site</p>"#
        );
    }

    #[test]
    fn test_process_multiple_expressions() {
        let expressions = vec![
            TagExpression::new("oranges", "a")
                .with_attribute("href", "http://www.google.com?q=oranges"),
            TagExpression::new("bananas", "a")
                .with_attribute("href", "http://www.google.com?q=bananas"),
        ];
        let tagger = HtmlTagger::with_expressions(expressions).unwrap();

        let result = tagger.process("I like oranges and bananas").unwrap();
        assert_eq!(
            result,
            r#"I like <a href="http://www.google.com?q=oranges">oranges</a> and <a href="http://www.google.com?q=bananas">bananas</a>"#
        );
    }

    #[test]
    fn test_process_longest_first() {
        let expressions = vec![
            TagExpression::new("Cool World", "a")
                .with_attribute("href", "cw")
                .with_attribute("alt", "foo"),
            TagExpression::new("Hello Cool World", "a").with_attribute("href", "hcw"),
        ];
        let tagger = HtmlTagger::with_expressions(expressions).unwrap();

        let result = tagger.process("Hello Cool World!").unwrap();
        // Longest expression should win
        assert_eq!(result, r#"<a href="hcw">Hello Cool World</a>!"#);
    }

    #[test]
    fn test_process_with_existing_tags() {
        let mut tagger = HtmlTagger::new();
        tagger
            .add_expression(TagExpression::new("world", "em"))
            .unwrap();

        let html = "<p>Hello <strong>brave</strong> world!</p>";
        let result = tagger.process(html).unwrap();
        assert_eq!(
            result,
            "<p>Hello <strong>brave</strong> <em>world</em>!</p>"
        );
    }

    #[test]
    fn test_process_case_insensitive() {
        let mut tagger = HtmlTagger::new();
        tagger
            .add_expression(TagExpression::new("hello", "strong"))
            .unwrap();

        let result = tagger.process("<p>HELLO world</p>").unwrap();
        assert_eq!(result, "<p><strong>HELLO</strong> world</p>");
    }

    #[test]
    fn test_process_complex_html() {
        let expressions = vec![
            TagExpression::new("SGML", "abbr")
                .with_attribute("title", "Standard Generalized Markup Language"),
            TagExpression::new("completely described", "em"),
        ];
        let tagger = HtmlTagger::with_expressions(expressions).unwrap();

        let html = r#"The Extensible Markup Language (XML) is a subset of <strong>SGML</strong>
that is <a href="foo">completely described</a> in this document."#;

        let result = tagger.process(html).unwrap();

        // SGML is already in a tag, but we still tag it
        assert!(
            result.contains(r#"<abbr title="Standard Generalized Markup Language">SGML</abbr>"#)
        );
        // "completely described" is also tagged
        assert!(result.contains("<em>completely described</em>"));
    }
}
