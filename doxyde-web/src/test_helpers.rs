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

#[cfg(test)]
use crate::{autoreload_templates::TemplateEngine, AppState};
#[cfg(test)]
use doxyde_core::models::{session::Session, site::Site, user::User};
#[cfg(test)]
use doxyde_db::repositories::{SessionRepository, SiteRepository, UserRepository};
#[cfg(test)]
use sqlx::SqlitePool;
#[cfg(test)]
use std::sync::Arc;

#[cfg(test)]
pub async fn create_test_app_state() -> Result<AppState, anyhow::Error> {
    // Create in-memory SQLite database
    let pool = SqlitePool::connect(":memory:").await?;

    // Create minimal schema for tests
    sqlx::query(
        r#"
        CREATE TABLE sites (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            domain TEXT NOT NULL UNIQUE,
            title TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        
        CREATE TABLE pages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            site_id INTEGER NOT NULL,
            parent_page_id INTEGER,
            slug TEXT NOT NULL,
            title TEXT NOT NULL,
            description TEXT,
            keywords TEXT,
            template TEXT DEFAULT 'default',
            meta_robots TEXT DEFAULT 'index,follow',
            canonical_url TEXT,
            og_image_url TEXT,
            structured_data_type TEXT DEFAULT 'WebPage',
            position INTEGER NOT NULL DEFAULT 0,
            sort_mode TEXT NOT NULL DEFAULT 'position',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (site_id) REFERENCES sites(id) ON DELETE CASCADE,
            FOREIGN KEY (parent_page_id) REFERENCES pages(id) ON DELETE CASCADE,
            UNIQUE(site_id, parent_page_id, slug)
        );
        
        CREATE INDEX idx_pages_site_id ON pages(site_id);
        CREATE INDEX idx_pages_parent_page_id ON pages(parent_page_id);
        
        CREATE TABLE users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            email TEXT NOT NULL UNIQUE,
            username TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1,
            is_admin INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        
        CREATE TABLE sessions (
            id TEXT PRIMARY KEY,
            user_id INTEGER NOT NULL,
            expires_at TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
        );
        
        CREATE TABLE site_users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            site_id INTEGER NOT NULL,
            user_id INTEGER NOT NULL,
            role TEXT NOT NULL CHECK (role IN ('owner', 'editor', 'viewer')),
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (site_id) REFERENCES sites(id) ON DELETE CASCADE,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
            UNIQUE(site_id, user_id)
        );

        CREATE TABLE page_versions (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            page_id INTEGER NOT NULL,
            version_number INTEGER NOT NULL,
            created_by TEXT,
            is_published BOOLEAN NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (page_id) REFERENCES pages(id) ON DELETE CASCADE,
            UNIQUE(page_id, version_number)
        );
        
        CREATE TABLE components (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            page_version_id INTEGER NOT NULL,
            component_type TEXT NOT NULL,
            position INTEGER NOT NULL,
            title TEXT,
            template TEXT DEFAULT 'default',
            content TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now')),
            FOREIGN KEY (page_version_id) REFERENCES page_versions(id) ON DELETE CASCADE
        );
        "#,
    )
    .execute(&pool)
    .await?;

    // Create templates
    let mut tera = tera::Tera::default();
    tera.add_raw_template(
        "page_move.html",
        r#"
        <!DOCTYPE html>
        <html>
        <head><title>Move Page</title></head>
        <body>
            <h1>Move {{ page.title }}</h1>
            <form method="post">
                <select name="target_parent_id">
                {% for target in targets %}
                    <option value="{{ target.id }}">{{ target.path }} - {{ target.title }}</option>
                {% endfor %}
                </select>
                <button type="submit">Move</button>
            </form>
        </body>
        </html>
    "#,
    )?;

    tera.add_raw_template(
        "page_delete.html",
        r#"
        <!DOCTYPE html>
        <html>
        <head><title>Delete Page</title></head>
        <body>
            <h1>Delete {{ page.title }}</h1>
            <p>This will permanently delete '{{ page.title }}' and all its versions.</p>
            <form method="post">
                <input type="text" name="confirm" placeholder="Type DELETE to confirm">
                <button type="submit">Delete</button>
            </form>
        </body>
        </html>
    "#,
    )?;


    // Create a test config
    let config = crate::config::Config {
        database_url: "sqlite::memory:".to_string(),
        host: "localhost".to_string(),
        port: 3000,
        templates_dir: "templates".to_string(),
        session_secret: "test-secret".to_string(),
        development_mode: false,
        uploads_dir: "/tmp/mkdoc-test-uploads".to_string(),
        max_upload_size: 1048576,      // 1MB for tests
        secure_cookies: false,         // Disable for tests
        session_timeout_minutes: 1440, // 24 hours
    };

    // Create rate limiters for tests
    let login_rate_limiter = crate::rate_limit::create_login_rate_limiter();
    let api_rate_limiter = crate::rate_limit::create_api_rate_limiter();

    Ok(AppState {
        db: pool,
        templates: TemplateEngine::Static(Arc::new(tera)),
        config,
        login_rate_limiter,
        api_rate_limiter,
    })
}

#[cfg(test)]
pub async fn create_test_user(
    pool: &SqlitePool,
    username: &str,
    email: &str,
    is_admin: bool,
) -> Result<User, anyhow::Error> {
    let user_repo = UserRepository::new(pool.clone());
    let mut user = User::new(email.to_string(), username.to_string(), "password123")?;
    user.is_admin = is_admin;

    let user_id = user_repo.create(&user).await?;
    user.id = Some(user_id);

    Ok(user)
}

#[cfg(test)]
pub async fn create_test_site(
    pool: &SqlitePool,
    domain: &str,
    title: &str,
) -> Result<Site, anyhow::Error> {
    let site_repo = SiteRepository::new(pool.clone());
    let site = Site::new(domain.to_string(), title.to_string());

    let site_id = site_repo.create(&site).await?;

    let site = site_repo.find_by_id(site_id).await?.unwrap();

    Ok(site)
}

#[cfg(test)]
pub async fn create_test_session(
    pool: &SqlitePool,
    user_id: i64,
) -> Result<Session, anyhow::Error> {
    let session_repo = SessionRepository::new(pool.clone());
    let session = Session::new(user_id);

    let _session_id = session_repo.create(&session).await?;
    let session = session_repo.find_by_id(&session.id).await?.unwrap();

    Ok(session)
}
