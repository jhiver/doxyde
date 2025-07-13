use crate::models::component::Component;
use crate::models::component_trait::{escape_html, extract_text, ComponentRenderer};

pub struct MarkdownComponent {
    pub id: Option<i64>,
    pub text: String,
    pub title: Option<String>,
}

impl MarkdownComponent {
    pub fn from_component(component: &Component) -> Self {
        Self {
            id: component.id,
            text: extract_text(&component.content, "text"),
            title: component.title.clone(),
        }
    }

    /// Convert markdown to HTML - this will be handled by the web layer
    /// For now, we'll just return the markdown wrapped in a data attribute
    fn markdown_placeholder(&self) -> String {
        format!(
            r#"<div data-markdown="{}">{}</div>"#,
            escape_html(&self.text).replace('"', "&quot;"),
            escape_html(&self.text)
        )
    }
}

impl ComponentRenderer for MarkdownComponent {
    fn render(&self, template: &str) -> String {
        let markdown_html = self.markdown_placeholder();

        match template {
            "default" => {
                format!(r#"<div class="markdown-component">{}</div>"#, markdown_html)
            }
            "with_title" => {
                if let Some(ref title) = self.title {
                    format!(
                        r#"<div class="markdown-component with-title">
    <h3 class="component-title">{}</h3>
    <div class="component-content">{}</div>
</div>"#,
                        escape_html(title),
                        markdown_html
                    )
                } else {
                    format!(
                        r#"<div class="markdown-component with-title">
    <div class="component-content">{}</div>
</div>"#,
                        markdown_html
                    )
                }
            }
            "card" => {
                let mut html = String::from(r#"<div class="markdown-component card">"#);
                if let Some(ref title) = self.title {
                    html.push_str(&format!(
                        r#"
    <div class="card-header">
        <h3 class="component-title">{}</h3>
    </div>"#,
                        escape_html(title)
                    ));
                }
                html.push_str(&format!(
                    r#"
    <div class="card-body">{}</div>
</div>"#,
                    markdown_html
                ));
                html
            }
            "highlight" => {
                let mut html = String::from(r#"<div class="markdown-component highlight">"#);
                if let Some(ref title) = self.title {
                    html.push_str(&format!(
                        r#"
    <h3 class="component-title">{}</h3>"#,
                        escape_html(title)
                    ));
                }
                html.push_str(&format!(
                    r#"
    <div class="component-content">{}</div>
</div>"#,
                    markdown_html
                ));
                html
            }
            "quote" => {
                let mut html = String::from(r#"<blockquote class="markdown-component quote">"#);
                if let Some(ref title) = self.title {
                    html.push_str(&format!(
                        r#"
    <h4 class="component-title">{}</h4>"#,
                        escape_html(title)
                    ));
                }
                html.push_str(&format!(
                    r#"
    <div class="component-content">{}</div>
</blockquote>"#,
                    markdown_html
                ));
                html
            }
            "hidden" => String::new(),
            _ => self.render("default"),
        }
    }

    fn get_available_templates(&self) -> Vec<&'static str> {
        vec![
            "default",
            "with_title",
            "card",
            "highlight",
            "quote",
            "hidden",
        ]
    }
}
