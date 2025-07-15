use crate::Result;

/// Represents a token in HTML content
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// Plain text content
    Text(String),
    /// Opening tag with name and raw attributes string
    OpenTag { name: String, raw: String },
    /// Closing tag
    CloseTag { name: String, raw: String },
    /// Self-closing tag
    SelfClosing { name: String, raw: String },
}

impl Token {
    /// Returns the raw string representation of the token
    pub fn as_str(&self) -> &str {
        match self {
            Token::Text(s) => s,
            Token::OpenTag { raw, .. } => raw,
            Token::CloseTag { raw, .. } => raw,
            Token::SelfClosing { raw, .. } => raw,
        }
    }

    /// Returns true if this is a text token
    pub fn is_text(&self) -> bool {
        matches!(self, Token::Text(_))
    }

    /// Returns true if this is any kind of tag
    pub fn is_tag(&self) -> bool {
        !self.is_text()
    }
}

/// Tokenizes HTML content into a vector of tokens
/// This is a simple tokenizer that handles basic HTML structure
pub fn tokenize(html: &str) -> Result<Vec<Token>> {
    let mut tokens = Vec::new();
    let mut chars = html.chars().peekable();
    let mut current_text = String::new();

    while let Some(ch) = chars.next() {
        if ch == '<' {
            // Save any accumulated text
            if !current_text.is_empty() {
                tokens.push(Token::Text(current_text.clone()));
                current_text.clear();
            }

            // Read the tag
            let mut tag_content = String::from("<");
            let mut in_quotes = false;
            let mut quote_char = ' ';

            while let Some(&next_ch) = chars.peek() {
                chars.next();
                tag_content.push(next_ch);

                // Handle quotes to avoid breaking on > inside attributes
                if !in_quotes && (next_ch == '"' || next_ch == '\'') {
                    in_quotes = true;
                    quote_char = next_ch;
                } else if in_quotes && next_ch == quote_char {
                    in_quotes = false;
                }

                // End of tag
                if !in_quotes && next_ch == '>' {
                    break;
                }
            }

            // Parse the tag type
            if let Some(stripped) = tag_content.strip_prefix("</") {
                // Closing tag
                let name = extract_tag_name(stripped);
                tokens.push(Token::CloseTag {
                    name: name.to_string(),
                    raw: tag_content,
                });
            } else if tag_content.ends_with("/>") {
                // Self-closing tag
                let name = extract_tag_name(&tag_content[1..]);
                tokens.push(Token::SelfClosing {
                    name: name.to_string(),
                    raw: tag_content,
                });
            } else {
                // Opening tag
                let name = extract_tag_name(&tag_content[1..]);
                tokens.push(Token::OpenTag {
                    name: name.to_string(),
                    raw: tag_content,
                });
            }
        } else {
            current_text.push(ch);
        }
    }

    // Don't forget any remaining text
    if !current_text.is_empty() {
        tokens.push(Token::Text(current_text));
    }

    Ok(tokens)
}

/// Extracts the tag name from tag content
fn extract_tag_name(content: &str) -> &str {
    content
        .split(|c: char| c.is_whitespace() || c == '>' || c == '/')
        .next()
        .unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_as_str() {
        let token = Token::Text("hello".to_string());
        assert_eq!(token.as_str(), "hello");

        let token = Token::OpenTag {
            name: "div".to_string(),
            raw: "<div class=\"test\">".to_string(),
        };
        assert_eq!(token.as_str(), "<div class=\"test\">");
    }

    #[test]
    fn test_token_is_text() {
        let token = Token::Text("hello".to_string());
        assert!(token.is_text());
        assert!(!token.is_tag());

        let token = Token::OpenTag {
            name: "div".to_string(),
            raw: "<div>".to_string(),
        };
        assert!(!token.is_text());
        assert!(token.is_tag());
    }

    #[test]
    fn test_tokenize_simple_text() {
        let tokens = tokenize("Hello, World!").unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0], Token::Text("Hello, World!".to_string()));
    }

    #[test]
    fn test_tokenize_simple_tag() {
        let tokens = tokenize("<div>Hello</div>").unwrap();
        assert_eq!(tokens.len(), 3);
        assert_eq!(
            tokens[0],
            Token::OpenTag {
                name: "div".to_string(),
                raw: "<div>".to_string(),
            }
        );
        assert_eq!(tokens[1], Token::Text("Hello".to_string()));
        assert_eq!(
            tokens[2],
            Token::CloseTag {
                name: "div".to_string(),
                raw: "</div>".to_string(),
            }
        );
    }

    #[test]
    fn test_tokenize_self_closing_tag() {
        let tokens = tokenize("Before<br/>After").unwrap();
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0], Token::Text("Before".to_string()));
        assert_eq!(
            tokens[1],
            Token::SelfClosing {
                name: "br".to_string(),
                raw: "<br/>".to_string(),
            }
        );
        assert_eq!(tokens[2], Token::Text("After".to_string()));
    }

    #[test]
    fn test_tokenize_tag_with_attributes() {
        let tokens = tokenize("<a href=\"https://example.com\" class='link'>Link</a>").unwrap();
        assert_eq!(tokens.len(), 3);
        assert_eq!(
            tokens[0],
            Token::OpenTag {
                name: "a".to_string(),
                raw: "<a href=\"https://example.com\" class='link'>".to_string(),
            }
        );
        assert_eq!(tokens[1], Token::Text("Link".to_string()));
        assert_eq!(
            tokens[2],
            Token::CloseTag {
                name: "a".to_string(),
                raw: "</a>".to_string(),
            }
        );
    }

    #[test]
    fn test_tokenize_nested_tags() {
        let tokens = tokenize("<p>Hello <strong>World</strong>!</p>").unwrap();
        assert_eq!(tokens.len(), 7);
        assert_eq!(
            tokens[0],
            Token::OpenTag {
                name: "p".to_string(),
                raw: "<p>".to_string(),
            }
        );
        assert_eq!(tokens[1], Token::Text("Hello ".to_string()));
        assert_eq!(
            tokens[2],
            Token::OpenTag {
                name: "strong".to_string(),
                raw: "<strong>".to_string(),
            }
        );
        assert_eq!(tokens[3], Token::Text("World".to_string()));
        assert_eq!(
            tokens[4],
            Token::CloseTag {
                name: "strong".to_string(),
                raw: "</strong>".to_string(),
            }
        );
        assert_eq!(tokens[5], Token::Text("!".to_string()));
        assert_eq!(
            tokens[6],
            Token::CloseTag {
                name: "p".to_string(),
                raw: "</p>".to_string(),
            }
        );
    }

    #[test]
    fn test_tokenize_empty_string() {
        let tokens = tokenize("").unwrap();
        assert_eq!(tokens.len(), 0);
    }

    #[test]
    fn test_tokenize_only_tags() {
        let tokens = tokenize("<div></div>").unwrap();
        assert_eq!(tokens.len(), 2);
        assert!(tokens[0].is_tag());
        assert!(tokens[1].is_tag());
    }

    #[test]
    fn test_tokenize_malformed_tag() {
        // Even malformed HTML should tokenize without panicking
        let tokens = tokenize("<div").unwrap();
        assert_eq!(tokens.len(), 1);
        // The unclosed tag becomes text or a malformed tag
    }

    #[test]
    fn test_extract_tag_name() {
        assert_eq!(extract_tag_name("div>"), "div");
        assert_eq!(extract_tag_name("div class=\"test\">"), "div");
        assert_eq!(extract_tag_name("br/>"), "br");
        assert_eq!(extract_tag_name(""), "");
    }
}
