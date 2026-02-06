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
/// Loads ALL active custom templates from the site's database and
/// overlays them on a clone of the default Tera engine. This ensures
/// that parent templates (e.g. `base.html`) are also overridden when
/// a child template uses `{% extends "base.html" %}`.
///
/// Falls back to the default filesystem templates when no overrides exist.
pub async fn render_with_overrides(
    default_engine: &TemplateEngine,
    db: &SqlitePool,
    template_name: &str,
    context: &Context,
) -> Result<String> {
    let repo = SiteTemplateRepository::new(db.clone());
    let active_templates = repo.list_all_active().await?;

    if active_templates.is_empty() {
        return default_engine.render(template_name, context);
    }

    render_with_all_overrides(default_engine, template_name, &active_templates, context)
}

fn render_with_all_overrides(
    default_engine: &TemplateEngine,
    template_name: &str,
    templates: &[doxyde_core::models::site_template::SiteTemplate],
    context: &Context,
) -> Result<String> {
    let base_tera = default_engine.tera();
    let mut tera = (*base_tera).clone();
    for template in templates {
        tera.add_raw_template(&template.template_name, &template.content)?;
    }
    Ok(tera.render(template_name, context)?)
}
