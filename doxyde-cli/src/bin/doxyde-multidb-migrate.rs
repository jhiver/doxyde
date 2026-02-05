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
use clap::{Parser, Subcommand};
use doxyde_web::domain_utils::extract_base_domain;
use sha2::{Digest, Sha256};
use sqlx::types::chrono;
use sqlx::{Row, SqlitePool};
use std::fs;
use std::path::Path;

#[derive(Parser)]
#[command(author, version, about = "Doxyde database migration tool", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Migrate from single database to multi-database mode
    ToMultiDb {
        /// Path to the single database file
        #[arg(short, long)]
        source: String,

        /// Directory where site-specific databases will be created
        #[arg(short, long)]
        target_dir: String,

        /// Dry run - show what would be done without making changes
        #[arg(short = 'n', long)]
        dry_run: bool,
    },

    /// Show information about sites in a database
    Info {
        /// Path to the database file
        #[arg(short, long)]
        database: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::ToMultiDb {
            source,
            target_dir,
            dry_run,
        } => {
            migrate_to_multi_db(&source, &target_dir, dry_run).await?;
        }
        Commands::Info { database } => {
            show_database_info(&database).await?;
        }
    }

    Ok(())
}

async fn show_database_info(database_path: &str) -> Result<()> {
    let pool = SqlitePool::connect(&format!("sqlite:{}", database_path))
        .await
        .context("Failed to connect to database")?;

    println!("Database: {}", database_path);
    println!();

    // Check if this is an old-style database (has sites table) or new-style (has site_config)
    let has_sites_table: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='sites')",
    )
    .fetch_one(&pool)
    .await?;

    if has_sites_table {
        // Old-style database with sites table
        let sites = sqlx::query("SELECT id, domain, title FROM sites ORDER BY domain")
            .fetch_all(&pool)
            .await
            .context("Failed to query sites")?;

        println!("Sites found: {}", sites.len());
        println!();

        for site in sites {
            let site_id: i64 = site.get("id");
            let domain: String = site.get("domain");
            let title: String = site.get("title");

            // Count pages for this site
            let page_count: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM pages WHERE site_id = ?")
                    .bind(site_id)
                    .fetch_one(&pool)
                    .await?;

            // Count users for this site
            let user_count: i64 = sqlx::query_scalar(
                "SELECT COUNT(DISTINCT user_id) FROM site_users WHERE site_id = ?",
            )
            .bind(site_id)
            .fetch_one(&pool)
            .await?;

            println!("Site: {} ({})", title, domain);
            println!("  ID: {}", site_id);
            println!("  Pages: {}", page_count);
            println!("  Users: {}", user_count);
            println!();
        }
    } else {
        // New-style database with site_config
        let config = sqlx::query("SELECT title FROM site_config WHERE id = 1")
            .fetch_optional(&pool)
            .await?;

        match config {
            Some(row) => {
                let title: String = row.get("title");
                let page_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pages")
                    .fetch_one(&pool)
                    .await?;
                let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
                    .fetch_one(&pool)
                    .await?;

                println!("Site: {} (multi-db format)", title);
                println!("  Pages: {}", page_count);
                println!("  Users: {}", user_count);
            }
            None => {
                println!("No site configuration found");
            }
        }
    }

    Ok(())
}

