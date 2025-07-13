use anyhow::{Context, Result};
use std::{env, path::PathBuf};
use uuid;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub host: String,
    pub port: u16,
    pub templates_dir: String,
    pub session_secret: String,
    pub development_mode: bool,
    pub uploads_dir: String,
    pub max_upload_size: usize,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        // Find project root by looking for workspace Cargo.toml
        let project_root = Self::find_project_root()?;

        // Default templates directory relative to project root
        let default_templates_dir = project_root.join("templates").to_string_lossy().to_string();

        // Default uploads directory
        let default_uploads_dir = env::var("HOME")
            .map(|home| PathBuf::from(home).join(".doxyde").join("uploads"))
            .unwrap_or_else(|_| PathBuf::from("/var/doxyde/uploads"))
            .to_string_lossy()
            .to_string();

        Ok(Self {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:doxyde.db".to_string()),
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .context("Invalid PORT")?,
            templates_dir: env::var("TEMPLATES_DIR").unwrap_or(default_templates_dir),
            session_secret: env::var("SESSION_SECRET").unwrap_or_else(|_| {
                // Generate a random secret for development
                uuid::Uuid::new_v4().to_string()
            }),
            development_mode: env::var("DEVELOPMENT_MODE")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
            uploads_dir: env::var("UPLOADS_DIR").unwrap_or(default_uploads_dir),
            max_upload_size: env::var("MAX_UPLOAD_SIZE")
                .unwrap_or_else(|_| "10485760".to_string()) // 10MB default
                .parse()
                .unwrap_or(10_485_760),
        })
    }

    /// Find the project root by looking for the workspace Cargo.toml
    fn find_project_root() -> Result<PathBuf> {
        let mut current_dir = env::current_dir()?;

        loop {
            let cargo_toml = current_dir.join("Cargo.toml");
            if cargo_toml.exists() {
                // Check if this is the workspace root
                let content = std::fs::read_to_string(&cargo_toml)?;
                if content.contains("[workspace]") {
                    return Ok(current_dir);
                }
            }

            // Move up one directory
            if !current_dir.pop() {
                // We've reached the root directory
                break;
            }
        }

        // If we can't find the workspace root, use current directory
        env::current_dir().context("Failed to determine project root")
    }

    pub fn bind_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}
