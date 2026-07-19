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

use crate::db_middleware::SiteDatabase;
use axum::response::Html;
use axum::{
    extract::{Form, Path},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Json, Redirect, Response},
};
use doxyde_db::repositories::ComponentRepository;
use lettre::{transport::smtp::authentication::Credentials, Message, SmtpTransport, Transport};
use std::collections::HashMap;

pub async fn submit_handler(
    SiteDatabase(db): SiteDatabase,
    Path(component_id): Path<i64>,
    headers: HeaderMap,
    Form(form_data): Form<HashMap<String, String>>,
) -> Result<Response, StatusCode> {
    if is_honeypot_triggered(&form_data) {
        let msg = "Votre message a bien été envoyé !";
        return Ok(success_response(&headers, msg).into_response());
    }

    let component_repo = ComponentRepository::new(db);
    let component = get_component(&component_repo, component_id).await?;
    let config = parse_config(&component)?;

    let email_body = build_and_validate_body(&config, &form_data)?;
    send_email(&config, &email_body, component_id).await?;

    let success_msg = config
        .success_message
        .as_deref()
        .unwrap_or("Votre message a bien été envoyé !");
    Ok(success_response(&headers, success_msg))
}

fn is_honeypot_triggered(form_data: &HashMap<String, String>) -> bool {
    form_data
        .get("website_url")
        .map(|val| !val.trim().is_empty())
        .unwrap_or(false)
}

