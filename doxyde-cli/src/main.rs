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

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use doxyde_core::models::user::User;
use doxyde_db::repositories::{ComponentRepository, SiteUserRepository, UserRepository};
use doxyde_web::domain_utils::resolve_site_directory;
use doxyde_web::uploads::{
    build_hash_based_path, build_thumb_path, compute_content_hash, extract_image_metadata,
    generate_thumbnail, ImageFormat,
};
use sqlx::SqlitePool;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "doxyde")]
#[command(about = "Doxyde CLI tool for site and user management")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize the database (create tables)
    Init {
        /// Site domain to initialize (required for multi-database mode)
        #[arg(long)]
        site: Option<String>,
    },

    /// Site management commands
    Site {
        #[command(subcommand)]
        command: SiteCommands,

        /// Site domain to operate on (required for init, show, update-title commands)
        #[arg(long, global = true)]
        site: Option<String>,
    },

    /// User management commands
    User {
        #[command(subcommand)]
        command: UserCommands,

        /// Site domain to operate on (required for multi-database mode)
        #[arg(long, global = true)]
        site: Option<String>,
    },

    /// Image management commands
    Image {
        #[command(subcommand)]
        command: ImageCommands,
    },
}

#[derive(Subcommand)]
enum SiteCommands {
    /// Create a new site with directory and database
    Create {
        /// Domain name for the site
        domain: String,
        /// Site title
        title: String,
    },

    /// List all sites
    List,

    /// Initialize site configuration in current database
    Init {
        /// Site title
        title: String,
    },

    /// Show site configuration
    Show,

    /// Update site title
    UpdateTitle {
        /// New site title
        title: String,
    },
}

#[derive(Subcommand)]
enum UserCommands {
    /// Create a new user
    Create {
        /// Email address
        email: String,
        /// Username
        username: String,
        /// Make user an admin
        #[arg(long)]
        admin: bool,
        /// Password (will prompt if not provided)
        #[arg(long)]
        password: Option<String>,
    },

    /// Grant user access to this site
    Grant {
        /// Username or email
        user: String,
        /// Role (viewer, editor, owner)
        #[arg(default_value = "owner")]
        role: String,
    },

    /// Change user password
    Password {
        /// Username or email
        user: String,
        /// New password (will prompt if not provided)
        #[arg(long)]
        password: Option<String>,
    },
}

