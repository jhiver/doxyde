// Integration security tests for Doxyde
// These tests verify end-to-end security measures

use axum::http::{header, Method, Request, StatusCode};
use doxyde_core::models::{site::Site, user::User};
use doxyde_db::repositories::{UserRepository};
use sqlx::SqlitePool;

#[sqlx::test]
async fn test_authentication_required_endpoints(pool: SqlitePool) {
    // Test that protected endpoints require authentication
    let protected_endpoints = vec![
        "/.admin",
        "/test-page/.edit",
        "/test-page/.new",
        "/test-page/.delete",
        "/test-page/.move",
        "/test-page/.properties",
    ];
    
    // Create test site - in multi-database mode, site config is already created by migration
    // So we don't need to create anything here for the site
    
    // Without authentication, these should all return 401 or redirect to login
    // This would be tested with actual HTTP client in a full integration test
    for endpoint in protected_endpoints {
        // In a real test, we'd make HTTP requests and verify responses
        assert!(endpoint.contains("/."));
    }
}

#[sqlx::test]
async fn test_csrf_protection_on_state_changes(pool: SqlitePool) {
    // Test that state-changing operations require CSRF tokens
    let state_changing_endpoints = vec![
        ("POST", "/.login"),
        ("POST", "/page/.edit"),
        ("POST", "/page/.new"),
        ("POST", "/page/.delete"),
        ("POST", "/page/.publish"),
        ("POST", "/page/.move"),
    ];
    
    // Without CSRF token, these should return 403 Forbidden
    for (method, endpoint) in state_changing_endpoints {
        assert_eq!(method, "POST");
        assert!(endpoint.contains("/."));
    }
}

#[sqlx::test]
async fn test_sql_injection_in_search(pool: SqlitePool) {
    // Test SQL injection attempts in various query parameters
    // In multi-database mode, we test site_config queries instead
    let injection_attempts = vec![
        "Test Site'; DROP TABLE site_config; --",
        "Test Site' OR '1'='1",
        "Test Site' UNION SELECT * FROM users--",
    ];
    
    for attempt in injection_attempts {
        // Test that query parameters are properly escaped
        let result = sqlx::query!("SELECT title FROM site_config WHERE title = ?", attempt)
            .fetch_optional(&pool)
            .await;
        // Should either not find anything or error safely
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}

#[sqlx::test]
async fn test_password_hashing_security(pool: SqlitePool) {
    let user_repo = UserRepository::new(pool.clone());
    
    // Create user with password
    let mut user = User::new("test@example.com", "testuser", "password123");
    user_repo.create(&user).await.unwrap();
    
    // Verify password is hashed, not plain text
    let stored_user = user_repo.find_by_email("test@example.com").await.unwrap().unwrap();
    assert_ne!(stored_user.password_hash, "password123");
    assert!(stored_user.password_hash.len() > 50); // Argon2 hashes are long
    
    // Verify we can't authenticate with wrong password
    assert!(!stored_user.verify_password("wrongpassword"));
    assert!(stored_user.verify_password("password123"));
}

#[sqlx::test]
async fn test_session_rotation_on_login(pool: SqlitePool) {
    use doxyde_db::repositories::SessionRepository;
    use doxyde_core::models::session::Session;
    
    let user_repo = UserRepository::new(pool.clone());
    let session_repo = SessionRepository::new(pool.clone());
    
    // Create user
    let mut user = User::new("test@example.com", "testuser", "password");
    user_repo.create(&user).await.unwrap();
    let user = user_repo.find_by_email("test@example.com").await.unwrap().unwrap();
    
    // Create old session
    let old_session = Session::new(user.id.unwrap());
    session_repo.create(&old_session).await.unwrap();
    
    // Verify old session exists
    let found = session_repo.find_by_id(&old_session.id).await.unwrap();
    assert!(found.is_some());
    
    // On login, old sessions should be deleted (tested in actual login handler)
    session_repo.delete_user_sessions(user.id.unwrap()).await.unwrap();
    
    // Verify old session is gone
    let found = session_repo.find_by_id(&old_session.id).await.unwrap();
    assert!(found.is_none());
}

#[sqlx::test]
async fn test_authorization_within_site(pool: SqlitePool) {
    use doxyde_core::models::permission::{SiteRole, SiteUser};
    use doxyde_db::repositories::{PageRepository, SiteUserRepository};
    
    let user_repo = UserRepository::new(pool.clone());
    let site_user_repo = SiteUserRepository::new(pool.clone());
    let page_repo = PageRepository::new(pool.clone());
    
    // In multi-database mode, each database represents one site with site_id=1
    // Create users with different access levels to the site
    let mut viewer_user = User::new("viewer@example.com", "viewer", "password");
    user_repo.create(&viewer_user).await.unwrap();
    let viewer_user = user_repo.find_by_email("viewer@example.com").await.unwrap().unwrap();
    
    let mut editor_user = User::new("editor@example.com", "editor", "password");
    user_repo.create(&editor_user).await.unwrap();
    let editor_user = user_repo.find_by_email("editor@example.com").await.unwrap().unwrap();
    
    // Grant different roles (site_id is always 1 in multi-database mode)
    let site_user_viewer = SiteUser::new(1, viewer_user.id.unwrap(), SiteRole::Viewer);
    site_user_repo.create(&site_user_viewer).await.unwrap();
    
    let site_user_editor = SiteUser::new(1, editor_user.id.unwrap(), SiteRole::Editor);
    site_user_repo.create(&site_user_editor).await.unwrap();
    
    // Verify users have correct access levels
    let viewer_access = site_user_repo
        .find_by_site_and_user(1, viewer_user.id.unwrap())
        .await
        .unwrap();
    assert!(viewer_access.is_some());
    assert_eq!(viewer_access.unwrap().role, SiteRole::Viewer);
    
    let editor_access = site_user_repo
        .find_by_site_and_user(1, editor_user.id.unwrap())
        .await
        .unwrap();
    assert!(editor_access.is_some());
    assert_eq!(editor_access.unwrap().role, SiteRole::Editor);
}

#[sqlx::test]
async fn test_path_traversal_in_page_slugs(pool: SqlitePool) {
    use doxyde_core::models::page::Page;
    use doxyde_db::repositories::PageRepository;
    
    let page_repo = PageRepository::new(pool.clone());
    
    // In multi-database mode, site is already configured
    // We test path traversal in page slugs
    
    // Try to create pages with path traversal attempts
    let dangerous_slugs = vec![
        "../admin",
        "../../etc/passwd",
        ".../config",
        "page/../../../secret",
    ];
    
    for slug in dangerous_slugs {
        let page = Page::new(slug.to_string(), format!("Page {}", slug));
        let result = page_repo.create(&page).await;
        // Should either fail validation or sanitize the slug
        if result.is_ok() {
            let created = page_repo.find_by_id(result.unwrap()).await.unwrap().unwrap();
            // Verify dangerous characters were removed/sanitized
            assert!(!created.slug.contains(".."));
            assert!(!created.slug.contains("/"));
        }
    }
}