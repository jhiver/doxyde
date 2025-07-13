use crate::models::component::Component;
use crate::models::component_trait::ComponentRenderer;

pub struct CustomComponent {
    pub id: Option<i64>,
    pub component_type: String,
    pub content: serde_json::Value,
    pub title: Option<String>,
}

impl CustomComponent {
    pub fn from_component(component: &Component) -> Self {
        Self {
            id: component.id,
            component_type: component.component_type.clone(),
            content: component.content.clone(),
            title: component.title.clone(),
        }
    }
}

impl ComponentRenderer for CustomComponent {
    fn render(&self, _template: &str) -> String {
        format!(
            r#"<div class="custom-component" data-type="{}">{}</div>"#,
            self.component_type,
            serde_json::to_string_pretty(&self.content).unwrap_or_else(|_| "{}".to_string())
        )
    }

    fn get_available_templates(&self) -> Vec<&'static str> {
        vec!["default"]
    }
}
