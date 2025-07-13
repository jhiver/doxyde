use crate::models::component::Component;
use crate::models::component_trait::{extract_text, ComponentRenderer};

pub struct HtmlComponent {
    pub id: Option<i64>,
    pub html: String,
    pub title: Option<String>,
}

impl HtmlComponent {
    pub fn from_component(component: &Component) -> Self {
        Self {
            id: component.id,
            html: extract_text(&component.content, "html"),
            title: component.title.clone(),
        }
    }
}

impl ComponentRenderer for HtmlComponent {
    fn render(&self, template: &str) -> String {
        match template {
            "default" => {
                format!(r#"<div class="html-component">{}</div>"#, self.html)
            }
            _ => self.render("default"),
        }
    }

    fn get_available_templates(&self) -> Vec<&'static str> {
        vec!["default"]
    }
}
