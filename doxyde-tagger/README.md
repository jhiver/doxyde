# doxyde-tagger

HTML auto-tagging library for Doxyde CMS. This is a Rust port of the Perl MKDoc::XML::Tagger module.

## Features

- Automatically add HTML markup to text by matching expressions
- Case-insensitive matching with flexible whitespace handling
- Preserve existing tags to avoid double-tagging
- Process expressions longest-first to handle overlapping matches
- Handle malformed HTML gracefully

## Usage

Basic usage:

```rust
use doxyde_tagger::{HtmlTagger, TagExpression};

let mut tagger = HtmlTagger::new();
tagger.add_expression(
    TagExpression::new("Rust", "a")
        .with_attribute("href", "https://rust-lang.org")
).unwrap();

let html = "<p>I love Rust programming!</p>";
let result = tagger.process(html).unwrap();
// Result: <p>I love <a href="https://rust-lang.org">Rust</a> programming!</p>
```

Preserve existing tags:

```rust
use doxyde_tagger::{PreserveTagger, TagExpression};

let mut tagger = PreserveTagger::new(vec!["a".to_string()]);
tagger.add_expression(
    TagExpression::new("example", "strong")
).unwrap();

let html = r#"Visit <a href="test">example</a> for an example"#;
let result = tagger.process(html).unwrap();
// "example" inside <a> is preserved, only the second one is tagged
// Result: Visit <a href="test">example</a> for an <strong>example</strong>
```

## Use Cases

- Auto-linking: Convert mentions of terms into hyperlinks
- Glossaries: Automatically mark up abbreviations and definitions
- Wiki-style linking: Create automatic cross-references between pages
- SEO enhancement: Add semantic markup to improve content structure

## License

AGPL-3.0 (same as Doxyde)