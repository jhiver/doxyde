use anyhow::{anyhow, Context, Result};
use sqlx::SqlitePool;
use std::path::Path;

/// Initialize the database, creating the file if needed and running migrations
pub async fn init_database(database_url: &str) -> Result<SqlitePool> {
    // Create database file if it doesn't exist
    if database_url.starts_with("sqlite:") {
        let path = database_url.trim_start_matches("sqlite:");
        if !path.starts_with(":memory:") {
            if let Some(parent) = Path::new(path).parent() {
                std::fs::create_dir_all(parent).context("Failed to create database directory")?;
            }
        }
    }

    // Create connection pool
    let pool = SqlitePool::connect(database_url)
        .await
        .context("Failed to connect to database")?;

    // Run migrations
    check_and_run_migrations(&pool).await?;

    Ok(pool)
}

/// Run database migrations, handling cases where schema already exists
async fn check_and_run_migrations(pool: &SqlitePool) -> Result<()> {
    tracing::info!("Checking for pending migrations...");
    
    // First ensure the migrations table exists
    sqlx::query!(
        r#"
        CREATE TABLE IF NOT EXISTS _sqlx_migrations (
            version BIGINT PRIMARY KEY,
            description TEXT NOT NULL,
            installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            success BOOLEAN NOT NULL,
            checksum BLOB NOT NULL,
            execution_time BIGINT NOT NULL
        )
        "#
    )
    .execute(pool)
    .await
    .context("Failed to create migrations table")?;
    
    let migrator = sqlx::migrate!("../migrations");
    
    // Iterate through each migration
    for migration in migrator.migrations.iter() {
        // Check if this migration has already been applied
        let applied = sqlx::query!(
            "SELECT version FROM _sqlx_migrations WHERE version = ?",
            migration.version
        )
        .fetch_optional(pool)
        .await
        .unwrap_or(None)
        .is_some();
        
        if !applied {
            tracing::info!("Running migration {}: {}", migration.version, migration.description);
            
            // Try to run this specific migration
            match sqlx::query(&migration.sql)
                .execute(pool)
                .await
            {
                Ok(_) => {
                    // Record successful migration
                    let checksum_bytes: &[u8] = &migration.checksum;
                    sqlx::query!(
                        "INSERT INTO _sqlx_migrations (version, description, installed_on, success, checksum, execution_time) 
                         VALUES (?, ?, datetime('now'), 1, ?, ?)",
                        migration.version,
                        migration.description,
                        checksum_bytes,
                        1000000i64 // 1ms in nanoseconds
                    )
                    .execute(pool)
                    .await
                    .context("Failed to record migration")?;
                    
                    tracing::info!("Migration {} applied successfully", migration.version);
                },
                Err(e) => {
                    let error_str = e.to_string();
                    
                    // Check if migration was already applied based on the error
                    let already_applied = match migration.version {
                        // For 20250712: check if is_published column exists
                        20250712 if error_str.contains("duplicate column name: is_published") => {
                            tracing::info!("Checking if migration 20250712 was already applied...");
                            true
                        },
                        // For 20250713: check if style_options column exists
                        20250713 if error_str.contains("duplicate column name: style_options") => {
                            tracing::info!("Checking if migration 20250713 was already applied...");
                            true
                        },
                        // For 20250714: check if components table exists without style_options
                        20250714 if error_str.contains("components_new already exists") => {
                            tracing::info!("Checking if migration 20250714 was already applied...");
                            // Check if the components table already lacks style_options column
                            let has_style_options = sqlx::query!("PRAGMA table_info(components)")
                                .fetch_all(pool)
                                .await
                                .map(|rows| rows.iter().any(|r| r.name == "style_options"))
                                .unwrap_or(false);
                            !has_style_options // If no style_options, migration was already applied
                        },
                        // For 20250715: check if mcp_tokens table exists
                        20250715 if error_str.contains("already exists") => {
                            tracing::info!("Checking if migration 20250715 was already applied...");
                            sqlx::query!("SELECT name FROM sqlite_master WHERE type='table' AND name='mcp_tokens'")
                                .fetch_optional(pool)
                                .await
                                .map(|r| r.is_some())
                                .unwrap_or(false)
                        },
                        // For 20250719: check if sort_mode column exists
                        20250719 if error_str.contains("duplicate column name: sort_mode") => {
                            tracing::info!("Checking if migration 20250719 was already applied...");
                            true
                        },
                        // For any other error or migration, it's a real failure
                        _ => false
                    };
                    
                    if already_applied {
                        tracing::warn!("Migration {} was already applied, marking as complete", migration.version);
                        
                        // Record it as applied
                        let checksum_bytes: &[u8] = &migration.checksum;
                        sqlx::query!(
                            "INSERT INTO _sqlx_migrations (version, description, installed_on, success, checksum, execution_time) 
                             VALUES (?, ?, datetime('now'), 1, ?, ?)",
                            migration.version,
                            migration.description,
                            checksum_bytes,
                            1000000i64
                        )
                        .execute(pool)
                        .await
                        .context("Failed to record migration")?;
                    } else {
                        return Err(anyhow!("Failed to run migration {}: {}", migration.version, e));
                    }
                }
            }
        }
    }
    
    tracing::info!("All migrations processed successfully");
    
    Ok(())
}