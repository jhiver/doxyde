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
use sqlx::{Column, Row, SqlitePool};
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

    // Get all sites
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
        let page_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pages WHERE site_id = ?")
            .bind(site_id)
            .fetch_one(&pool)
            .await?;

        // Count users for this site
        let user_count: i64 =
            sqlx::query_scalar("SELECT COUNT(DISTINCT user_id) FROM site_users WHERE site_id = ?")
                .bind(site_id)
                .fetch_one(&pool)
                .await?;

        println!("Site: {} ({})", title, domain);
        println!("  ID: {}", site_id);
        println!("  Pages: {}", page_count);
        println!("  Users: {}", user_count);
        println!();
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

    // Connect to source database
    let source_pool = SqlitePool::connect(&format!("sqlite:{}", source_path))
        .await
        .context("Failed to connect to source database")?;

    // Get all sites
    let sites = sqlx::query("SELECT id, domain FROM sites ORDER BY domain")
        .fetch_all(&source_pool)
        .await
        .context("Failed to query sites")?;

    println!("Found {} sites to migrate", sites.len());
    println!();

    // Process each site
    for site in sites {
        let site_id: i64 = site.get("id");
        let domain: String = site.get("domain");

        // Calculate site directory
        let base_domain = extract_base_domain(&domain);
        let site_key = calculate_site_key(&base_domain);
        let site_dir = target_path.join(format!("{}-{}", base_domain, site_key));

        println!("Processing site: {} (ID: {})", domain, site_id);
        println!("  Target directory: {}", site_dir.display());

        if dry_run {
            println!("  [DRY RUN] Would create site database");
        } else {
            // Create site directory
            if !site_dir.exists() {
                fs::create_dir_all(&site_dir).context("Failed to create site directory")?;
            }

            // Create site database
            let site_db_path = site_dir.join("site.db");
            let site_db_url = format!("sqlite:{}?mode=rwc", site_db_path.display());

            // Connect to target database (with create mode)
            let target_pool = SqlitePool::connect(&site_db_url)
                .await
                .context("Failed to connect to target database")?;

            // Run migrations on target database
            println!("  Running migrations...");
            sqlx::migrate!("../migrations")
                .run(&target_pool)
                .await
                .context("Failed to run migrations")?;

            // Copy site data
            println!("  Copying site data...");
            copy_site_data(&source_pool, &target_pool, site_id, &domain).await?;

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

async fn copy_site_data(
    source: &SqlitePool,
    target: &SqlitePool,
    site_id: i64,
    _domain: &str,
) -> Result<()> {
    // Start transaction
    let mut tx = target.begin().await?;

    // Get site data from source
    let site_data = sqlx::query("SELECT * FROM sites WHERE id = ?")
        .bind(site_id)
        .fetch_one(source)
        .await?;

    // In multi-database mode, we create a site_config table with just title and description
    // The domain is determined by the directory/context, not stored in the DB
    let has_description = site_data
        .columns()
        .iter()
        .any(|col| col.name() == "description");

    // Create site_config with a single row
    if has_description {
        sqlx::query(
            "INSERT INTO site_config (id, title, description, created_at, updated_at)
             VALUES (1, ?, ?, ?, ?)",
        )
        .bind(site_data.get::<String, _>("title"))
        .bind(site_data.get::<Option<String>, _>("description"))
        .bind(site_data.get::<String, _>("created_at"))
        .bind(site_data.get::<String, _>("updated_at"))
        .execute(&mut *tx)
        .await?;
    } else {
        sqlx::query(
            "INSERT INTO site_config (id, title, created_at, updated_at)
             VALUES (1, ?, ?, ?)",
        )
        .bind(site_data.get::<String, _>("title"))
        .bind(site_data.get::<String, _>("created_at"))
        .bind(site_data.get::<String, _>("updated_at"))
        .execute(&mut *tx)
        .await?;
    }

    // Copy users that belong to this site
    let site_users = sqlx::query("SELECT DISTINCT user_id FROM site_users WHERE site_id = ?")
        .bind(site_id)
        .fetch_all(source)
        .await?;

    for site_user in site_users {
        let user_id: i64 = site_user.get("user_id");

        // Check if user already exists in target
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE id = ?)")
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await?;

        if !exists {
            // Copy user
            let user = sqlx::query("SELECT * FROM users WHERE id = ?")
                .bind(user_id)
                .fetch_one(source)
                .await?;

            sqlx::query(
                "INSERT INTO users (id, email, username, password_hash, is_active, is_admin, created_at, updated_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(user.get::<i64, _>("id"))
            .bind(user.get::<String, _>("email"))
            .bind(user.get::<String, _>("username"))
            .bind(user.get::<String, _>("password_hash"))
            .bind(user.get::<bool, _>("is_active"))
            .bind(user.get::<bool, _>("is_admin"))
            .bind(user.get::<String, _>("created_at"))
            .bind(user.get::<String, _>("updated_at"))
            .execute(&mut *tx)
            .await?;
        }
    }

    // Copy site_users (without site_id since this DB is for one site only)
    let site_users_data = sqlx::query("SELECT * FROM site_users WHERE site_id = ?")
        .bind(site_id)
        .fetch_all(source)
        .await?;

    for su in site_users_data {
        sqlx::query(
            "INSERT INTO site_users (user_id, role, created_at)
             VALUES (?, ?, ?)",
        )
        .bind(su.get::<i64, _>("user_id"))
        .bind(su.get::<String, _>("role"))
        .bind(su.get::<String, _>("created_at"))
        .execute(&mut *tx)
        .await?;
    }

    // Copy pages and their versions/components
    let pages = sqlx::query("SELECT * FROM pages WHERE site_id = ?")
        .bind(site_id)
        .fetch_all(source)
        .await?;

    for page in pages {
        let page_id: i64 = page.get("id");

        // Copy page with all available columns
        let columns = page.columns();
        let mut query_parts = vec![
            "id",
            "parent_page_id",
            "slug",
            "position",
            "created_at",
            "updated_at",
        ];
        let mut values_parts = vec!["?", "?", "?", "?", "?", "?"];

        // Check for optional columns
        if columns.iter().any(|c| c.name() == "title") {
            query_parts.push("title");
            values_parts.push("?");
        }
        if columns.iter().any(|c| c.name() == "description") {
            query_parts.push("description");
            values_parts.push("?");
        }
        if columns.iter().any(|c| c.name() == "keywords") {
            query_parts.push("keywords");
            values_parts.push("?");
        }
        if columns.iter().any(|c| c.name() == "template") {
            query_parts.push("template");
            values_parts.push("?");
        }
        if columns.iter().any(|c| c.name() == "meta_robots") {
            query_parts.push("meta_robots");
            values_parts.push("?");
        }
        if columns.iter().any(|c| c.name() == "canonical_url") {
            query_parts.push("canonical_url");
            values_parts.push("?");
        }
        if columns.iter().any(|c| c.name() == "og_image_url") {
            query_parts.push("og_image_url");
            values_parts.push("?");
        }
        if columns.iter().any(|c| c.name() == "structured_data_type") {
            query_parts.push("structured_data_type");
            values_parts.push("?");
        }
        if columns.iter().any(|c| c.name() == "sort_mode") {
            query_parts.push("sort_mode");
            values_parts.push("?");
        }

        let query = format!(
            "INSERT INTO pages ({}) VALUES ({})",
            query_parts.join(", "),
            values_parts.join(", ")
        );

        let mut q = sqlx::query(&query)
            .bind(page.get::<i64, _>("id"))
            .bind(page.get::<Option<i64>, _>("parent_page_id"))
            .bind(page.get::<String, _>("slug"))
            .bind(page.get::<i32, _>("position"))
            .bind(page.get::<String, _>("created_at"))
            .bind(page.get::<String, _>("updated_at"));

        // Bind optional columns
        if columns.iter().any(|c| c.name() == "title") {
            q = q.bind(page.get::<String, _>("title"));
        }
        if columns.iter().any(|c| c.name() == "description") {
            q = q.bind(page.get::<Option<String>, _>("description"));
        }
        if columns.iter().any(|c| c.name() == "keywords") {
            q = q.bind(page.get::<Option<String>, _>("keywords"));
        }
        if columns.iter().any(|c| c.name() == "template") {
            q = q.bind(page.get::<Option<String>, _>("template"));
        }
        if columns.iter().any(|c| c.name() == "meta_robots") {
            q = q.bind(page.get::<String, _>("meta_robots"));
        }
        if columns.iter().any(|c| c.name() == "canonical_url") {
            q = q.bind(page.get::<Option<String>, _>("canonical_url"));
        }
        if columns.iter().any(|c| c.name() == "og_image_url") {
            q = q.bind(page.get::<Option<String>, _>("og_image_url"));
        }
        if columns.iter().any(|c| c.name() == "structured_data_type") {
            q = q.bind(page.get::<String, _>("structured_data_type"));
        }
        if columns.iter().any(|c| c.name() == "sort_mode") {
            q = q.bind(page.get::<String, _>("sort_mode"));
        }

        q.execute(&mut *tx).await?;

        // Copy page versions
        let versions = sqlx::query("SELECT * FROM page_versions WHERE page_id = ?")
            .bind(page_id)
            .fetch_all(source)
            .await?;

        for version in versions {
            let version_id: i64 = version.get("id");
            let columns = version.columns();

            // Handle different schema versions
            let version_number = if columns.iter().any(|c| c.name() == "version_number") {
                version.get::<i32, _>("version_number")
            } else if columns.iter().any(|c| c.name() == "version") {
                version.get::<i32, _>("version")
            } else {
                1 // Default version
            };

            // Get title from page if not in version
            let _title = if columns.iter().any(|c| c.name() == "title") {
                version.get::<String, _>("title")
            } else {
                // Use page title as fallback
                page.get::<String, _>("title")
            };

            // Get template from page if not in version
            let _template = if columns.iter().any(|c| c.name() == "template") {
                version.get::<String, _>("template")
            } else if page.columns().iter().any(|c| c.name() == "template") {
                page.get::<Option<String>, _>("template")
                    .unwrap_or_else(|| "default".to_string())
            } else {
                "default".to_string()
            };

            sqlx::query(
                "INSERT INTO page_versions (id, page_id, version_number, is_published, created_at, created_by)
                 VALUES (?, ?, ?, ?, ?, ?)"
            )
            .bind(version.get::<i64, _>("id"))
            .bind(version.get::<i64, _>("page_id"))
            .bind(version_number)
            .bind(version.get::<bool, _>("is_published"))
            .bind(version.get::<String, _>("created_at"))
            .bind(version.get::<Option<String>, _>("created_by"))
            .execute(&mut *tx)
            .await?;

            // Copy components for this version
            let components = sqlx::query("SELECT * FROM components WHERE page_version_id = ?")
                .bind(version_id)
                .fetch_all(source)
                .await?;

            for component in components {
                sqlx::query(
                    "INSERT INTO components (id, page_version_id, component_type, position, content, title, template, created_at, updated_at)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
                )
                .bind(component.get::<i64, _>("id"))
                .bind(component.get::<i64, _>("page_version_id"))
                .bind(component.get::<String, _>("component_type"))
                .bind(component.get::<i32, _>("position"))
                .bind(component.get::<serde_json::Value, _>("content"))
                .bind(component.get::<Option<String>, _>("title"))
                .bind(component.get::<String, _>("template"))
                .bind(component.get::<String, _>("created_at"))
                .bind(component.get::<String, _>("updated_at"))
                .execute(&mut *tx)
                .await?;
            }
        }
    }

    // Note: We're not copying sessions, CSRF tokens, or OAuth tokens as those are temporary

    // Commit transaction
    tx.commit().await?;

    Ok(())
}
