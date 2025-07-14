use std::collections::HashMap;

/// Represents a text expression to be tagged with HTML markup
#[derive(Debug, Clone, PartialEq)]
pub struct TagExpression {
    /// The text expression to match (case-insensitive)
    pub expression: String,
    /// The HTML tag name to wrap matches with
    pub tag: String,
    /// Additional attributes for the tag
    pub attributes: HashMap<String, String>,
}

impl TagExpression {
    /// Creates a new TagExpression
    pub fn new(expression: impl Into<String>, tag: impl Into<String>) -> Self {
        Self {
            expression: expression.into(),
            tag: tag.into(),
            attributes: HashMap::new(),
        }
    }

    /// Adds an attribute to the tag
    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    /// Validates that the expression and tag are valid
    pub fn validate(&self) -> crate::Result<()> {
        if self.expression.trim().is_empty() {
            return Err(crate::error::TaggerError::InvalidExpression(
                "Expression cannot be empty".to_string(),
            ));
        }

        if self.tag.trim().is_empty() {
            return Err(crate::error::TaggerError::InvalidTag(
                "Tag name cannot be empty".to_string(),
            ));
        }

        // Basic tag name validation - alphanumeric plus hyphen
        if !self.tag.chars().all(|c| c.is_alphanumeric() || c == '-') {
            return Err(crate::error::TaggerError::InvalidTag(format!(
                "Invalid tag name: {}",
                self.tag
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_expression_new() {
        let expr = TagExpression::new("hello world", "strong");
        assert_eq!(expr.expression, "hello world");
        assert_eq!(expr.tag, "strong");
        assert!(expr.attributes.is_empty());
    }

    #[test]
    fn test_tag_expression_with_attributes() {
        let expr = TagExpression::new("example", "a")
            .with_attribute("href", "https://example.com")
            .with_attribute("class", "link");

        assert_eq!(expr.expression, "example");
        assert_eq!(expr.tag, "a");
        assert_eq!(expr.attributes.len(), 2);
        assert_eq!(
            expr.attributes.get("href"),
            Some(&"https://example.com".to_string())
        );
        assert_eq!(expr.attributes.get("class"), Some(&"link".to_string()));
    }

    #[test]
    fn test_validate_valid_expression() {
        let expr = TagExpression::new("valid expression", "div");
        assert!(expr.validate().is_ok());

        let expr = TagExpression::new("another", "my-tag");
        assert!(expr.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_expression() {
        let expr = TagExpression::new("", "div");
        assert!(expr.validate().is_err());

        let expr = TagExpression::new("   ", "div");
        assert!(expr.validate().is_err());
    }

    #[test]
    fn test_validate_empty_tag() {
        let expr = TagExpression::new("hello", "");
        assert!(expr.validate().is_err());

        let expr = TagExpression::new("hello", "   ");
        assert!(expr.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_tag_name() {
        let expr = TagExpression::new("hello", "div>");
        assert!(expr.validate().is_err());

        let expr = TagExpression::new("hello", "<script");
        assert!(expr.validate().is_err());

        let expr = TagExpression::new("hello", "div class='foo'");
        assert!(expr.validate().is_err());
    }
}