#[derive(Subcommand)]
enum ImageCommands {
    /// Migrate images to SHA256-based storage with thumbnails
    Migrate {
        /// Site domain to migrate
        #[arg(long)]
        site: String,
        /// Show what would be done without making changes
        #[arg(long)]
        dry_run: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if it exists
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    // Get database URL from environment
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:doxyde.db".to_string());

    match cli.command {
        Commands::Init { site } => {
            let database_url = resolve_database_url(&database_url, site.as_deref())?;
            init_database(&database_url).await
        }
        Commands::Site { command, site } => handle_site_command(command, site.as_deref()).await,
        Commands::User { command, site } => {
            let database_url = resolve_database_url(&database_url, site.as_deref())?;
            let pool = connect_database(&database_url).await?;
            handle_user_command(command, pool).await
        }
        Commands::Image { command } => handle_image_command(command).await,
    }
}

async fn init_database(database_url: &str) -> Result<()> {
    println!("Initializing database at: {}", database_url);

    // Use the shared init_database function from doxyde-db
    let _pool = doxyde_db::init_database(database_url).await?;

    println!("Database initialized successfully!");
    Ok(())
}

async fn connect_database(database_url: &str) -> Result<SqlitePool> {
    // Use the shared init_database which also ensures migrations are run
    doxyde_db::init_database(database_url).await
}

async fn handle_site_command(command: SiteCommands, site: Option<&str>) -> Result<()> {
    match command {
        SiteCommands::Create { domain, title } => create_site(&domain, &title).await,

        SiteCommands::List => list_sites().await,
        SiteCommands::Init { title } => {
            println!("Initializing site configuration with title: {}", title);

            let database_url = resolve_database_url("sqlite:doxyde.db", site)?;
            let pool = connect_database(&database_url).await?;

            // Check if site_config already exists
            let existing = sqlx::query!("SELECT title FROM site_config WHERE id = 1")
                .fetch_optional(&pool)
                .await?;

            if existing.is_some() {
                anyhow::bail!(
                    "Site configuration already exists. Use 'update-title' to change it."
                );
            }

            // Create site_config entry
            sqlx::query!("INSERT INTO site_config (id, title) VALUES (1, ?)", title)
                .execute(&pool)
                .await
                .context("Failed to create site configuration")?;

            println!("Site configuration initialized successfully!");
            Ok(())
        }

        SiteCommands::Show => {
            println!("Site configuration:");

            let database_url = resolve_database_url("sqlite:doxyde.db", site)?;
            let pool = connect_database(&database_url).await?;

            let config = sqlx::query!("SELECT title FROM site_config WHERE id = 1")
                .fetch_optional(&pool)
                .await?;

            match config {
                Some(config) => {
                    println!("Title: {}", config.title);
                }
                None => {
                    println!("No site configuration found. Run 'site init' first.");
                }
            }
            Ok(())
        }

        SiteCommands::UpdateTitle { title } => {
            println!("Updating site title to: {}", title);

            let database_url = resolve_database_url("sqlite:doxyde.db", site)?;
            let pool = connect_database(&database_url).await?;

            let result = sqlx::query!("UPDATE site_config SET title = ? WHERE id = 1", title)
                .execute(&pool)
                .await
                .context("Failed to update site title")?;

            if result.rows_affected() == 0 {
                anyhow::bail!("Site configuration not found. Run 'site init' first.");
            }

            println!("Site title updated successfully!");
            Ok(())
        }
    }
}

async fn handle_user_command(command: UserCommands, pool: SqlitePool) -> Result<()> {
    let user_repo = UserRepository::new(pool.clone());

    match command {
        UserCommands::Create {
            email,
            username,
            admin,
            password,
        } => {
            println!("Creating user: {} ({})", username, email);

            // Get password
            let password = match password {
                Some(pwd) => pwd,
                None => {
                    // Prompt for password
                    print!("Password: ");
                    use std::io::{self, Write};
                    io::stdout().flush()?;

                    rpassword::read_password().context("Failed to read password")?
                }
            };

            let mut user = User::new(email.clone(), username.clone(), &password)?;
            user.is_admin = admin;

            if let Err(e) = user.is_valid() {
                anyhow::bail!("Invalid user data: {}", e);
            }

            let user_id = user_repo
                .create(&user)
                .await
                .context("Failed to create user")?;

            println!("User created successfully with ID: {}", user_id);
            if admin {
                println!("User has admin privileges");
            }
            Ok(())
        }

        UserCommands::Grant { user, role } => {
            println!("Granting {} role to {}", role, user);

            // Find user by username or email
            let found_user = if user.contains('@') {
                user_repo.find_by_email(&user).await?
            } else {
                user_repo.find_by_username(&user).await?
            };

            let found_user = found_user.ok_or_else(|| anyhow::anyhow!("User not found"))?;

            // Parse role
            let site_role = match role.as_str() {
                "viewer" => doxyde_core::models::permission::SiteRole::Viewer,
                "editor" => doxyde_core::models::permission::SiteRole::Editor,
                "owner" => doxyde_core::models::permission::SiteRole::Owner,
                _ => anyhow::bail!("Invalid role. Must be: viewer, editor, or owner"),
            };

            // In multi-database mode, site_id is always 1
            let site_user_repo = SiteUserRepository::new(pool);
            let user_id = found_user.id.ok_or_else(|| anyhow!("User has no ID"))?;
            let site_user = doxyde_core::models::permission::SiteUser::new(user_id, site_role);

            site_user_repo.create(&site_user).await?;

            println!("Access granted successfully!");
            Ok(())
        }

        UserCommands::Password { user, password } => {
            println!("Changing password for {}", user);

            // Find user by username or email
            let found_user = if user.contains('@') {
                user_repo.find_by_email(&user).await?
            } else {
                user_repo.find_by_username(&user).await?
            };

            let mut found_user = found_user.ok_or_else(|| anyhow::anyhow!("User not found"))?;

            // Get password
            let password = match password {
                Some(p) => p,
                None => {
                    // Prompt for password
                    print!("New password: ");
                    std::io::stdout().flush()?;
                    rpassword::read_password()?
                }
            };

            // Set new password
            found_user.set_password(&password)?;

            // Update user
            user_repo.update(&found_user).await?;

            println!("Password changed successfully!");
            Ok(())
        }
    }
}

/// Resolves the database URL based on the given base URL and optional site domain
fn resolve_database_url(base_url: &str, site: Option<&str>) -> Result<String> {
    match site {
        Some(domain) => {
            // Get sites directory (default: "./sites")
            let sites_dir =
                std::env::var("DOXYDE_SITES_DIRECTORY").unwrap_or_else(|_| "./sites".to_string());

            let sites_path = PathBuf::from(sites_dir);
            let site_dir = resolve_site_directory(&sites_path, domain);
            let db_path = site_dir.join("site.db");

            Ok(format!("sqlite:{}", db_path.display()))
        }
        None => {
            // If DATABASE_URL is set, use it; otherwise show error
            if base_url != "sqlite:doxyde.db" {
                Ok(base_url.to_string())
            } else {
                Err(anyhow!(
                    "No site specified and DATABASE_URL not set. Use --site <domain> or set DATABASE_URL"
                ))
            }
        }
    }
}

/// Creates a new site with directory and database
async fn create_site(domain: &str, title: &str) -> Result<()> {
    println!("Creating site: {} ({})", domain, title);

    // Validate domain
    if domain.is_empty() {
        return Err(anyhow!("Domain cannot be empty"));
    }

    // Get sites directory
    let sites_dir =
        std::env::var("DOXYDE_SITES_DIRECTORY").unwrap_or_else(|_| "./sites".to_string());

    let sites_path = PathBuf::from(&sites_dir);
    let site_dir = resolve_site_directory(&sites_path, domain);

    // Check if site already exists
    if site_dir.exists() {
        let db_path = site_dir.join("site.db");
        if db_path.exists() {
            return Err(anyhow!(
                "Site already exists at: {}\nDatabase: {}",
                site_dir.display(),
                db_path.display()
            ));
        }
    }

    // Create site directory
    println!("Creating directory: {}", site_dir.display());
    fs::create_dir_all(&site_dir)
        .with_context(|| format!("Failed to create site directory: {}", site_dir.display()))?;

    // Initialize database
    let db_path = site_dir.join("site.db");
    let database_url = format!("sqlite:{}", db_path.display());

    println!("Initializing database: {}", db_path.display());
    let pool = doxyde_db::init_database(&database_url)
        .await
        .with_context(|| format!("Failed to initialize database: {}", database_url))?;

    // Create or update site_config entry (migrations create a default entry)
    println!("Setting site title: {}", title);
    sqlx::query!("UPDATE site_config SET title = ? WHERE id = 1", title)
        .execute(&pool)
        .await
        .context("Failed to set site configuration")?;

    println!("✅ Site created successfully!");
    println!("   Domain: {}", domain);
    println!("   Title: {}", title);
    println!("   Directory: {}", site_dir.display());
    println!("   Database: {}", db_path.display());

    Ok(())
}

/// Lists all sites
async fn list_sites() -> Result<()> {
    // Get sites directory
    let sites_dir =
        std::env::var("DOXYDE_SITES_DIRECTORY").unwrap_or_else(|_| "./sites".to_string());

    let sites_path = PathBuf::from(&sites_dir);

    println!("Sites directory: {}", sites_path.display());

    if !sites_path.exists() {
        println!("No sites directory found. Use 'doxyde site create' to create your first site.");
        return Ok(());
    }

    // Read all directories
    let entries = fs::read_dir(&sites_path)
        .with_context(|| format!("Failed to read sites directory: {}", sites_path.display()))?;

    let mut sites = Vec::new();

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let db_path = path.join("site.db");

            if db_path.exists() {
                // Try to connect to database and get site info
                let database_url = format!("sqlite:{}", db_path.display());

                match connect_database(&database_url).await {
                    Ok(pool) => {
                        // Get site config
                        match sqlx::query!("SELECT title FROM site_config WHERE id = 1")
                            .fetch_optional(&pool)
                            .await
                        {
                            Ok(Some(config)) => {
                                // Extract domain from directory name
                                if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                                    // Remove hash suffix to get domain
                                    let domain = extract_domain_from_directory(dir_name);
                                    sites.push((domain, config.title, path.clone()));
                                }
                            }
                            Ok(None) => {
                                println!(
                                    "⚠️  Database found but no site config: {}",
                                    db_path.display()
                                );
                            }
                            Err(e) => {
                                println!(
                                    "⚠️  Error reading site config from {}: {}",
                                    db_path.display(),
                                    e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        println!(
                            "⚠️  Error connecting to database {}: {}",
                            db_path.display(),
                            e
                        );
                    }
                }
            }
        }
    }

    if sites.is_empty() {
        println!("No sites found. Use 'doxyde site create' to create your first site.");
    } else {
        println!("\nFound {} site(s):", sites.len());
        for (domain, title, path) in sites {
            println!("  • {} - {} ({})", domain, title, path.display());
        }
    }

    Ok(())
}

async fn handle_image_command(command: ImageCommands) -> Result<()> {
    match command {
        ImageCommands::Migrate { site, dry_run } => migrate_images(&site, dry_run).await,
    }
}

/// Migrate a single image file to hash-based storage
fn migrate_single_file(
    file_path: &str,
    upload_base: &Path,
    dry_run: bool,
) -> Result<Option<MigratedFile>> {
    let path = PathBuf::from(file_path);
    if !path.exists() {
        println!("  SKIP (missing): {}", file_path);
        return Ok(None);
    }

    let data = fs::read(&path).with_context(|| format!("Failed to read: {}", file_path))?;

    let metadata =
        extract_image_metadata(&data).with_context(|| format!("Failed to parse: {}", file_path))?;

    let ext = metadata.format.extension();
    let hash = compute_content_hash(&data);
    let new_path = build_hash_based_path(upload_base, &hash, ext)?;

    let thumb_path = build_thumb_path(&new_path)?;
    let has_thumb = matches!(
        metadata.format,
        ImageFormat::Jpeg | ImageFormat::Png | ImageFormat::Webp
    );

    if dry_run {
        println!("  {} -> {}", file_path, new_path.display());
        if has_thumb {
            println!("    + thumbnail: {}", thumb_path.display());
        }
        return Ok(Some(MigratedFile {
            new_path,
            thumb_path: if has_thumb { Some(thumb_path) } else { None },
            content_hash: hash,
            saved_bytes: 0,
        }));
    }

    // Copy original to hash-based path (dedup)
    let mut saved_bytes: i64 = 0;
    if new_path.exists() {
        saved_bytes = data.len() as i64;
    } else {
        if let Some(parent) = new_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&new_path, &data)?;
    }

