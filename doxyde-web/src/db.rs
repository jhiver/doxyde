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
use std::path::Path;

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

async fn check_and_run_migrations(pool: &SqlitePool) -> Result<()> {
    // Create version table if it doesn't exist
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS _schema_version (
            version TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(pool)
    .await
    .context("Failed to create schema version table")?;

    // Get the last applied version
    let last_version: Option<String> =
        sqlx::query_scalar("SELECT version FROM _schema_version ORDER BY version DESC LIMIT 1")
            .fetch_optional(pool)
            .await
            .context("Failed to query schema version")?;

    let expected_version = "20250712_add_draft_support";

    if last_version.as_deref() != Some(expected_version) {
        tracing::info!("Running migrations...");

        // Use sqlx::migrate! macro to run all migrations
        sqlx::migrate!("../migrations")
            .run(pool)
            .await
            .context("Failed to run migrations")?;

        // Record the new version
        sqlx::query("INSERT OR REPLACE INTO _schema_version (version) VALUES (?)")
            .bind(expected_version)
            .execute(pool)
            .await
            .context("Failed to update schema version")?;

        tracing::info!("Migrations complete, now at version: {}", expected_version);
    } else {
        tracing::info!(
            "Database schema is up to date (version: {})",
            expected_version
        );
    }

    Ok(())
}
