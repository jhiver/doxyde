// Doxyde - A modern, AI-native CMS built with Rust
// Copyright (C) 2025 Doxyde Project Contributors
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use crate::models::component::Component;
use crate::models::component_trait::{escape_html, ComponentRenderer};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactFormField {
    pub name: String,
    pub label: String,
    pub r#type: String, // "text", "email", "textarea", etc.
    pub required: bool,
    pub placeholder: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactFormConfig {
    pub recipient_email: String,
    pub sender_email: Option<String>,
    pub smtp_host: Option<String>,
    pub smtp_port: Option<u16>,
    pub smtp_username: Option<String>,
    pub smtp_password: Option<String>,
    pub smtp_encryption: Option<String>,
    pub submit_button_text: Option<String>,
    pub success_message: Option<String>,
    pub fields: Option<Vec<ContactFormField>>,
}

pub struct ContactFormComponent {
    pub id: Option<i64>,
    pub config: ContactFormConfig,
    pub title: Option<String>,
}

impl ContactFormComponent {
    pub fn from_component(component: &Component) -> Self {
        let config: ContactFormConfig = serde_json::from_value(component.content.clone())
            .unwrap_or_else(|_| ContactFormConfig {
                recipient_email: "contact@example.com".to_string(),
                sender_email: None,
                smtp_host: None,
                smtp_port: None,
                smtp_username: None,
                smtp_password: None,
                smtp_encryption: None,
                submit_button_text: Some("Envoyer".to_string()),
                success_message: Some("Votre message a bien été envoyé !".to_string()),
                fields: None,
            });

        Self {
            id: component.id,
            config,
            title: component.title.clone(),
        }
    }
}

impl ComponentRenderer for ContactFormComponent {
    fn render(&self, template: &str) -> String {
        match template {
            "default" => self.render_default(),
            _ => self.render("default"),
        }
    }
}

impl ContactFormComponent {
    fn render_default(&self) -> String {
        let id_str = self
            .id
            .map(|id| id.to_string())
            .unwrap_or_else(|| "new".to_string());
        let button_text = self
            .config
            .submit_button_text
            .as_deref()
            .unwrap_or("Envoyer");
        let fields_html = self.render_fields(&id_str);

        format!(
            r#"<div class="contact-form-component">
    <form action="/.contact-submit/{id_str}" method="POST">
        <!-- Honeypot -->
        <div class="form-group" style="display:none !important; visibility:hidden !important;">
            <label for="website_url_{id_str}">Website URL</label>
            <input type="text" id="website_url_{id_str}" name="website_url" tabindex="-1" autocomplete="off">
        </div>
        {fields_html}
        <button type="submit" class="submit-button">{button_text}</button>
    </form>
</div>"#,
            id_str = id_str,
            fields_html = fields_html,
            button_text = escape_html(button_text)
        )
    }

    fn render_fields(&self, id_str: &str) -> String {
        let default_fields = get_default_fields();
        let fields = self.config.fields.as_ref().unwrap_or(&default_fields);
        let mut fields_html = String::new();

        for field in fields {
            let req_attr = if field.required { "required" } else { "" };
            let placeholder_attr = field
                .placeholder
                .as_deref()
                .map(|p| format!("placeholder=\"{}\"", escape_html(p)))
                .unwrap_or_default();
            fields_html.push_str(&self.render_single_field(
                id_str,
                field,
                req_attr,
                &placeholder_attr,
            ));
        }
        fields_html
    }

    fn render_single_field(
        &self,
        id_str: &str,
        field: &ContactFormField,
        req_attr: &str,
        placeholder_attr: &str,
    ) -> String {
        let name = escape_html(&field.name);
        let label = escape_html(&field.label);
        match field.r#type.as_str() {
            "textarea" => format!(
                r#"<div class="form-group">
    <label for="field_{id_str}_{name}">{label}</label>
    <textarea id="field_{id_str}_{name}" name="{name}" {req_attr} {placeholder_attr}></textarea>
</div>
"#,
            ),
            _ => format!(
                r#"<div class="form-group">
    <label for="field_{id_str}_{name}">{label}</label>
    <input type="{field_type}" id="field_{id_str}_{name}" name="{name}" {req_attr} {placeholder_attr}>
</div>
"#,
                field_type = escape_html(&field.r#type),
            ),
        }
    }
}

fn get_default_fields() -> Vec<ContactFormField> {
    vec![
        ContactFormField {
            name: "name".to_string(),
            label: "Nom".to_string(),
            r#type: "text".to_string(),
            required: true,
            placeholder: Some("Votre nom...".to_string()),
        },
        ContactFormField {
            name: "email".to_string(),
            label: "Email".to_string(),
            r#type: "email".to_string(),
            required: true,
            placeholder: Some("Votre adresse email...".to_string()),
        },
        ContactFormField {
            name: "message".to_string(),
            label: "Message".to_string(),
            r#type: "textarea".to_string(),
            required: true,
            placeholder: Some("Votre message...".to_string()),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_contact_form_from_component() {
        let component = Component::new(
            1,
            "contact_form".to_string(),
            0,
            json!({
                "recipient_email": "hello@example.com",
                "submit_button_text": "Submit"
            }),
        );
        let comp = ContactFormComponent::from_component(&component);

        assert_eq!(comp.config.recipient_email, "hello@example.com");
        assert_eq!(comp.config.submit_button_text.as_deref(), Some("Submit"));
    }

    #[test]
    fn test_contact_form_render_default() {
        let comp = ContactFormComponent {
            id: Some(42),
            config: ContactFormConfig {
                recipient_email: "test@example.com".to_string(),
                sender_email: None,
                smtp_host: None,
                smtp_port: None,
                smtp_username: None,
                smtp_password: None,
                smtp_encryption: None,
                submit_button_text: Some("Send".to_string()),
                success_message: None,
                fields: None,
            },
            title: None,
        };

        let html = comp.render("default");
        assert!(html.contains("action=\"/.contact-submit/42\""));
        assert!(html.contains("type=\"text\""));
        assert!(html.contains("name=\"website_url\""));
        assert!(html.contains(">Send</button>"));
    }
}
