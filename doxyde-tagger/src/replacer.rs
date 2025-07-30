use crate::expression::TagExpression;
use crate::tokenizer::Token;
use crate::Result;
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashMap;

/// Regex pattern that matches any amount of whitespace, carriage returns,
/// or placeholder tags like &(123)
static IGNORABLE_RE: Lazy<Regex> =
    Lazy::new(|| {
        match Regex::new(r"(?:\s|\r|\n|&\(\d+\))*") {
            Ok(regex) => regex,
            Err(_) => {
                // This is a compile-time constant regex that should never fail
                // If it does fail, the program cannot function correctly
                std::process::abort();
            }
        }
    });

/// Represents the segregated text with placeholders and the original tags
pub struct SegregatedContent {
    /// Text with placeholders like &(1), &(2), etc.
    pub text: String,
    /// Original tag strings in order
    pub tags: Vec<String>,
}

/// Segregates markup from text, replacing tags with placeholders
///
/// Example:
/// Input tokens: ["<span>", "Hello ", "<br />", "World", "</span>"]
/// Output: SegregatedContent {
///     text: "&(1)Hello &(2)World&(3)",
///     tags: ["<span>", "<br />", "</span>"]
/// }
pub fn segregate_markup_from_text(tokens: &[Token]) -> Result<SegregatedContent> {
    let mut text = String::new();
    let mut tags = Vec::new();

    for token in tokens {
        match token {
            Token::Text(s) => {
                text.push_str(s);
            }
            Token::OpenTag { raw, .. }
            | Token::CloseTag { raw, .. }
            | Token::SelfClosing { raw, .. } => {
                tags.push(raw.clone());
                let placeholder = format!("&({})", tags.len());
                text.push_str(&placeholder);
            }
        }
    }

    Ok(SegregatedContent { text, tags })
}

/// Converts a text expression into a regex pattern that matches it flexibly
///
/// The pattern will:
/// - Be case-insensitive
/// - Allow any amount of whitespace/placeholders between words
/// - Escape special regex characters
/// - Match expressions bounded by spaces, punctuation, or placeholders
///
/// Example: "hello world" becomes a pattern that matches:
/// - "Hello World"
/// - "hello  world"
/// - "HELLO&(1)WORLD"
/// - etc.
pub fn expression_to_regex(expression: &str) -> Result<Regex> {
    // Normalize: lowercase and trim
    let normalized = expression.to_lowercase();
    let trimmed = normalized.trim();

    if trimmed.is_empty() {
        return Err(crate::TaggerError::InvalidExpression(
            "Expression cannot be empty".to_string(),
        ));
    }

    // Split by whitespace and escape each word
    let words: Vec<String> = trimmed.split_whitespace().map(regex::escape).collect();

    // Join with the ignorable pattern
    let pattern = words.join(IGNORABLE_RE.as_str());

    // Create regex with case-insensitive flag
    // Note: We'll handle boundaries differently during replacement
    Regex::new(&format!(r"(?i){}", pattern)).map_err(crate::TaggerError::RegexError)
}

