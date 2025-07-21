use anyhow::{Context, Result};
use sqlx::SqlitePool;
use std::path::Path;

/// Initialize the database, creating the file if needed and running migrations
pub async fn init_database(database_url: &str) -> Result<SqlitePool> {
    // Create database file if it doesn't exist
    if database_url.starts_with("sqlite:") {
        let path = database_url.trim_start_matches("sqlite:");
        if !path.starts_with(":memory:") {
            let db_path = Path::new(path);
            if let Some(parent) = db_path.parent() {
                if !parent.as_os_str().is_empty() {
                    std::fs::create_dir_all(parent).context("Failed to create database directory")?;
                }
            }
            // Touch the file to ensure it exists
            if !db_path.exists() {
                std::fs::File::create(db_path).context("Failed to create database file")?;
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

/// Run database migrations with proper error handling for already-applied migrations
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
            tracing::info!(
                "Running migration {}: {}",
                migration.version,
                migration.description
            );

            // Try to run this specific migration
            match sqlx::query(&migration.sql).execute(pool).await {
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
                }
                Err(e) => {
                    let error_str = e.to_string();
                    
                    // Check if the error indicates the migration was already applied
                    let already_applied = 
                        error_str.contains("already exists") ||
                        error_str.contains("duplicate column name") ||
                        error_str.contains("no such column"); // For DROP COLUMN on non-existent column
                    
                    if already_applied {
                        tracing::warn!(
                            "Migration {} appears to have been already applied: {}",
                            migration.version,
                            error_str
                        );
                        
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
                        
                        tracing::info!("Migration {} marked as applied", migration.version);
                    } else {
                        return Err(anyhow::anyhow!(
                            "Failed to run migration {}: {}",
                            migration.version,
                            e
                        ));
                    }
                }
            }
        }
    }

    tracing::info!("All migrations processed successfully");

    Ok(())
}
