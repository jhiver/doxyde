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

use anyhow::Result;
use axum::{
    extract::{Host, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect},
    Form,
};
use axum_extra::extract::{cookie::Cookie, CookieJar};
use doxyde_core::models::session::Session;
use doxyde_db::repositories::{SessionRepository, SiteRepository, UserRepository};
use serde::Deserialize;
use tera::Context;

use crate::{template_context::add_base_context, AppState};

/// Helper to create a login context with site-specific data
async fn create_login_context(state: &AppState, host: &str, error: Option<&str>) -> Context {
    let mut context = Context::new();

    // Add error if present
    if let Some(err) = error {
        context.insert("error", err);
    }

    // Try to resolve the site from the host
    let site_repo = SiteRepository::new(state.db.clone());
    if let Ok(Some(site)) = site_repo.find_by_domain(host).await {
        // Add base context with logo support
        if let Err(e) = add_base_context(&mut context, state, &site, None).await {
            tracing::warn!("Failed to add base context: {:?}", e);
            // Fall back to just site title
            context.insert("site_title", &site.title);
        }
    } else {
        // No site found, use default
        context.insert("site_title", "Doxyde");
    }

    context
}

#[derive(Debug, Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

/// Display login form
pub async fn login_form(
    Host(host): Host,
    State(state): State<AppState>,
) -> Result<Html<String>, StatusCode> {
    let context = create_login_context(&state, &host, None).await;

    let html = state
        .templates
        .render("login.html", &context)
        .map_err(|e| {
            tracing::error!("Failed to render login.html: {:?}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Html(html))
}

/// Handle login POST request
pub async fn login(
    Host(host): Host,
    State(state): State<AppState>,
    jar: CookieJar,
    Form(form): Form<LoginForm>,
) -> Result<impl IntoResponse, StatusCode> {
    // Find user by username or email
    let user_repo = UserRepository::new(state.db.clone());

    let user = if form.username.contains('@') {
        user_repo
            .find_by_email(&form.username)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        user_repo
            .find_by_username(&form.username)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    };

    // Verify user exists and is active
    let user = match user {
        Some(u) if u.is_active => u,
        Some(_) => {
            // Account disabled
            let context = create_login_context(&state, &host, Some("Account is disabled")).await;
            let html = state
                .templates
                .render("login.html", &context)
                .map_err(|e| {
                    tracing::error!("Failed to render login.html: {:?}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
            return Ok((jar, Html(html)).into_response());
        }
        None => {
            // User not found
            let context =
                create_login_context(&state, &host, Some("Invalid username or password")).await;
            let html = state
                .templates
                .render("login.html", &context)
                .map_err(|e| {
                    tracing::error!("Failed to render login.html: {:?}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
            return Ok((jar, Html(html)).into_response());
        }
    };

    // Verify password
    match user.verify_password(&form.password) {
        Ok(true) => {} // Password is correct, continue
        Ok(false) => {
            // Return to login form with error message
            let context =
                create_login_context(&state, &host, Some("Invalid username or password")).await;
            let html = state
                .templates
                .render("login.html", &context)
                .map_err(|e| {
                    tracing::error!("Failed to render login.html: {:?}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
            return Ok((jar, Html(html)).into_response());
        }
        Err(e) => {
            // Check if it's a disabled password
            if e.to_string().contains("Password disabled") {
                let context =
                    create_login_context(&state, &host, Some("Account is disabled")).await;
                let html = state
                    .templates
                    .render("login.html", &context)
                    .map_err(|e| {
                        tracing::error!("Failed to render login.html: {:?}", e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?;
                return Ok((jar, Html(html)).into_response());
            }
            // Other errors (invalid hash format, etc.) are server errors
            tracing::error!("Password verification error: {:?}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    // Create session
    let session = Session::new(user.id.unwrap());
    let session_id = session.id.clone();

    let session_repo = SessionRepository::new(state.db.clone());
    session_repo
        .create(&session)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Set session cookie
    let cookie = Cookie::build(("session_id", session_id))
        .path("/")
        .http_only(true)
        .same_site(cookie::SameSite::Lax)
        .build();

    Ok((jar.add(cookie), Redirect::to("/")).into_response())
}

/// Handle logout
pub async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<impl IntoResponse, StatusCode> {
    // Get session ID from cookie
    if let Some(session_cookie) = jar.get("session_id") {
        let session_id = session_cookie.value();

        // Delete session from database
        let session_repo = SessionRepository::new(state.db.clone());
        let _ = session_repo.delete(session_id).await; // Ignore errors
    }

    // Remove session cookie
    let jar = jar.remove("session_id");

    Ok((jar, Redirect::to("/.login")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{templates::init_templates, AppState};
    use doxyde_core::models::user::User;
    use sqlx::SqlitePool;

    async fn create_test_db() -> Result<SqlitePool> {
        let pool = SqlitePool::connect(":memory:").await?;

        // Run migrations inline for tests
        sqlx::query(
            r#"
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
            "#,
        )
        .execute(&pool)
        .await?;

        Ok(pool)
    }

    #[tokio::test]
    async fn test_login_form_renders() -> Result<()> {
        let pool = create_test_db().await?;
        let templates = init_templates("templates", false)?;
        let config = crate::config::Config {
            database_url: "sqlite::memory:".to_string(),
            host: "localhost".to_string(),
            port: 3000,
            templates_dir: "templates".to_string(),
            session_secret: "test-secret".to_string(),
            development_mode: false,
            uploads_dir: "/tmp/test-uploads".to_string(),
            max_upload_size: 1048576,
        };
        let state = AppState::new(pool, templates, config);

        let response = login_form(Host("localhost:3000".to_string()), State(state)).await;
        assert!(response.is_ok());

        Ok(())
    }

    #[tokio::test]
    async fn test_login_success() -> Result<()> {
        let pool = create_test_db().await?;
        let templates = init_templates("templates", false)?;
        let config = crate::config::Config {
            database_url: "sqlite::memory:".to_string(),
            host: "localhost".to_string(),
            port: 3000,
            templates_dir: "templates".to_string(),
            session_secret: "test-secret".to_string(),
            development_mode: false,
            uploads_dir: "/tmp/test-uploads".to_string(),
            max_upload_size: 1048576,
        };
        let state = AppState::new(pool.clone(), templates, config);

        // Create test user
        let user_repo = UserRepository::new(pool.clone());
        let user = User::new(
            "test@example.com".to_string(),
            "testuser".to_string(),
            "password123",
        )?;
        let user_id = user_repo.create(&user).await?;

        // Test login
        let form = LoginForm {
            username: "testuser".to_string(),
            password: "password123".to_string(),
        };

        let jar = CookieJar::new();
        let response = login(
            Host("localhost:3000".to_string()),
            State(state),
            jar,
            Form(form),
        )
        .await;
        assert!(response.is_ok());

        // Verify session was created
        let session_repo = SessionRepository::new(pool);
        let sessions = session_repo.find_by_user_id(user_id).await?;
        assert_eq!(sessions.len(), 1);

        Ok(())
    }

    #[tokio::test]
    async fn test_login_with_email() -> Result<()> {
        let pool = create_test_db().await?;
        let templates = init_templates("templates", false)?;
        let config = crate::config::Config {
            database_url: "sqlite::memory:".to_string(),
            host: "localhost".to_string(),
            port: 3000,
            templates_dir: "templates".to_string(),
            session_secret: "test-secret".to_string(),
            development_mode: false,
            uploads_dir: "/tmp/test-uploads".to_string(),
            max_upload_size: 1048576,
        };
        let state = AppState::new(pool.clone(), templates, config);

        // Create test user
        let user_repo = UserRepository::new(pool.clone());
        let user = User::new(
            "test@example.com".to_string(),
            "testuser".to_string(),
            "password123",
        )?;
        user_repo.create(&user).await?;

        // Test login with email
        let form = LoginForm {
            username: "test@example.com".to_string(),
            password: "password123".to_string(),
        };

        let jar = CookieJar::new();
        let response = login(
            Host("localhost:3000".to_string()),
            State(state),
            jar,
            Form(form),
        )
        .await;
        assert!(response.is_ok());

        Ok(())
    }

    #[tokio::test]
    async fn test_login_invalid_password() -> Result<()> {
        let pool = create_test_db().await?;
        let templates = init_templates("templates", false)?;
        let config = crate::config::Config {
            database_url: "sqlite::memory:".to_string(),
            host: "localhost".to_string(),
            port: 3000,
            templates_dir: "templates".to_string(),
            session_secret: "test-secret".to_string(),
            development_mode: false,
            uploads_dir: "/tmp/test-uploads".to_string(),
            max_upload_size: 1048576,
        };
        let state = AppState::new(pool.clone(), templates, config);

        // Create test user
        let user_repo = UserRepository::new(pool.clone());
        let user = User::new(
            "test@example.com".to_string(),
            "testuser".to_string(),
            "password123",
        )?;
        let user_id = user_repo.create(&user).await?;

        // Test login with wrong password
        let form = LoginForm {
            username: "testuser".to_string(),
            password: "wrongpassword".to_string(),
        };

        let jar = CookieJar::new();
        let response = login(
            Host("localhost:3000".to_string()),
            State(state),
            jar,
            Form(form),
        )
        .await;
        assert!(response.is_ok());

        // Verify no session was created
        let session_repo = SessionRepository::new(pool);
        let sessions = session_repo.find_by_user_id(user_id).await?;
        assert_eq!(sessions.len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_login_nonexistent_user() -> Result<()> {
        let pool = create_test_db().await?;
        let templates = init_templates("templates", false)?;
        let config = crate::config::Config {
            database_url: "sqlite::memory:".to_string(),
            host: "localhost".to_string(),
            port: 3000,
            templates_dir: "templates".to_string(),
            session_secret: "test-secret".to_string(),
            development_mode: false,
            uploads_dir: "/tmp/test-uploads".to_string(),
            max_upload_size: 1048576,
        };
        let state = AppState::new(pool.clone(), templates, config);

        let form = LoginForm {
            username: "nonexistent".to_string(),
            password: "password123".to_string(),
        };

        let jar = CookieJar::new();
        let response = login(
            Host("localhost:3000".to_string()),
            State(state),
            jar,
            Form(form),
        )
        .await;
        assert!(response.is_ok());

        // The response should be an HTML page with an error (not a redirect)
        // Since there's no user to create a session for, we can't check sessions

        Ok(())
    }

    #[tokio::test]
    async fn test_login_inactive_user() -> Result<()> {
        let pool = create_test_db().await?;
        let templates = init_templates("templates", false)?;
        let config = crate::config::Config {
            database_url: "sqlite::memory:".to_string(),
            host: "localhost".to_string(),
            port: 3000,
            templates_dir: "templates".to_string(),
            session_secret: "test-secret".to_string(),
            development_mode: false,
            uploads_dir: "/tmp/test-uploads".to_string(),
            max_upload_size: 1048576,
        };
        let state = AppState::new(pool.clone(), templates, config);

        // Create inactive user
        let user_repo = UserRepository::new(pool.clone());
        let mut user = User::new(
            "test@example.com".to_string(),
            "testuser".to_string(),
            "password123",
        )?;
        user.is_active = false;
        let user_id = user_repo.create(&user).await?;

        let form = LoginForm {
            username: "testuser".to_string(),
            password: "password123".to_string(),
        };

        let jar = CookieJar::new();
        let response = login(
            Host("localhost:3000".to_string()),
            State(state),
            jar,
            Form(form),
        )
        .await;
        assert!(response.is_ok());

        // Verify no session was created for inactive user
        let session_repo = SessionRepository::new(pool);
        let sessions = session_repo.find_by_user_id(user_id).await?;
        assert_eq!(sessions.len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_login_starred_password() -> Result<()> {
        let pool = create_test_db().await?;
        let templates = init_templates("templates", false)?;
        let config = crate::config::Config {
            database_url: "sqlite::memory:".to_string(),
            host: "localhost".to_string(),
            port: 3000,
            templates_dir: "templates".to_string(),
            session_secret: "test-secret".to_string(),
            development_mode: false,
            uploads_dir: "/tmp/test-uploads".to_string(),
            max_upload_size: 1048576,
        };
        let state = AppState::new(pool.clone(), templates, config);

        // Create user with starred password
        let user_repo = UserRepository::new(pool.clone());
        let user = User::new(
            "test@example.com".to_string(),
            "testuser".to_string(),
            "password123",
        )?;
        let user_id = user_repo.create(&user).await?;

        // Star the password in the database
        let starred_hash = format!("*{}", user.password_hash);
        sqlx::query!(
            "UPDATE users SET password_hash = ? WHERE id = ?",
            starred_hash,
            user_id
        )
        .execute(&pool)
        .await?;

        let form = LoginForm {
            username: "testuser".to_string(),
            password: "password123".to_string(),
        };

        let jar = CookieJar::new();
        let response = login(
            Host("localhost:3000".to_string()),
            State(state),
            jar,
            Form(form),
        )
        .await;
        assert!(response.is_ok());

        // Verify no session was created
        let session_repo = SessionRepository::new(pool);
        let sessions = session_repo.find_by_user_id(user_id).await?;
        assert_eq!(sessions.len(), 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_logout() -> Result<()> {
        let pool = create_test_db().await?;
        let templates = init_templates("templates", false)?;
        let config = crate::config::Config {
            database_url: "sqlite::memory:".to_string(),
            host: "localhost".to_string(),
            port: 3000,
            templates_dir: "templates".to_string(),
            session_secret: "test-secret".to_string(),
            development_mode: false,
            uploads_dir: "/tmp/test-uploads".to_string(),
            max_upload_size: 1048576,
        };
        let state = AppState::new(pool.clone(), templates, config);

        // Create test session
        let user_repo = UserRepository::new(pool.clone());
        let user = User::new(
            "test@example.com".to_string(),
            "testuser".to_string(),
            "password123",
        )?;
        let user_id = user_repo.create(&user).await?;

        let session = Session::new(user_id);
        let session_id = session.id.clone();
        let session_repo = SessionRepository::new(pool.clone());
        session_repo.create(&session).await?;

        // Test logout
        let jar = CookieJar::new();
        let cookie = Cookie::build(("session_id", session_id.clone()))
            .path("/")
            .build();
        let jar = jar.add(cookie);

        let response = logout(State(state), jar).await;
        assert!(response.is_ok());

        // Verify session was deleted
        let found = session_repo.find_by_id(&session_id).await?;
        assert!(found.is_none());

        Ok(())
    }
}
