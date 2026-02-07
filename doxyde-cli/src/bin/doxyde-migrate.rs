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
use clap::Parser;
use sqlx::SqlitePool;
use std::fs;
use std::path::{Path, PathBuf};

/// Migrate a legacy Doxyde database to the new multi-site structure
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the legacy database file
    #[arg(short, long, default_value = "doxyde.db")]
    source: String,

    /// Domain name for the site (e.g., example.com)
    #[arg(short, long)]
    domain: String,

    /// Sites directory where the new database will be stored
    #[arg(short = 't', long, default_value = "./sites")]
    sites_directory: String,

    /// Force migration even if destination exists
    #[arg(short, long)]
    force: bool,

    /// Dry run - show what would be done without actually doing it
    #[arg(long)]
    dry_run: bool,

    /// Update the domain in the database to match the specified domain
    #[arg(long)]
    update_domain: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Validate source database exists
    if !Path::new(&args.source).exists() {
        anyhow::bail!("Source database '{}' does not exist", args.source);
    }

    // Calculate destination path
    let site_dir = doxyde_web::domain_utils::resolve_site_directory(
        &PathBuf::from(&args.sites_directory),
        &args.domain,
    );
    let dest_db = site_dir.join("site.db");

    println!("Migration Plan:");
    println!("  Source:      {}", args.source);
    println!("  Domain:      {}", args.domain);
    println!("  Destination: {}", dest_db.display());

    // Check if destination exists
    if dest_db.exists() && !args.force {
        anyhow::bail!(
            "Destination database already exists: {}\nUse --force to overwrite",
            dest_db.display()
        );
    }

    if args.dry_run {
        println!("\n[DRY RUN] Would perform the following actions:");
        println!("1. Create directory: {}", site_dir.display());
        println!("2. Copy {} to {}", args.source, dest_db.display());
        println!("3. Verify the migrated database");
        return Ok(());
    }

    // Create site directory
    println!("\nCreating site directory...");
    fs::create_dir_all(&site_dir)
        .with_context(|| format!("Failed to create directory: {}", site_dir.display()))?;

    // Copy database file
    println!("Copying database...");
    fs::copy(&args.source, &dest_db).with_context(|| {
        format!(
            "Failed to copy database from {} to {}",
            args.source,
            dest_db.display()
        )
    })?;

    // Verify the new database
    println!("Verifying migrated database...");
    let db_url = format!("sqlite:{}", dest_db.display());
    let pool = SqlitePool::connect(&db_url)
        .await
        .with_context(|| format!("Failed to connect to migrated database: {}", db_url))?;

    // Check that we can query the sites table
    let site_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sites")
        .fetch_one(&pool)
        .await
        .context("Failed to query sites table")?;

    // Check that the domain matches
    let site_domain: Option<String> = sqlx::query_scalar("SELECT domain FROM sites LIMIT 1")
        .fetch_optional(&pool)
        .await
        .context("Failed to query site domain")?;

    if let Some(existing_domain) = site_domain {
        if existing_domain != args.domain {
            println!(
                "\n⚠️  WARNING: The database contains site '{}' but you specified '{}'",
                existing_domain, args.domain
            );

            if args.update_domain {
                println!("   Updating domain in database...");
                sqlx::query("UPDATE sites SET domain = ? WHERE id = 1")
                    .bind(&args.domain)
                    .execute(&pool)
                    .await
                    .context("Failed to update domain")?;
                println!("   ✅ Domain updated successfully");
            } else {
                println!("   Use --update-domain to automatically update it");
                println!(
                    "   Or manually run: UPDATE sites SET domain = '{}' WHERE id = 1;",
                    args.domain
                );
            }
        }
    }

    pool.close().await;

    println!("\n✅ Migration completed successfully!");
    println!("   Sites in database: {}", site_count);
    println!("\nTo use this database, configure Doxyde with:");
    println!("   SITES_DIR={}", args.sites_directory);
    println!("   or add to /etc/doxyde.conf:");
    println!("   sites_directory = \"{}\"", args.sites_directory);

    Ok(())
}
