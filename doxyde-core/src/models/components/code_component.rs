use crate::models::component::Component;
use crate::models::component_trait::{escape_html, extract_text, ComponentRenderer};

pub struct CodeComponent {
    pub id: Option<i64>,
    pub code: String,
    pub language: String,
    pub title: Option<String>,
}

impl CodeComponent {
    pub fn from_component(component: &Component) -> Self {
        Self {
            id: component.id,
            code: extract_text(&component.content, "code"),
            language: if extract_text(&component.content, "language").is_empty() {
                "plaintext".to_string()
            } else {
                extract_text(&component.content, "language")
            },
            title: component.title.clone(),
        }
    }
}

impl ComponentRenderer for CodeComponent {
    fn render(&self, template: &str) -> String {
        let escaped_code = escape_html(&self.code);

        match template {
            "default" => {
                format!(
                    r#"<div class="code-component">
    <pre><code class="language-{}">{}</code></pre>
</div>"#,
                    escape_html(&self.language),
                    escaped_code
                )
            }
            "with_title" => {
                let mut html = String::from(r#"<div class="code-component with-title">"#);
                if let Some(ref title) = self.title {
                    html.push_str(&format!(
                        r#"
    <h4 class="component-title">{}</h4>"#,
                        escape_html(title)
                    ));
                }
                html.push_str(&format!(
                    r#"
    <pre><code class="language-{}">{}</code></pre>
</div>"#,
                    escape_html(&self.language),
                    escaped_code
                ));
                html
            }
            _ => self.render("default"),
        }
    }

    fn get_available_templates(&self) -> Vec<&'static str> {
        vec!["default", "with_title"]
    }
}
