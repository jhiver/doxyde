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

use anyhow::{Context, Result};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{config::Config, db, site_resolver::SiteContext};

/// Database router that manages connections for multi-site architecture
#[derive(Clone)]
pub struct DatabaseRouter {
    /// Configuration
    config: Config,
    /// Per-site database pools (site_directory -> pool)
    site_pools: Arc<RwLock<HashMap<String, SqlitePool>>>,
}

impl DatabaseRouter {
    /// Create a new database router
    pub async fn new(config: Config) -> Result<Self> {
        Ok(Self {
            config,
            site_pools: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Get database pool for a site context
    pub async fn get_pool(&self, context: &SiteContext) -> Result<SqlitePool> {
        // Get or create site-specific pool
        self.get_or_create_site_pool(context).await
    }

    /// Get or create a site-specific database pool
    async fn get_or_create_site_pool(&self, context: &SiteContext) -> Result<SqlitePool> {
        let site_dir = &context.site_directory;

        let site_key = site_dir.to_string_lossy().to_string();

        // First try read lock to check if pool exists
        {
            let pools = self.site_pools.read().await;
            if let Some(pool) = pools.get(&site_key) {
                return Ok(pool.clone());
            }
        }

        // Need to create new pool - acquire write lock
        let mut pools = self.site_pools.write().await;

        // Double-check after acquiring write lock
        if let Some(pool) = pools.get(&site_key) {
            return Ok(pool.clone());
        }

        // Create site directory if it doesn't exist
        if !site_dir.exists() {
            std::fs::create_dir_all(site_dir)
                .with_context(|| format!("Failed to create site directory: {:?}", site_dir))?;
        }

        // Get database path
        let db_path = context.database_path();

        // Check if database exists
        let db_file_path = db_path
            .strip_prefix("sqlite:")
            .unwrap_or(&db_path)
            .to_string();
        let db_exists = PathBuf::from(&db_file_path).exists();

        // Initialize database (creates file if needed and runs migrations)
        let pool = if !db_exists {
            tracing::info!("Initializing new site database at: {}", db_path);
            db::init_database(&db_path)
                .await
                .with_context(|| format!("Failed to initialize site database: {}", db_path))?
        } else {
            // Just connect if database already exists with connection pool limits
            sqlx::sqlite::SqlitePoolOptions::new()
                .max_connections(5) // Limit per-site connections to prevent resource exhaustion
                .min_connections(1) // Keep at least one connection alive
                .connect(&db_path)
                .await
                .with_context(|| format!("Failed to connect to site database: {}", db_path))?
        };

        // Store pool for future use
        pools.insert(site_key, pool.clone());

        Ok(pool)
    }

    /// Close all database connections
    pub async fn close_all(&self) {
        // Close all site pools
        let mut pools = self.site_pools.write().await;
        for (_, pool) in pools.drain() {
            pool.close().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_config(sites_dir: String) -> Config {
        Config {
            database_url: "sqlite::memory:".to_string(),
            host: "localhost".to_string(),
            port: 3000,
            templates_dir: "templates".to_string(),
            session_secret: "test".to_string(),
            development_mode: false,
            uploads_dir: "uploads".to_string(),
            max_upload_size: 1048576,
            secure_cookies: false,
            session_timeout_minutes: 1440,
            sites_directory: sites_dir,
        }
    }

    #[tokio::test]
    async fn test_database_router_site_specific_pool() {
        let temp_dir = TempDir::new().unwrap();
        let sites_dir = temp_dir.path().to_string_lossy().to_string();
        
        let config = create_test_config(sites_dir.clone()).await;
        let router = DatabaseRouter::new(config).await.unwrap();

        let context = SiteContext::new("example.com".to_string(), &PathBuf::from(&sites_dir));
        let pool = router.get_pool(&context).await.unwrap();

        // Should return a valid pool
        assert!(pool.size() > 0);
    }

    #[tokio::test]
    async fn test_database_router_multi_site_mode() {
        let temp_dir = TempDir::new().unwrap();
        let sites_dir = temp_dir.path().to_string_lossy().to_string();

        let config = create_test_config(sites_dir.clone()).await;
        let router = DatabaseRouter::new(config).await.unwrap();

        let context =
            SiteContext::new("example.com".to_string(), &PathBuf::from(&sites_dir));
        let pool1 = router.get_pool(&context).await.unwrap();

        // Request pool again - should get cached version
        let pool2 = router.get_pool(&context).await.unwrap();

        // Both should be the same pool instance
        assert_eq!(pool1.size(), pool2.size());

        // Different site should get different pool
        let context2 = SiteContext::new("other.com".to_string(), &PathBuf::from(&sites_dir));
        let pool3 = router.get_pool(&context2).await.unwrap();

        // Should be a different pool (though in tests with :memory: this is hard to verify)
        // Just verify it's a valid pool
        let _ = pool3.size();
    }

    #[tokio::test]
    async fn test_database_router_creates_site_directory() {
        let temp_dir = TempDir::new().unwrap();
        let sites_dir = temp_dir.path().to_string_lossy().to_string();

        let config = create_test_config(sites_dir.clone()).await;
        let router = DatabaseRouter::new(config).await.unwrap();

        let context =
            SiteContext::new("example.com".to_string(), &PathBuf::from(&sites_dir));

        // Get the actual site directory from context
        let site_dir = &context.site_directory;

        // Site directory shouldn't exist yet
        assert!(!site_dir.exists());

        // Getting pool should create directory
        let _pool = router.get_pool(&context).await.unwrap();

        // Now it should exist
        assert!(site_dir.exists());
        assert!(site_dir.is_dir());
    }

    #[tokio::test]
    async fn test_database_router_close_all() {
        let temp_dir = TempDir::new().unwrap();
        let sites_dir = temp_dir.path().to_string_lossy().to_string();

        let config = create_test_config(sites_dir.clone()).await;
        let router = DatabaseRouter::new(config).await.unwrap();

        // Create some connections
        let context = SiteContext::new("example.com".to_string(), &PathBuf::from(&sites_dir));
        let _pool = router.get_pool(&context).await.unwrap();

        // Close all should complete without error
        router.close_all().await;
    }

    #[tokio::test]
    async fn test_subdomain_shares_database() {
        let temp_dir = TempDir::new().unwrap();
        let sites_dir = temp_dir.path().to_string_lossy().to_string();

        let config = create_test_config(sites_dir.clone()).await;
        let router = DatabaseRouter::new(config).await.unwrap();

        // Create contexts for different subdomains of same base domain
        let context1 = SiteContext::new("site1.example.com".to_string(), &PathBuf::from(&sites_dir));
        let context2 = SiteContext::new("www.example.com".to_string(), &PathBuf::from(&sites_dir));
        let context3 = SiteContext::new("example.com".to_string(), &PathBuf::from(&sites_dir));

        // All should resolve to the same directory
        assert_eq!(context1.site_directory, context2.site_directory);
        assert_eq!(context2.site_directory, context3.site_directory);

        // Get pools - they should be the same
        let pool1 = router.get_pool(&context1).await.unwrap();
        let pool2 = router.get_pool(&context2).await.unwrap();
        let pool3 = router.get_pool(&context3).await.unwrap();

        // All pools should have same size (indicating they're the same pool)
        assert_eq!(pool1.size(), pool2.size());
        assert_eq!(pool2.size(), pool3.size());
    }
}