async fn get_component(
    repo: &ComponentRepository,
    id: i64,
) -> Result<doxyde_core::models::component::Component, StatusCode> {
    let comp = repo.find_by_id(id).await.map_err(|e| {
        tracing::error!(error = ?e, component_id = id, "Failed to find component");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    let component = comp.ok_or(StatusCode::NOT_FOUND)?;
    if component.component_type != "contact_form" {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(component)
}

fn parse_config(
    component: &doxyde_core::models::component::Component,
) -> Result<doxyde_core::models::components::contact_form_component::ContactFormConfig, StatusCode>
{
    serde_json::from_value(component.content.clone()).map_err(|e| {
        tracing::error!(error = ?e, "Failed to parse contact_form config");
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

fn build_and_validate_body(
    config: &doxyde_core::models::components::contact_form_component::ContactFormConfig,
    form_data: &HashMap<String, String>,
) -> Result<String, StatusCode> {
    let mut email_body = String::new();
    email_body.push_str("Nouveau message reçu depuis le formulaire de contact :\n\n");

    let default_fields = get_default_fields();
    let fields = config.fields.as_ref().unwrap_or(&default_fields);
    for field in fields {
        let value = form_data.get(&field.name).cloned().unwrap_or_default();
        if field.required && value.trim().is_empty() {
            return Err(StatusCode::BAD_REQUEST);
        }
        email_body.push_str(&format!("{}: {}\n", field.label, value));
    }
    Ok(email_body)
}

fn get_default_fields(
) -> Vec<doxyde_core::models::components::contact_form_component::ContactFormField> {
    use doxyde_core::models::components::contact_form_component::ContactFormField;
    vec![
        ContactFormField {
            name: "name".to_string(),
            label: "Nom".to_string(),
            r#type: "text".to_string(),
            required: true,
            placeholder: None,
        },
        ContactFormField {
            name: "email".to_string(),
            label: "Email".to_string(),
            r#type: "email".to_string(),
            required: true,
            placeholder: None,
        },
        ContactFormField {
            name: "message".to_string(),
            label: "Message".to_string(),
            r#type: "textarea".to_string(),
            required: true,
            placeholder: None,
        },
    ]
}

async fn send_email(
    config: &doxyde_core::models::components::contact_form_component::ContactFormConfig,
    body: &str,
    component_id: i64,
) -> Result<(), StatusCode> {
    if let (Some(smtp_host), Some(smtp_username), Some(_smtp_password)) = (
        &config.smtp_host,
        &config.smtp_username,
        &config.smtp_password,
    ) {
        if !smtp_host.trim().is_empty() && !smtp_username.trim().is_empty() {
            let email = build_message(config, body)?;
            let mailer = build_mailer(config, smtp_host)?;
            if let Err(e) = mailer.send(&email) {
                tracing::error!(error = ?e, "Failed to send email via SMTP");
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    } else {
        tracing::warn!(
            "SMTP host/creds not configured for form {}. E-mail not sent.",
            component_id
        );
    }
    Ok(())
}

fn build_message(
    config: &doxyde_core::models::components::contact_form_component::ContactFormConfig,
    body: &str,
) -> Result<Message, StatusCode> {
    let sender = config
        .sender_email
        .clone()
        .unwrap_or_else(|| "no-reply@example.com".to_string());
    Message::builder()
        .from(sender.parse().map_err(|_| StatusCode::BAD_REQUEST)?)
        .to(config
            .recipient_email
            .parse()
            .map_err(|_| StatusCode::BAD_REQUEST)?)
        .subject("Nouveau message de contact")
        .body(body.to_string())
        .map_err(|e| {
            tracing::error!(error = ?e, "Failed to build email message");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

fn build_mailer(
    config: &doxyde_core::models::components::contact_form_component::ContactFormConfig,
    smtp_host: &str,
) -> Result<SmtpTransport, StatusCode> {
    let smtp_port = config.smtp_port.unwrap_or(587);
    let username = config.smtp_username.clone().unwrap_or_default();
    let password = config.smtp_password.clone().unwrap_or_default();
    let creds = Credentials::new(username, password);
    let mailer_builder = SmtpTransport::builder_dangerous(smtp_host)
        .port(smtp_port)
        .credentials(creds);

    let encryption = config.smtp_encryption.as_deref().unwrap_or("starttls");
    match encryption {
        "ssl_tls" => {
            let tls = lettre::transport::smtp::client::Tls::Required(
                lettre::transport::smtp::client::TlsParameters::new(smtp_host.to_string())
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
            );
            Ok(mailer_builder.tls(tls).build())
        }
        "none" => Ok(mailer_builder
            .tls(lettre::transport::smtp::client::Tls::None)
            .build()),
        _ => {
            let tls = lettre::transport::smtp::client::Tls::Required(
                lettre::transport::smtp::client::TlsParameters::new(smtp_host.to_string())
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
            );
            Ok(mailer_builder.tls(tls).build())
        }
    }
}

fn success_response(headers: &HeaderMap, message: &str) -> Response {
    let wants_json = headers
        .get(axum::http::header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("application/json"))
        .unwrap_or(false);

    if wants_json {
        Json(serde_json::json!({
            "status": "success",
            "message": message
        }))
        .into_response()
    } else {
        match get_referer_redirect(headers) {
            Some(redirect) => redirect.into_response(),
            None => success_html_fallback(message).into_response(),
        }
    }
}

fn get_referer_redirect(headers: &HeaderMap) -> Option<Redirect> {
    let referer = headers
        .get(axum::http::header::REFERER)
        .and_then(|v| v.to_str().ok())?;
    let clean_referer = if referer.contains("contact_success=") {
        referer.split('?').next().unwrap_or(referer).to_string()
    } else {
        referer.to_string()
    };

    let redirect_url = if clean_referer.contains('?') {
        format!("{}&contact_success=1", clean_referer)
    } else {
        format!("{}?contact_success=1", clean_referer)
    };
    Some(Redirect::to(&redirect_url))
}

fn success_html_fallback(message: &str) -> Html<String> {
    Html(format!(
        r#"<!DOCTYPE html><html><head><meta charset="utf-8"><title>Message Envoyé</title>
<style>body {{ font-family: sans-serif; display: flex; justify-content: center; align-items: center; height: 100vh; margin: 0; background: #f8f9fa; }}
.card {{ background: white; padding: 2rem; border-radius: 8px; box-shadow: 0 4px 12px rgba(0,0,0,0.1); text-align: center; max-width: 400px; }}
h1 {{ color: #2b8a3e; margin: 0 0 1rem; }}
a {{ display: inline-block; margin-top: 1rem; color: #228be6; text-decoration: none; }}</style></head>
<body><div class="card"><h1>Merci !</h1><p>{}</p><a href="/">Retour au site</a></div></body></html>"#,
        message
    ))
}
