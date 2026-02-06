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

use anyhow::Result;
use doxyde_db::repositories::SiteTemplateRepository;
use sqlx::SqlitePool;
use tera::Context;

use crate::autoreload_templates::TemplateEngine;

/// Render a template with per-site DB overrides applied.
///
/// If the site has an active custom template matching `template_name`,
/// a clone of the default Tera engine is created with the override
/// injected via `add_raw_template`. This preserves all registered
/// filters and functions while allowing the custom template to
/// `{% extends "base.html" %}` normally.
///
/// Falls back to the default filesystem template when no override exists.
pub async fn render_with_overrides(
    default_engine: &TemplateEngine,
    db: &SqlitePool,
    template_name: &str,
    context: &Context,
) -> Result<String> {
    let repo = SiteTemplateRepository::new(db.clone());
    let custom = repo.find_active_by_name(template_name).await?;

    match custom {
        Some(template) => {
            render_with_custom(default_engine, template_name, &template.content, context)
        }
        None => default_engine.render(template_name, context),
    }
}

fn render_with_custom(
    default_engine: &TemplateEngine,
    template_name: &str,
    custom_content: &str,
    context: &Context,
) -> Result<String> {
    let base_tera = default_engine.tera();
    let mut tera = (*base_tera).clone();
    tera.add_raw_template(template_name, custom_content)?;
    Ok(tera.render(template_name, context)?)
}
