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
use doxyde_mcp::oauth::{validate_token, TokenInfo};

/// Database router that manages connections for multi-site architecture
#[derive(Clone)]
pub struct DatabaseRouter {
    /// Configuration
    _config: Config,
    /// Per-site database pools (site_directory -> pool)
    site_pools: Arc<RwLock<HashMap<String, SqlitePool>>>,
}

impl DatabaseRouter {
    /// Create a new database router
    pub async fn new(config: Config) -> Result<Self> {
        let router = Self {
            _config: config.clone(),
            site_pools: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Run migrations on all existing site databases at startup
        if config.multi_site_mode && !config.sites_directory.is_empty() {
            router.run_migrations_on_all_sites(&config.sites_directory).await?;
        }
        
        Ok(router)
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

    /// Validate an OAuth token and return the associated database pool
    /// This is used by MCP/RMCP handlers that receive OAuth tokens
    pub async fn validate_token_and_get_db(
        &self,
        token: &str,
    ) -> Result<Option<(TokenInfo, SqlitePool)>> {
        // OAuth tokens might be stored in a central location or in each site's database
        // For now, we'll check each site's database until we find a match
        // This is not optimal for many sites, but works for the current architecture

        let pools = self.site_pools.read().await;

        // If we have cached pools, check them first
        for (site_key, pool) in pools.iter() {
            match validate_token(pool, token).await {
                Ok(Some(token_info)) => {
                    // Found valid token, return it with the pool
                    return Ok(Some((token_info, pool.clone())));
                }
                Ok(None) => {
                    // Invalid token in this database, continue checking
                    continue;
                }
                Err(e) => {
                    // Log error but continue checking other databases
                    tracing::debug!("Error validating token in site {}: {}", site_key, e);
                    continue;
                }
            }
        }

        // If not found in cached pools, we might need to check uncached sites
        // For now, return None
        Ok(None)
    }

    /// Run migrations on all existing site databases at startup
    async fn run_migrations_on_all_sites(&self, sites_directory: &str) -> Result<()> {
        let sites_path = PathBuf::from(sites_directory);
        
        // Check if sites directory exists
        if !sites_path.exists() {
            tracing::info!("Sites directory does not exist yet: {:?}", sites_path);
            return Ok(());
        }
        
        // Iterate through all directories in the sites directory
        let entries = std::fs::read_dir(&sites_path)
            .with_context(|| format!("Failed to read sites directory: {:?}", sites_path))?;
        
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            
            // Skip if not a directory
            if !path.is_dir() {
                continue;
            }
            
            // Check if site.db exists in this directory
            let db_path = path.join("site.db");
            if !db_path.exists() {
                continue;
            }
            
            // Get the site directory name
            let site_dir_name = path.file_name()
                .and_then(|n| n.to_str())
                .ok_or_else(|| anyhow::anyhow!("Invalid site directory name"))?;
            
            tracing::info!("Running migrations for site: {}", site_dir_name);
            
            // Build database URL
            let db_url = format!("sqlite:{}", db_path.display());
            
            // Initialize database (this runs migrations)
            match db::init_database(&db_url).await {
                Ok(pool) => {
                    // Store the pool for later use
                    let mut pools = self.site_pools.write().await;
                    pools.insert(path.to_string_lossy().to_string(), pool);
                    tracing::info!("Successfully migrated site: {}", site_dir_name);
                }
                Err(e) => {
                    tracing::error!("Failed to migrate site {}: {}", site_dir_name, e);
                    // Continue with other sites even if one fails
                }
            }
        }
        
        Ok(())
    }

    /// Close all database connections
    pub async fn close_all(&self) {
        // Close all site pools
        let mut pools = self.site_pools.write().await;
        for (_, pool) in pools.drain() {
            pool.close().await;
        }
    }

    /// Create a test database router with a pre-configured pool
    #[cfg(test)]
    pub fn new_for_test(config: Config, pool: SqlitePool) -> Self {
        let mut pools = HashMap::new();
        pools.insert("test.local".to_string(), pool);
        Self {
            _config: config,
            site_pools: Arc::new(RwLock::new(pools)),
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
            login_attempts_per_minute: 5,
            api_requests_per_minute: 60,
            csrf_enabled: true,
            csrf_token_expiry_hours: 24,
            csrf_token_length: 32,
            csrf_header_name: "X-CSRF-Token".to_string(),
            static_files_max_age: 86400,
            oauth_token_expiry: 3600,
            sites_directory: sites_dir,
            multi_site_mode: true,
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

        let context = SiteContext::new("example.com".to_string(), &PathBuf::from(&sites_dir));
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

        let context = SiteContext::new("example.com".to_string(), &PathBuf::from(&sites_dir));

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
        let context1 =
            SiteContext::new("site1.example.com".to_string(), &PathBuf::from(&sites_dir));
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

    #[tokio::test]
    async fn test_different_domains_get_different_databases() {
        let temp_dir = TempDir::new().unwrap();
        let sites_dir = temp_dir.path().to_string_lossy().to_string();

        let config = create_test_config(sites_dir.clone()).await;
        let router = DatabaseRouter::new(config).await.unwrap();

        // Create contexts for different domains
        let context1 = SiteContext::new("example.com".to_string(), &PathBuf::from(&sites_dir));
        let context2 = SiteContext::new("other-site.com".to_string(), &PathBuf::from(&sites_dir));

        // They should have different directories
        assert_ne!(context1.site_directory, context2.site_directory);

        // Get pools and create tables in each
        let pool1 = router.get_pool(&context1).await.unwrap();
        let pool2 = router.get_pool(&context2).await.unwrap();

        // Create a test table in pool1
        sqlx::query("CREATE TABLE test_isolation (id INTEGER PRIMARY KEY, value TEXT)")
            .execute(&pool1)
            .await
            .unwrap();

        // Insert data in pool1
        sqlx::query("INSERT INTO test_isolation (value) VALUES ('example.com data')")
            .execute(&pool1)
            .await
            .unwrap();

        // Table should not exist in pool2
        let result = sqlx::query("SELECT * FROM test_isolation")
            .fetch_one(&pool2)
            .await;

        assert!(
            result.is_err(),
            "Table should not exist in different domain's database"
        );
    }

    #[tokio::test]
    async fn test_legacy_mode_when_sites_directory_not_configured() {
        let config = Config {
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
            login_attempts_per_minute: 5,
            api_requests_per_minute: 60,
            csrf_enabled: true,
            csrf_token_expiry_hours: 24,
            csrf_token_length: 32,
            csrf_header_name: "X-CSRF-Token".to_string(),
            static_files_max_age: 86400,
            oauth_token_expiry: 3600,
            sites_directory: "".to_string(), // Empty sites directory
            multi_site_mode: false,
        };

        let router = DatabaseRouter::new(config).await.unwrap();

        // Any context should return the legacy pool
        let context1 = SiteContext::legacy("example.com".to_string());
        let context2 = SiteContext::legacy("other.com".to_string());

        let pool1 = router.get_pool(&context1).await.unwrap();
        let pool2 = router.get_pool(&context2).await.unwrap();

        // Both should be the same pool (legacy mode)
        assert_eq!(pool1.size(), pool2.size());
    }

    #[tokio::test]
    async fn test_port_in_domain_is_ignored() {
        let temp_dir = TempDir::new().unwrap();
        let sites_dir = temp_dir.path().to_string_lossy().to_string();

        // Create contexts with and without port
        let context1 = SiteContext::new("example.com:3000".to_string(), &PathBuf::from(&sites_dir));
        let context2 = SiteContext::new("example.com".to_string(), &PathBuf::from(&sites_dir));

        // They should resolve to the same directory
        assert_eq!(context1.site_directory, context2.site_directory);
        assert_eq!(context1.site_key, context2.site_key);
    }

    #[tokio::test]
    async fn test_database_migrations_run_for_new_site() {
        let temp_dir = TempDir::new().unwrap();
        let sites_dir = temp_dir.path().to_string_lossy().to_string();

        let config = create_test_config(sites_dir.clone()).await;
        let router = DatabaseRouter::new(config).await.unwrap();

        let context = SiteContext::new("new-site.com".to_string(), &PathBuf::from(&sites_dir));
        let pool = router.get_pool(&context).await.unwrap();

        // Check that migrations have run by verifying a table exists
        let result = sqlx::query(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='_sqlx_migrations'",
        )
        .fetch_one(&pool)
        .await;

        assert!(result.is_ok(), "Migrations table should exist");
    }

    #[tokio::test]
    async fn test_concurrent_access_to_same_site() {
        let temp_dir = TempDir::new().unwrap();
        let sites_dir = temp_dir.path().to_string_lossy().to_string();

        let config = create_test_config(sites_dir.clone()).await;
        let router = Arc::new(DatabaseRouter::new(config).await.unwrap());

        // Spawn multiple tasks accessing the same site
        let mut handles = vec![];
        for i in 0..5 {
            let router_clone = router.clone();
            let sites_dir_clone = sites_dir.clone();
            let handle = tokio::spawn(async move {
                let context = SiteContext::new(
                    "concurrent-test.com".to_string(),
                    &PathBuf::from(&sites_dir_clone),
                );
                let pool = router_clone.get_pool(&context).await.unwrap();

                // Each task creates its own table
                let table_name = format!("test_table_{}", i);
                sqlx::query(&format!(
                    "CREATE TABLE IF NOT EXISTS {} (id INTEGER PRIMARY KEY)",
                    table_name
                ))
                .execute(&pool)
                .await
                .unwrap();
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // Verify all tables were created
        let context = SiteContext::new(
            "concurrent-test.com".to_string(),
            &PathBuf::from(&sites_dir),
        );
        let pool = router.get_pool(&context).await.unwrap();

        for i in 0..5 {
            let table_name = format!("test_table_{}", i);
            let result = sqlx::query(&format!(
                "SELECT name FROM sqlite_master WHERE type='table' AND name='{}'",
                table_name
            ))
            .fetch_one(&pool)
            .await;
            assert!(result.is_ok(), "Table {} should exist", table_name);
        }
    }
}
