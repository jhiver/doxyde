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
use std::sync::{Arc, RwLock};
use tera::{Context, Tera};

use crate::component_render::{GetComponentTemplatesFunction, RenderComponentFunction};
use crate::markdown::make_markdown_filter;
use std::collections::HashMap;
use tera::{to_value, Filter, Value};

/// A wrapper around Tera that can reload templates in development mode
pub enum TemplateEngine {
    /// Static templates loaded once at startup
    Static(Arc<Tera>),
    /// Reloadable templates that refresh on each render
    Reloadable {
        templates_dir: String,
        cached: Arc<RwLock<Tera>>,
    },
}

impl TemplateEngine {
    /// Create a new template engine
    pub fn new(templates_dir: &str, development_mode: bool) -> Result<Self> {
        if development_mode {
            tracing::info!("Template hot reload enabled (development mode)");
            let tera = Self::create_tera_instance(templates_dir)?;
            Ok(Self::Reloadable {
                templates_dir: templates_dir.to_string(),
                cached: Arc::new(RwLock::new(tera)),
            })
        } else {
            tracing::info!("Templates loaded once (production mode)");
            let tera = Self::create_tera_instance(templates_dir)?;
            Ok(Self::Static(Arc::new(tera)))
        }
    }

    /// Create a configured Tera instance
    fn create_tera_instance(templates_dir: &str) -> Result<Tera> {
        // Load templates - include both HTML and CSS files
        let pattern = format!("{}/**/*", templates_dir);
        let mut tera = Tera::new(&pattern)?;

        // Register markdown filter
        tera.register_filter("markdown", make_markdown_filter());

        // Register round filter
        tera.register_filter("round", make_round_filter());

        // Register component rendering functions
        tera.register_function("render_component", RenderComponentFunction {
            templates_dir: templates_dir.to_string(),
        });
        tera.register_function("get_component_templates", GetComponentTemplatesFunction {
            templates_dir: templates_dir.to_string(),
        });

        Ok(tera)
    }

    /// Render a template
    pub fn render(&self, template_name: &str, context: &Context) -> Result<String> {
        match self {
            Self::Static(tera) => Ok(tera.render(template_name, context)?),
            Self::Reloadable {
                templates_dir,
                cached,
            } => {
                // In development mode, reload templates on each request
                match Self::create_tera_instance(templates_dir) {
                    Ok(new_tera) => {
                        // Update the cached instance
                        if let Ok(mut write_guard) = cached.write() {
                            *write_guard = new_tera;
                        }
                        // Use the updated instance
                        let read_guard = cached.read().unwrap();
                        Ok(read_guard.render(template_name, context)?)
                    }
                    Err(e) => {
                        // If reload fails, use the cached version and log the error
                        tracing::warn!("Failed to reload templates: {}. Using cached version.", e);
                        let read_guard = cached.read().unwrap();
                        Ok(read_guard.render(template_name, context)?)
                    }
                }
            }
        }
    }

    /// Get the underlying Tera instance (for backward compatibility)
    pub fn tera(&self) -> Arc<Tera> {
        match self {
            Self::Static(tera) => Arc::clone(tera),
            Self::Reloadable { cached, .. } => {
                let read_guard = cached.read().unwrap();
                // This is a bit inefficient but maintains compatibility
                Arc::new(read_guard.clone())
            }
        }
    }

    /// One-shot render for testing or backward compatibility
    pub fn one_shot_render(
        &self,
        template_name: &str,
        context: &tera::Context,
    ) -> tera::Result<String> {
        match self {
            Self::Static(tera) => tera.render(template_name, context),
            Self::Reloadable { cached, .. } => {
                let read_guard = cached.read().unwrap();
                read_guard.render(template_name, context)
            }
        }
    }

    /// Register a filter (for backward compatibility)
    pub fn register_filter<F>(&mut self, name: &str, filter: F)
    where
        F: tera::Filter + 'static,
    {
        match self {
            Self::Static(tera) => {
                if let Some(tera_mut) = Arc::get_mut(tera) {
                    tera_mut.register_filter(name, filter);
                }
            }
            Self::Reloadable { cached, .. } => {
                if let Ok(mut write_guard) = cached.write() {
                    write_guard.register_filter(name, filter);
                }
            }
        }
    }

    /// Register a function (for backward compatibility)
    pub fn register_function<F>(&mut self, name: &str, function: F)
    where
        F: tera::Function + 'static,
    {
        match self {
            Self::Static(tera) => {
                if let Some(tera_mut) = Arc::get_mut(tera) {
                    tera_mut.register_function(name, function);
                }
            }
            Self::Reloadable { cached, .. } => {
                if let Ok(mut write_guard) = cached.write() {
                    write_guard.register_function(name, function);
                }
            }
        }
    }
}

// Implement Clone manually since we need to handle the Arc properly
impl Clone for TemplateEngine {
    fn clone(&self) -> Self {
        match self {
            Self::Static(tera) => Self::Static(Arc::clone(tera)),
            Self::Reloadable {
                templates_dir,
                cached,
            } => Self::Reloadable {
                templates_dir: templates_dir.clone(),
                cached: Arc::clone(cached),
            },
        }
    }
}

/// Create a round filter for Tera
fn make_round_filter() -> impl Filter {
    RoundFilter
}

struct RoundFilter;

impl Filter for RoundFilter {
    fn filter(&self, value: &Value, args: &HashMap<String, Value>) -> tera::Result<Value> {
        let num = match value {
            Value::Number(n) => n
                .as_f64()
                .ok_or_else(|| tera::Error::msg("Failed to convert number to float"))?,
            _ => return Err(tera::Error::msg("round filter only works on numbers")),
        };

        let precision = args
            .get("precision")
            .or_else(|| args.values().next()) // Support positional argument
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as i32;

        let multiplier = 10f64.powi(precision);
        let rounded = (num * multiplier).round() / multiplier;

        Ok(to_value(rounded)?)
    }
}
