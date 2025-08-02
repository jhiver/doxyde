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
use doxyde_core::models::{site::Site, user::User};
use doxyde_db::repositories::{SiteRepository, SiteUserRepository, UserRepository};
use sqlx::SqlitePool;
use std::io::Write;

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
    Init,

    /// Site management commands
    Site {
        #[command(subcommand)]
        command: SiteCommands,
    },

    /// User management commands
    User {
        #[command(subcommand)]
        command: UserCommands,
    },
}

#[derive(Subcommand)]
enum SiteCommands {
    /// Create a new site
    Create {
        /// Domain name (e.g., localhost, example.com)
        domain: String,
        /// Site title
        title: String,
    },

    /// List all sites
    List,
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

    /// Grant user access to a site
    Grant {
        /// Username or email
        user: String,
        /// Site domain
        site: String,
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

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file if it exists
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    // Get database URL from environment
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:doxyde.db".to_string());

    match cli.command {
        Commands::Init => init_database(&database_url).await,
        Commands::Site { command } => {
            let pool = connect_database(&database_url).await?;
            handle_site_command(command, pool).await
        }
        Commands::User { command } => {
            let pool = connect_database(&database_url).await?;
            handle_user_command(command, pool).await
        }
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

async fn handle_site_command(command: SiteCommands, pool: SqlitePool) -> Result<()> {
    let site_repo = SiteRepository::new(pool);

    match command {
        SiteCommands::Create { domain, title } => {
            println!("Creating site: {} - {}", domain, title);

            let site = Site::new(domain.clone(), title.clone());
            if let Err(e) = site.is_valid() {
                anyhow::bail!("Invalid site data: {}", e);
            }

            let site_id = site_repo
                .create(&site)
                .await
                .context("Failed to create site")?;

            println!("Site created successfully with ID: {}", site_id);
            Ok(())
        }

        SiteCommands::List => {
            println!("Listing all sites:");
            println!("{:<5} {:<30} {:<30}", "ID", "Domain", "Title");
            println!("{:-<65}", "");

            // Since we don't have a list_all method, we'll need to add one
            // Display confirmation message for successful deletion
            println!("(Site listing not implemented yet)");
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

        UserCommands::Grant { user, site, role } => {
            println!("Granting {} role on {} to {}", role, site, user);

            // Find user by username or email
            let found_user = if user.contains('@') {
                user_repo.find_by_email(&user).await?
            } else {
                user_repo.find_by_username(&user).await?
            };

            let found_user = found_user.ok_or_else(|| anyhow::anyhow!("User not found"))?;

            // Find site by domain
            let site_repo = SiteRepository::new(pool.clone());
            let found_site = site_repo
                .find_by_domain(&site)
                .await?
                .ok_or_else(|| anyhow::anyhow!("Site not found"))?;

            // Parse role
            let site_role = match role.as_str() {
                "viewer" => doxyde_core::models::permission::SiteRole::Viewer,
                "editor" => doxyde_core::models::permission::SiteRole::Editor,
                "owner" => doxyde_core::models::permission::SiteRole::Owner,
                _ => anyhow::bail!("Invalid role. Must be: viewer, editor, or owner"),
            };

            // Create site-user relationship
            let site_user_repo = SiteUserRepository::new(pool);
            let site_id = found_site.id.ok_or_else(|| anyhow!("Site has no ID"))?;
            let user_id = found_user.id.ok_or_else(|| anyhow!("User has no ID"))?;
            let site_user =
                doxyde_core::models::permission::SiteUser::new(site_id, user_id, site_role);

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
