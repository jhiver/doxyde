use thiserror::Error;

#[derive(Error, Debug)]
pub enum TaggerError {
    #[error("Invalid expression: {0}")]
    InvalidExpression(String),

    #[error("Invalid tag name: {0}")]
    InvalidTag(String),

    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),

    #[error("Tokenization failed: {0}")]
    TokenizationError(String),
}

pub type Result<T> = std::result::Result<T, TaggerError>;