async fn migrate_to_multi_db(source_path: &str, target_dir: &str, dry_run: bool) -> Result<()> {
    // Verify source exists
    if !Path::new(source_path).exists() {
        anyhow::bail!("Source database file not found: {}", source_path);
    }

    // Create target directory if it doesn't exist
    let target_path = Path::new(target_dir);
    if !dry_run && !target_path.exists() {
        fs::create_dir_all(target_path).context("Failed to create target directory")?;
    }

    println!("Migration from single to multi-database mode");
    println!("Source: {}", source_path);
    println!("Target directory: {}", target_dir);
    if dry_run {
        println!("DRY RUN - No changes will be made");
    }
    println!();

    // Connect to source database to get site info
    let source_pool = SqlitePool::connect(&format!("sqlite:{}", source_path))
        .await
        .context("Failed to connect to source database")?;

    // Get all sites
    let sites = sqlx::query("SELECT id, domain, title FROM sites ORDER BY domain")
        .fetch_all(&source_pool)
        .await
        .context("Failed to query sites")?;

    println!("Found {} sites to migrate", sites.len());
    println!();

    // Close source pool before copying
    source_pool.close().await;

    // Process each site
    for site in sites {
        let site_id: i64 = site.get("id");
        let domain: String = site.get("domain");
        let title: String = site.get("title");

        // Calculate site directory (matching domain_utils::resolve_site_directory)
        let base_domain = extract_base_domain(&domain);
        // Sanitize domain for filesystem (replace . and : with -)
        let sanitized_domain = base_domain.replace(['.', ':'], "-");
        let site_key = calculate_site_key(&base_domain);
        let site_dir = target_path.join(format!("{}-{}", sanitized_domain, site_key));

        println!("Processing site: {} ({}) [ID: {}]", title, domain, site_id);
        println!("  Target directory: {}", site_dir.display());

        if dry_run {
            println!("  [DRY RUN] Would create site database");
        } else {
            // Create site directory
            if !site_dir.exists() {
                fs::create_dir_all(&site_dir).context("Failed to create site directory")?;
            }

            // Copy the source database to target
            let site_db_path = site_dir.join("site.db");
            println!("  Copying database...");
            fs::copy(source_path, &site_db_path).context("Failed to copy source database")?;

            // Connect to copied database
            let site_db_url = format!("sqlite:{}", site_db_path.display());
            let target_pool = SqlitePool::connect(&site_db_url)
                .await
                .context("Failed to connect to target database")?;

            // Transform the database for multi-db architecture
            println!("  Transforming schema for multi-database mode...");
            transform_to_multidb(&target_pool, site_id, &title).await?;

            // Close target pool
            target_pool.close().await;

            println!("  ✓ Migration complete");
        }

        println!();
    }

    println!("Migration completed successfully!");

    if !dry_run {
        println!();
        println!("IMPORTANT: Next steps:");
        println!("1. Update your configuration to set:");
        println!("   sites_directory = \"{}\"", target_dir);
        println!("   multi_site_mode = true");
        println!("2. Restart Doxyde");
        println!("3. Verify everything works correctly");
        println!("4. Keep the original database as backup: {}", source_path);
    }

    Ok(())
}

fn calculate_site_key(base_domain: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(base_domain.as_bytes());
    let hash = hasher.finalize();
    hex::encode(&hash[..4])
}

