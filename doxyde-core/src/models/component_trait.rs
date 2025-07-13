use serde_json::Value;

/// Trait for rendering components with different templates
pub trait ComponentRenderer {
    /// Render the component with the specified template
    fn render(&self, template: &str) -> String;

    /// Get list of available templates for this component type
    fn get_available_templates(&self) -> Vec<&'static str>;

    /// Get the default template name
    fn get_default_template(&self) -> &'static str {
        "default"
    }
}

/// Helper function to safely extract text from JSON value
pub fn extract_text(value: &Value, field: &str) -> String {
    value
        .get(field)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

/// Helper function to escape HTML
pub fn escape_html(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            '<' => "&lt;".to_string(),
            '>' => "&gt;".to_string(),
            '&' => "&amp;".to_string(),
            '"' => "&quot;".to_string(),
            '\'' => "&#39;".to_string(),
            _ => c.to_string(),
        })
        .collect()
}