/// Builds an opening HTML tag with attributes
///
/// Example: build_opening_tag("a", {"href": "test"}) => "<a href=\"test\">"
pub fn build_opening_tag(tag_name: &str, attributes: &HashMap<String, String>) -> String {
    if attributes.is_empty() {
        format!("<{}>", tag_name)
    } else {
        let attrs: Vec<String> = attributes
            .iter()
            .map(|(key, value)| {
                // Escape quotes in attribute values
                let escaped_value = value.replace('"', "&quot;");
                format!(r#"{}="{}""#, key, escaped_value)
            })
            .collect();

        format!("<{} {}>", tag_name, attrs.join(" "))
    }
}

/// Builds a closing HTML tag
///
/// Example: build_closing_tag("a") => "</a>"
pub fn build_closing_tag(tag_name: &str) -> String {
    format!("</{}>", tag_name)
}

/// Replaces expressions in text with tagged versions
///
/// This function:
/// - Finds all matches of the expression in the text
/// - Ensures matches are at word boundaries (preceded/followed by space, punctuation, or &)
/// - Replaces matches with tagged versions
/// - Handles placeholders within matches correctly
pub fn replace_expression_in_text(
    text: &str,
    expression: &TagExpression,
    tags: &mut Vec<String>,
) -> Result<String> {
    let regex = expression_to_regex(&expression.expression)?;

    // Create the opening and closing tags
    let open_tag = build_opening_tag(&expression.tag, &expression.attributes);
    let close_tag = build_closing_tag(&expression.tag);

    // Add spaces at start and end to simplify boundary matching
    let mut result = format!(" {} ", text);

    // Collect all matches with their positions
    let mut matches = Vec::new();

    // Find all matches in the text
    for mat in regex.find_iter(&result) {
        let start = mat.start();
        let end = mat.end();
        let matched = mat.as_str();

        // Check boundaries manually - simpler approach
        let before_ok = if start == 0 {
            true
        } else {
            // Get substring and check last char
            let before = &result[..start];
            if let Some(ch) = before.chars().last() {
                ch.is_whitespace()
                    || matches!(
                        ch,
                        '!' | '"'
                            | '#'
                            | '$'
                            | '%'
                            | '&'
                            | '\''
                            | '('
                            | ')'
                            | '*'
                            | '+'
                            | ','
                            | '-'
                            | '.'
                            | '/'
                            | ':'
                            | ';'
                            | '<'
                            | '='
                            | '>'
                            | '?'
                            | '@'
                            | '['
                            | '\\'
                            | ']'
                            | '^'
                            | '_'
                            | '`'
                            | '{'
                            | '|'
                            | '}'
                            | '~'
                    )
            } else {
                true
            }
        };

        let after_ok = if end >= result.len() {
            true
        } else {
            // Get the character after the match
            if let Some(ch) = result[end..].chars().next() {
                ch.is_whitespace()
                    || matches!(
                        ch,
                        '!' | '"'
                            | '#'
                            | '$'
                            | '%'
                            | '&'
                            | '\''
                            | '('
                            | ')'
                            | '*'
                            | '+'
                            | ','
                            | '-'
                            | '.'
                            | '/'
                            | ':'
                            | ';'
                            | '<'
                            | '='
                            | '>'
                            | '?'
                            | '@'
                            | '['
                            | '\\'
                            | ']'
                            | '^'
                            | '_'
                            | '`'
                            | '{'
                            | '|'
                            | '}'
                            | '~'
                    )
            } else {
                true
            }
        };

        if before_ok && after_ok {
            matches.push((start, end, matched.to_string()));
        }
    }

    // Sort matches by position (descending) to replace from end to start
    matches.sort_by(|a, b| b.0.cmp(&a.0));

    // Create placeholder regex once outside the loop
    let placeholder_regex = match Regex::new(r"(\&\(\d+\))") {
        Ok(regex) => regex,
        Err(e) => return Err(crate::TaggerError::RegexError(e)),
    };

    // Process each match
    for (start, end, matched_text) in matches {
        // Build the replacement with proper placeholder handling
        let replacement = if matched_text.contains("&(") {
            // Handle placeholders: wrap them with close/open tags
            let with_wrapped = placeholder_regex
                .replace_all(&matched_text, &format!("{close_tag}$1{open_tag}"))
                .to_string();
            format!("{open_tag}{with_wrapped}{close_tag}")
        } else {
            format!("{open_tag}{matched_text}{close_tag}")
        };

        // Store the replacement and create a placeholder
        tags.push(replacement);
        let placeholder = format!("&({})", tags.len());

        // Replace this match
        result.replace_range(start..end, &placeholder);
    }

    // Remove the padding spaces
    Ok(result.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::tokenize;

    #[test]
    fn test_segregate_simple() {
        let tokens = tokenize("<p>Hello</p>").unwrap();
        let segregated = segregate_markup_from_text(&tokens).unwrap();

        assert_eq!(segregated.text, "&(1)Hello&(2)");
        assert_eq!(segregated.tags.len(), 2);
        assert_eq!(segregated.tags[0], "<p>");
        assert_eq!(segregated.tags[1], "</p>");
    }

    #[test]
    fn test_segregate_with_attributes() {
        let tokens = tokenize("<a href=\"test\">Link</a>").unwrap();
        let segregated = segregate_markup_from_text(&tokens).unwrap();

        assert_eq!(segregated.text, "&(1)Link&(2)");
        assert_eq!(segregated.tags[0], "<a href=\"test\">");
        assert_eq!(segregated.tags[1], "</a>");
    }

    #[test]
    fn test_segregate_nested_tags() {
        let tokens = tokenize("<p>Hello <strong>World</strong>!</p>").unwrap();
        let segregated = segregate_markup_from_text(&tokens).unwrap();

        assert_eq!(segregated.text, "&(1)Hello &(2)World&(3)!&(4)");
        assert_eq!(segregated.tags.len(), 4);
        assert_eq!(segregated.tags[0], "<p>");
        assert_eq!(segregated.tags[1], "<strong>");
        assert_eq!(segregated.tags[2], "</strong>");
        assert_eq!(segregated.tags[3], "</p>");
    }

    #[test]
    fn test_segregate_self_closing() {
        let tokens = tokenize("Text<br/>More").unwrap();
        let segregated = segregate_markup_from_text(&tokens).unwrap();

        assert_eq!(segregated.text, "Text&(1)More");
        assert_eq!(segregated.tags.len(), 1);
        assert_eq!(segregated.tags[0], "<br/>");
    }

    #[test]
    fn test_segregate_only_text() {
        let tokens = tokenize("Just plain text").unwrap();
        let segregated = segregate_markup_from_text(&tokens).unwrap();

        assert_eq!(segregated.text, "Just plain text");
        assert_eq!(segregated.tags.len(), 0);
    }

    #[test]
    fn test_segregate_only_tags() {
        let tokens = tokenize("<div></div>").unwrap();
        let segregated = segregate_markup_from_text(&tokens).unwrap();

        assert_eq!(segregated.text, "&(1)&(2)");
        assert_eq!(segregated.tags.len(), 2);
    }

    #[test]
    fn test_segregate_complex_example() {
        let html = r#"Abstract

The Extensible Markup Language (XML) is a subset of <strong>SGML</strong>
that is <a href="foo">completely described</a> in this document."#;

        let tokens = tokenize(html).unwrap();
        let segregated = segregate_markup_from_text(&tokens).unwrap();

        // Check that SGML is surrounded by placeholders
        assert!(segregated.text.contains("&(1)SGML&(2)"));
        // Check that "completely described" is surrounded by placeholders
        assert!(segregated.text.contains("&(3)completely described&(4)"));
        assert_eq!(segregated.tags.len(), 4);
    }

    #[test]
    fn test_expression_to_regex_simple() {
        let regex = expression_to_regex("hello world").unwrap();

        // Should match various cases
        assert!(regex.is_match("hello world"));
        assert!(regex.is_match("Hello World"));
        assert!(regex.is_match("HELLO WORLD"));
        assert!(regex.is_match("hello  world")); // extra space
        assert!(regex.is_match("hello\nworld")); // newline
        assert!(regex.is_match("hello&(1)world")); // placeholder
    }

    #[test]
    fn test_expression_to_regex_special_chars() {
        let regex = expression_to_regex("test (with parens)").unwrap();

        // Should escape special regex characters
        assert!(regex.is_match("test (with parens)"));
        assert!(regex.is_match("TEST (WITH PARENS)"));

        // Should not match without proper escaping
        assert!(!regex.is_match("test with parens")); // missing parens
    }

    #[test]
    fn test_expression_to_regex_empty() {
        assert!(expression_to_regex("").is_err());
        assert!(expression_to_regex("   ").is_err());
    }

    #[test]
    fn test_expression_to_regex_single_word() {
        let regex = expression_to_regex("world").unwrap();

        assert!(regex.is_match("world"));
        assert!(regex.is_match("World"));
        assert!(regex.is_match("WORLD"));

        // Note: boundary checking will be done during replacement,
        // not in the regex itself
        assert!(regex.is_match("worldwide")); // this is expected
    }

    #[test]
    fn test_expression_to_regex_with_punctuation() {
        let regex = expression_to_regex("hello, world!").unwrap();

        assert!(regex.is_match("hello, world!"));
        assert!(regex.is_match("HELLO, WORLD!"));
        assert!(regex.is_match("hello,  world!")); // extra space
    }

    #[test]
    fn test_ignorable_regex() {
        // Test the IGNORABLE_RE pattern directly
        assert!(IGNORABLE_RE.is_match(" "));
        assert!(IGNORABLE_RE.is_match("\n"));
        assert!(IGNORABLE_RE.is_match("\r"));
        assert!(IGNORABLE_RE.is_match("&(123)"));
        assert!(IGNORABLE_RE.is_match(" \n&(1) \r "));
        assert!(IGNORABLE_RE.is_match("")); // matches empty
    }

    #[test]
    fn test_build_opening_tag_simple() {
        let attrs = HashMap::new();
        assert_eq!(build_opening_tag("div", &attrs), "<div>");
        assert_eq!(build_opening_tag("p", &attrs), "<p>");
    }

    #[test]
    fn test_build_opening_tag_with_attributes() {
        let mut attrs = HashMap::new();
        attrs.insert("href".to_string(), "https://example.com".to_string());
        assert_eq!(
            build_opening_tag("a", &attrs),
            r#"<a href="https://example.com">"#
        );

        attrs.insert("class".to_string(), "link".to_string());
        // Note: HashMap order is not guaranteed, so we check both possibilities
        let result = build_opening_tag("a", &attrs);
        assert!(
            result == r#"<a href="https://example.com" class="link">"#
                || result == r#"<a class="link" href="https://example.com">"#
        );
    }

    #[test]
    fn test_build_opening_tag_escape_quotes() {
        let mut attrs = HashMap::new();
        attrs.insert(
            "title".to_string(),
            r#"Click "here" to continue"#.to_string(),
        );
        assert_eq!(
            build_opening_tag("span", &attrs),
            r#"<span title="Click &quot;here&quot; to continue">"#
        );
    }

    #[test]
    fn test_build_closing_tag() {
        assert_eq!(build_closing_tag("div"), "</div>");
        assert_eq!(build_closing_tag("a"), "</a>");
        assert_eq!(build_closing_tag("strong"), "</strong>");
    }

    #[test]
    fn test_replace_expression_simple() {
        let mut tags = vec![];
        let expr = TagExpression::new("world", "strong");

        let result = replace_expression_in_text("Hello world!", &expr, &mut tags).unwrap();

        assert_eq!(result, "Hello &(1)!");
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0], "<strong>world</strong>");
    }

    #[test]
    fn test_replace_expression_with_attributes() {
        let mut tags = vec![];
        let expr = TagExpression::new("example", "a").with_attribute("href", "https://example.com");

        let result = replace_expression_in_text("Visit example site", &expr, &mut tags).unwrap();

        assert_eq!(result, "Visit &(1) site");
        assert_eq!(tags[0], r#"<a href="https://example.com">example</a>"#);
    }

    #[test]
    fn test_replace_expression_case_insensitive() {
        let mut tags = vec![];
        let expr = TagExpression::new("hello", "em");

        let result =
            replace_expression_in_text("HELLO world, hello there!", &expr, &mut tags).unwrap();

        // Should match both HELLO and hello
        assert!(result.contains("&(1)"));
        assert!(result.contains("&(2)"));
        assert_eq!(tags.len(), 2);
    }

    #[test]
    fn test_replace_expression_boundaries() {
        let mut tags = vec![];
        let expr = TagExpression::new("test", "span");

        let result =
            replace_expression_in_text("test testing test! test.", &expr, &mut tags).unwrap();

        // Should match "test" but not "testing"
        assert!(result.contains("&(1)"));
        assert!(!result.contains("&(1)ing"));
        assert!(result.contains("testing")); // unchanged
    }

    #[test]
    fn test_replace_expression_with_placeholders() {
        let mut tags = vec!["<em>emphasis</em>".to_string()];
        let expr = TagExpression::new("with &(1)", "strong");

        let result = replace_expression_in_text("Text with &(1) here", &expr, &mut tags).unwrap();

        // Should handle the placeholder within the match
        assert_eq!(tags.len(), 2);
        assert_eq!(result, "Text &(2) here");
        assert_eq!(tags[1], "<strong>with </strong>&(1)<strong></strong>");
    }

    #[test]
    fn test_replace_expression_punctuation_boundaries() {
        let mut tags = vec![];
        let expr = TagExpression::new("hello", "b");

        let tests = vec![
            ("Say hello!", "Say &(1)!"),
            ("hello, world", "&(1), world"),
            ("(hello)", "(&(1))"),
            ("'hello'", "'&(1)'"),
        ];

        for (input, expected) in tests {
            tags.clear();
            let result = replace_expression_in_text(input, &expr, &mut tags).unwrap();
            assert_eq!(result, expected, "Failed for input: {}", input);
        }
    }
}