/// Transform a database from single-site to multi-site format
/// This keeps only data for the specified site_id and restructures the schema
async fn transform_to_multidb(pool: &SqlitePool, site_id: i64, title: &str) -> Result<()> {
    // CRITICAL: Disable foreign keys to prevent CASCADE deletes
    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(pool)
        .await?;

    // Start transaction
    let mut tx = pool.begin().await?;

    // Step 1: Create site_config table if it doesn't exist
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS site_config (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            title TEXT NOT NULL,
            description TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(&mut *tx)
    .await?;

    // Step 2: Get site data and populate site_config
    let site_data = sqlx::query("SELECT created_at, updated_at FROM sites WHERE id = ?")
        .bind(site_id)
        .fetch_one(&mut *tx)
        .await?;

    let created_at: String = site_data.get("created_at");
    let updated_at: String = site_data.get("updated_at");

    sqlx::query(
        "INSERT OR REPLACE INTO site_config (id, title, created_at, updated_at)
         VALUES (1, ?, ?, ?)",
    )
    .bind(title)
    .bind(&created_at)
    .bind(&updated_at)
    .execute(&mut *tx)
    .await?;

    // Step 3: Delete pages not belonging to this site
    sqlx::query("DELETE FROM pages WHERE site_id != ?")
        .bind(site_id)
        .execute(&mut *tx)
        .await?;

    // Step 4: Delete site_users not belonging to this site
    sqlx::query("DELETE FROM site_users WHERE site_id != ?")
        .bind(site_id)
        .execute(&mut *tx)
        .await?;

    // Step 5: Delete orphaned users (users not in site_users)
    sqlx::query(
        "DELETE FROM users WHERE id NOT IN (SELECT DISTINCT user_id FROM site_users WHERE site_id = ?)",
    )
    .bind(site_id)
    .execute(&mut *tx)
    .await?;

    // Step 6: Clean up MCP tokens (keep only for this site)
    let has_site_id_in_mcp: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM pragma_table_info('mcp_tokens') WHERE name='site_id')",
    )
    .fetch_one(&mut *tx)
    .await?;

    if has_site_id_in_mcp {
        sqlx::query("DELETE FROM mcp_tokens WHERE site_id != ?")
            .bind(site_id)
            .execute(&mut *tx)
            .await?;
    }

    // Step 7: Drop other sites from sites table (before we drop it)
    sqlx::query("DELETE FROM sites WHERE id != ?")
        .bind(site_id)
        .execute(&mut *tx)
        .await?;

    // Commit this phase
    tx.commit().await?;

    // Step 8: Now rebuild tables without site_id columns
    // We need to do this carefully to preserve data

    // Rebuild pages table without site_id
    let mut tx = pool.begin().await?;

    // Create new pages table without site_id
    sqlx::query(
        "CREATE TABLE pages_new (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            parent_page_id INTEGER,
            slug TEXT NOT NULL,
            title TEXT NOT NULL,
            position INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            description TEXT,
            keywords TEXT,
            template TEXT DEFAULT 'default',
            meta_robots TEXT NOT NULL DEFAULT 'index,follow',
            canonical_url TEXT,
            og_image_url TEXT,
            structured_data_type TEXT NOT NULL DEFAULT 'WebPage',
            sort_mode TEXT NOT NULL DEFAULT 'created_at_asc',
            FOREIGN KEY (parent_page_id) REFERENCES pages_new(id) ON DELETE CASCADE,
            UNIQUE(parent_page_id, slug)
        )",
    )
    .execute(&mut *tx)
    .await?;

    // Copy data to new table
    sqlx::query(
        "INSERT INTO pages_new (id, parent_page_id, slug, title, position, created_at, updated_at,
         description, keywords, template, meta_robots, canonical_url, og_image_url,
         structured_data_type, sort_mode)
         SELECT id, parent_page_id, slug, title, position, created_at, updated_at,
         description, keywords, template, meta_robots, canonical_url, og_image_url,
         structured_data_type, sort_mode
         FROM pages",
    )
    .execute(&mut *tx)
    .await?;

    // Drop old table and rename new one
    sqlx::query("DROP TABLE pages").execute(&mut *tx).await?;
    sqlx::query("ALTER TABLE pages_new RENAME TO pages")
        .execute(&mut *tx)
        .await?;

    // Recreate indexes
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_pages_parent_page_id ON pages(parent_page_id)")
        .execute(&mut *tx)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_pages_template ON pages(template)")
        .execute(&mut *tx)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_pages_sort_mode ON pages(sort_mode)")
        .execute(&mut *tx)
        .await?;

    // Rebuild site_users table without site_id
    // Note: original table has composite primary key (site_id, user_id), no id column
    sqlx::query(
        "CREATE TABLE site_users_new (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL UNIQUE,
            role TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO site_users_new (user_id, role, created_at)
         SELECT user_id, role, created_at FROM site_users",
    )
    .execute(&mut *tx)
    .await?;

    sqlx::query("DROP TABLE site_users")
        .execute(&mut *tx)
        .await?;
    sqlx::query("ALTER TABLE site_users_new RENAME TO site_users")
        .execute(&mut *tx)
        .await?;

    // Rebuild mcp_tokens without site_id if it exists
    if has_site_id_in_mcp {
        // Create new mcp_tokens table without site_id
        // Based on production schema: id, token_hash, name, scopes, created_by, expires_at, created_at, last_used_at
        sqlx::query(
            "CREATE TABLE mcp_tokens_new (
                id INTEGER PRIMARY KEY,
                token_hash TEXT NOT NULL UNIQUE,
                name TEXT NOT NULL,
                scopes TEXT,
                created_by INTEGER NOT NULL,
                expires_at TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                last_used_at TEXT,
                FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE CASCADE
            )",
        )
        .execute(&mut *tx)
        .await?;

        // Copy data (excluding site_id)
        sqlx::query(
            "INSERT INTO mcp_tokens_new (id, token_hash, name, scopes, created_by, expires_at, created_at, last_used_at)
             SELECT id, token_hash, name, scopes, created_by, expires_at, created_at, last_used_at
             FROM mcp_tokens",
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query("DROP TABLE mcp_tokens")
            .execute(&mut *tx)
            .await?;
        sqlx::query("ALTER TABLE mcp_tokens_new RENAME TO mcp_tokens")
            .execute(&mut *tx)
            .await?;

        // Recreate indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_mcp_tokens_hash ON mcp_tokens(token_hash)")
            .execute(&mut *tx)
            .await?;
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_mcp_tokens_expires ON mcp_tokens(expires_at)")
            .execute(&mut *tx)
            .await?;
    }

    // Drop the sites table (no longer needed in multi-db mode)
    sqlx::query("DROP TABLE IF EXISTS sites")
        .execute(&mut *tx)
        .await?;

    // Update _sqlx_migrations to mark new migrations as applied
    // This prevents them from running again
    let now = chrono::Utc::now().to_rfc3339();

    // Mark migration 016 as applied
    sqlx::query(
        "INSERT OR IGNORE INTO _sqlx_migrations (version, description, installed_on, success, checksum, execution_time)
         VALUES (16, 'multidb_site_config', ?, 1, X'00', 0)",
    )
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    // Mark migration 017 as applied
    sqlx::query(
        "INSERT OR IGNORE INTO _sqlx_migrations (version, description, installed_on, success, checksum, execution_time)
         VALUES (17, 'remove_site_id_from_all_tables', ?, 1, X'00', 0)",
    )
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    // Mark migration 018 as applied
    sqlx::query(
        "INSERT OR IGNORE INTO _sqlx_migrations (version, description, installed_on, success, checksum, execution_time)
         VALUES (18, 'remove_site_id_from_mcp_tokens', ?, 1, X'00', 0)",
    )
    .bind(&now)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    // Re-enable foreign keys
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(pool)
        .await?;

    Ok(())
}