    // Generate thumbnail
    let actual_thumb = if has_thumb {
        if thumb_path.exists() {
            Some(thumb_path.clone())
        } else {
            match generate_thumbnail(&data, &metadata.format, 1600)? {
                Some(thumb_data) => {
                    fs::write(&thumb_path, &thumb_data)?;
                    Some(thumb_path.clone())
                }
                None => None,
            }
        }
    } else {
        None
    };

    println!("  OK: {} -> {}", file_path, new_path.display());

    Ok(Some(MigratedFile {
        new_path,
        thumb_path: actual_thumb,
        content_hash: hash,
        saved_bytes,
    }))
}

struct MigratedFile {
    new_path: PathBuf,
    thumb_path: Option<PathBuf>,
    content_hash: String,
    saved_bytes: i64,
}

/// Migrate images for a site to SHA256-based storage
async fn migrate_images(domain: &str, dry_run: bool) -> Result<()> {
    if dry_run {
        println!("DRY RUN: No files will be modified");
    }
    println!("Migrating images for site: {}", domain);

    // Connect to the site database
    let sites_dir =
        std::env::var("DOXYDE_SITES_DIRECTORY").unwrap_or_else(|_| "./sites".to_string());
    let sites_path = PathBuf::from(&sites_dir);
    let site_dir = resolve_site_directory(&sites_path, domain);
    let db_path = site_dir.join("site.db");

    if !db_path.exists() {
        return Err(anyhow!("Site database not found: {}", db_path.display()));
    }

    let database_url = format!("sqlite:{}", db_path.display());
    let pool = connect_database(&database_url).await?;

    // Determine upload base directory
    let upload_base = site_dir.join("uploads");
    fs::create_dir_all(&upload_base)?;

    // Find all image components
    let component_repo = ComponentRepository::new(pool.clone());
    let rows: Vec<(i64, String)> =
        sqlx::query_as("SELECT id, content FROM components WHERE component_type = 'image'")
            .fetch_all(&pool)
            .await
            .context("Failed to query image components")?;

    println!("Found {} image components", rows.len());

    // Group by unique file_path to avoid duplicating work
    let mut files_map: HashMap<String, Vec<i64>> = HashMap::new();
    for (id, content_str) in &rows {
        let content: serde_json::Value =
            serde_json::from_str(content_str).unwrap_or(serde_json::Value::Null);
        if let Some(fp) = content.get("file_path").and_then(|v| v.as_str()) {
            files_map.entry(fp.to_string()).or_default().push(*id);
        }
    }

    println!(
        "Found {} unique files across {} components",
        files_map.len(),
        rows.len()
    );

    let mut migrated = 0u32;
    let mut skipped = 0u32;
    let mut total_saved: i64 = 0;
    let mut thumbs_created = 0u32;

    for (file_path, component_ids) in &files_map {
        match migrate_single_file(file_path, &upload_base, dry_run)? {
            Some(result) => {
                migrated += 1;
                total_saved += result.saved_bytes;
                if result.thumb_path.is_some() {
                    thumbs_created += 1;
                }

                if !dry_run {
                    // Update all components that use this file
                    for &comp_id in component_ids {
                        let comp = component_repo
                            .find_by_id(comp_id)
                            .await?
                            .ok_or_else(|| anyhow!("Component {} not found", comp_id))?;

                        let mut content = comp.content.clone();
                        content["file_path"] = serde_json::json!(result.new_path.to_string_lossy());
                        content["content_hash"] = serde_json::json!(result.content_hash);
                        if let Some(ref tp) = result.thumb_path {
                            content["thumb_file_path"] = serde_json::json!(tp.to_string_lossy());
                        }

                        component_repo
                            .update_content(
                                comp_id,
                                content,
                                comp.title.clone(),
                                comp.template.clone(),
                            )
                            .await
                            .with_context(|| format!("Failed to update component {}", comp_id))?;
                    }
                }
            }
            None => {
                skipped += 1;
            }
        }
    }

    println!("\nMigration complete:");
    println!("  Files migrated: {}", migrated);
    println!("  Files skipped: {}", skipped);
    println!("  Thumbnails created: {}", thumbs_created);
    if total_saved > 0 {
        println!(
            "  Space saved (dedup): {} bytes ({:.1} MB)",
            total_saved,
            total_saved as f64 / 1_048_576.0
        );
    }

    Ok(())
}

/// Extracts the domain from a directory name by removing the hash suffix
fn extract_domain_from_directory(dir_name: &str) -> String {
    // Directory format is: domain-hash
    // We need to remove the last 9 characters (dash + 8 char hash)
    if dir_name.len() > 9 {
        let without_hash = &dir_name[..dir_name.len() - 9];
        without_hash.replace('-', ".")
    } else {
        dir_name.to_string()
    }
}
