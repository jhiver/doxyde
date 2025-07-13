use crate::autoreload_templates::TemplateEngine;
use crate::config::Config;
use axum::extract::FromRef;
use sqlx::SqlitePool;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub templates: TemplateEngine,
    pub config: Config,
}

impl AppState {
    pub fn new(db: SqlitePool, templates: TemplateEngine, config: Config) -> Self {
        Self {
            db,
            templates,
            config,
        }
    }
}

impl FromRef<AppState> for SqlitePool {
    fn from_ref(state: &AppState) -> Self {
        state.db.clone()
    }
}
