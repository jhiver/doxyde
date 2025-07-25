use serde_json::json;

use crate::{
    routes::create_router,
    test_helpers::{create_test_app_state, create_test_user},
    AppState,
};
use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
    Router,
};
use doxyde_core::models::mcp_token::McpToken;
use tower::ServiceExt;

async fn create_test_app(pool: sqlx::SqlitePool) -> anyhow::Result<Router> {
    let state = create_test_app_state().await?;
    let state = AppState { db: pool, ..state };
    Ok(create_router(state))
}

async fn login_as_user(app: &Router, username: &str, password: &str) -> anyhow::Result<String> {
    let login_data =
        urlencoding::Encoded::new(format!("username={}&password={}", username, password));

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/.login")
                .method("POST")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(Body::from(login_data.to_string()))?,
        )
        .await?;

    // Extract session cookie from response
    let cookie_header = response
        .headers()
        .get(header::SET_COOKIE)
        .ok_or_else(|| anyhow::anyhow!("No cookie in response"))?
        .to_str()?;

    Ok(cookie_header.to_string())
}

#[sqlx::test]
async fn test_oauth_discovery_endpoints(pool: sqlx::SqlitePool) -> anyhow::Result<()> {
    let app = create_test_app(pool.clone()).await?;

    // Test .well-known/oauth-authorization-server
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/.well-known/oauth-authorization-server")
                .header("Host", "localhost:3000")
                .body(Body::empty())?,
        )
        .await?;

    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await?;

    if status != StatusCode::OK {
        let error_text = String::from_utf8_lossy(&body);
        panic!("Expected 200 OK but got {}: {}", status, error_text);
    }
    let metadata: serde_json::Value = serde_json::from_slice(&body)?;

    assert_eq!(metadata["issuer"], "https://localhost:3000");
    assert_eq!(
        metadata["authorization_endpoint"],
        "https://localhost:3000/.oauth/authorize"
    );
    assert_eq!(
        metadata["token_endpoint"],
        "https://localhost:3000/.oauth/token"
    );
    assert_eq!(
        metadata["registration_endpoint"],
        "https://localhost:3000/.oauth/register"
    );

    Ok(())
}

#[sqlx::test]
async fn test_dynamic_client_registration(pool: sqlx::SqlitePool) -> anyhow::Result<()> {
    let app = create_test_app(pool.clone()).await?;

    // Create test user and login
    let user = create_test_user(&pool, "testuser", "test@example.com", false).await?;
    let session_cookie = login_as_user(&app, "testuser", "password123").await?;

    // Create MCP token
    let mcp_token_repo = doxyde_db::repositories::McpTokenRepository::new(pool.clone());
    let site_repo = doxyde_db::repositories::SiteRepository::new(pool.clone());

    let site = site_repo.find_by_domain("localhost:3000").await?.unwrap();

    let mcp_token = McpToken::new(user.id.unwrap(), site.id.unwrap(), "Test Token".to_string());

    mcp_token_repo.create(&mcp_token).await?;

    // Register OAuth client
    let registration_request = json!({
        "client_name": "Test OAuth Client",
        "redirect_uris": ["http://localhost:8080/callback", "claude://callback"],
        "grant_types": ["authorization_code", "refresh_token"],
        "response_types": ["code"],
        "scope": "mcp:read mcp:write",
        "token_endpoint_auth_method": "client_secret_basic"
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/.oauth/register")
                .method("POST")
                .header("Content-Type", "application/json")
                .header("Cookie", session_cookie)
                .body(Body::from(serde_json::to_string(&registration_request)?))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = to_bytes(response.into_body(), usize::MAX).await?;
    let registration_response: serde_json::Value = serde_json::from_slice(&body)?;

    assert!(registration_response["client_id"].is_string());
    assert!(registration_response["client_secret"].is_string());
    assert_eq!(registration_response["client_name"], "Test OAuth Client");

    Ok(())
}

#[sqlx::test]
async fn test_authorization_flow_with_pkce(pool: sqlx::SqlitePool) -> anyhow::Result<()> {
    let app = create_test_app(pool.clone()).await?;

    // Setup: Create user, login, create MCP token
    let user = create_test_user(&pool, "testuser", "test@example.com", false).await?;
    let session_cookie = login_as_user(&app, "testuser", "password123").await?;

    let mcp_token_repo = doxyde_db::repositories::McpTokenRepository::new(pool.clone());
    let site_repo = doxyde_db::repositories::SiteRepository::new(pool.clone());

    let site = site_repo.find_by_domain("localhost:3000").await?.unwrap();

    let mcp_token = McpToken::new(user.id.unwrap(), site.id.unwrap(), "Test Token".to_string());

    mcp_token_repo.create(&mcp_token).await?;

    // Register OAuth client
    let oauth_repo = doxyde_db::repositories::OAuthClientRepository::new(pool.clone());
    let oauth_client = doxyde_db::repositories::OAuthClient {
        client_id: "test-client".to_string(),
        client_secret_hash: None,
        client_name: "Test Client".to_string(),
        redirect_uris: vec!["http://localhost:8080/callback".to_string()],
        grant_types: vec!["authorization_code".to_string()],
        response_types: vec!["code".to_string()],
        scope: Some("mcp:read".to_string()),
        token_endpoint_auth_method: "none".to_string(),
        created_at: chrono::Utc::now(),
        created_by_token_id: Some(mcp_token.id.clone()),
    };

    oauth_repo.create(&oauth_client).await?;

    // Generate PKCE challenge
    let _code_verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    let code_challenge = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";

    // Test authorization endpoint
    let auth_url = format!(
        "/.oauth/authorize?client_id={}&redirect_uri={}&response_type=code&scope=mcp:read&state=xyz&code_challenge={}&code_challenge_method=S256",
        oauth_client.client_id,
        urlencoding::encode("http://localhost:8080/callback"),
        code_challenge
    );

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(&auth_url)
                .header("Cookie", session_cookie.clone())
                .body(Body::empty())?,
        )
        .await?;

    // Should show consent page
    assert_eq!(response.status(), StatusCode::OK);

    Ok(())
}

#[sqlx::test]
async fn test_token_exchange(pool: sqlx::SqlitePool) -> anyhow::Result<()> {
    let app = create_test_app(pool.clone()).await?;

    // Setup: Create authorization code
    let auth_repo = doxyde_db::repositories::AuthorizationCodeRepository::new(pool.clone());
    let oauth_repo = doxyde_db::repositories::OAuthClientRepository::new(pool.clone());

    // Create client
    let oauth_client = doxyde_db::repositories::OAuthClient {
        client_id: "test-client".to_string(),
        client_secret_hash: None,
        client_name: "Test Client".to_string(),
        redirect_uris: vec!["http://localhost:8080/callback".to_string()],
        grant_types: vec!["authorization_code".to_string()],
        response_types: vec!["code".to_string()],
        scope: Some("mcp:read".to_string()),
        token_endpoint_auth_method: "none".to_string(),
        created_at: chrono::Utc::now(),
        created_by_token_id: None,
    };

    oauth_repo.create(&oauth_client).await?;

    // Create authorization code
    let code = "test-auth-code";
    let auth_code = doxyde_db::repositories::AuthorizationCode {
        code: code.to_string(),
        client_id: oauth_client.client_id.clone(),
        user_id: 1,
        mcp_token_id: "test-token".to_string(),
        redirect_uri: "http://localhost:8080/callback".to_string(),
        scope: Some("mcp:read".to_string()),
        code_challenge: None,
        code_challenge_method: None,
        expires_at: chrono::Utc::now() + chrono::Duration::minutes(10),
        used_at: None,
        created_at: chrono::Utc::now(),
    };

    auth_repo.create(&auth_code).await?;

    // Exchange code for token
    let token_request = urlencoding::Encoded::new(format!(
        "grant_type=authorization_code&code={}&redirect_uri={}&client_id={}",
        code,
        urlencoding::encode("http://localhost:8080/callback"),
        oauth_client.client_id
    ));

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/.oauth/token")
                .method("POST")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(Body::from(token_request.to_string()))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await?;
    let token_response: serde_json::Value = serde_json::from_slice(&body)?;

    assert!(token_response["access_token"].is_string());
    assert_eq!(token_response["token_type"], "Bearer");
    assert!(token_response["expires_in"].is_i64());
    assert!(token_response["refresh_token"].is_string());

    Ok(())
}

#[sqlx::test]
async fn test_oauth_protected_mcp_endpoint(pool: sqlx::SqlitePool) -> anyhow::Result<()> {
    let app = create_test_app(pool.clone()).await?;

    // Setup: Create access token
    let token_repo = doxyde_db::repositories::AccessTokenRepository::new(pool.clone());
    let mcp_token_repo = doxyde_db::repositories::McpTokenRepository::new(pool.clone());
    let site_repo = doxyde_db::repositories::SiteRepository::new(pool.clone());

    let site = site_repo.find_by_domain("localhost:3000").await?.unwrap();

    // Create MCP token
    let mcp_token = McpToken::new(
        1, // user_id - just using a default for the test
        site.id.unwrap(),
        "Test Token".to_string(),
    );

    mcp_token_repo.create(&mcp_token).await?;

    // Create access token
    let access_token = "test-access-token";
    let token_hash = super::models::hash_token(access_token);

    let oauth_token = doxyde_db::repositories::AccessToken {
        token: access_token.to_string(),
        token_hash,
        client_id: "test-client".to_string(),
        user_id: 1,
        mcp_token_id: mcp_token.id.clone(),
        scope: Some("mcp:read".to_string()),
        expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
        created_at: chrono::Utc::now(),
    };

    token_repo.create(&oauth_token).await?;

    // Test MCP request with Bearer token
    let mcp_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "resources/list",
        "params": {}
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/.mcp")
                .method("POST")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", access_token))
                .body(Body::from(serde_json::to_string(&mcp_request)?))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await?;
    let mcp_response: serde_json::Value = serde_json::from_slice(&body)?;

    assert_eq!(mcp_response["jsonrpc"], "2.0");
    assert_eq!(mcp_response["id"], 1);
    assert!(mcp_response["result"].is_object());

    Ok(())
}

#[sqlx::test]
async fn test_invalid_bearer_token(pool: sqlx::SqlitePool) -> anyhow::Result<()> {
    let app = create_test_app(pool.clone()).await?;

    let mcp_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "resources/list",
        "params": {}
    });

    // Test with invalid token
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/.mcp")
                .method("POST")
                .header("Host", "localhost:3000")
                .header("Content-Type", "application/json")
                .header("Authorization", "Bearer invalid-token")
                .body(Body::from(serde_json::to_string(&mcp_request)?))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let body = to_bytes(response.into_body(), usize::MAX).await?;
    let error_response: serde_json::Value = serde_json::from_slice(&body)?;

    assert_eq!(error_response["error"], "invalid_token");

    Ok(())
}

#[sqlx::test]
async fn test_refresh_token_flow(pool: sqlx::SqlitePool) -> anyhow::Result<()> {
    let app = create_test_app(pool.clone()).await?;

    // Setup: Create refresh token
    let refresh_repo = doxyde_db::repositories::RefreshTokenRepository::new(pool.clone());
    let oauth_repo = doxyde_db::repositories::OAuthClientRepository::new(pool.clone());

    // Create client
    let oauth_client = doxyde_db::repositories::OAuthClient {
        client_id: "test-client".to_string(),
        client_secret_hash: None,
        client_name: "Test Client".to_string(),
        redirect_uris: vec!["http://localhost:8080/callback".to_string()],
        grant_types: vec!["refresh_token".to_string()],
        response_types: vec!["code".to_string()],
        scope: Some("mcp:read mcp:write".to_string()),
        token_endpoint_auth_method: "none".to_string(),
        created_at: chrono::Utc::now(),
        created_by_token_id: None,
    };

    oauth_repo.create(&oauth_client).await?;

    // Create refresh token
    let refresh_token = "test-refresh-token";
    let token_hash = super::models::hash_token(refresh_token);

    let oauth_refresh = doxyde_db::repositories::RefreshToken {
        token: refresh_token.to_string(),
        token_hash,
        client_id: oauth_client.client_id.clone(),
        user_id: 1,
        mcp_token_id: "test-token".to_string(),
        scope: Some("mcp:read mcp:write".to_string()),
        expires_at: chrono::Utc::now() + chrono::Duration::days(30),
        used_at: None,
        created_at: chrono::Utc::now(),
    };

    refresh_repo.create(&oauth_refresh).await?;

    // Use refresh token
    let token_request = urlencoding::Encoded::new(format!(
        "grant_type=refresh_token&refresh_token={}&client_id={}&scope=mcp:read",
        refresh_token, oauth_client.client_id
    ));

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/.oauth/token")
                .method("POST")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(Body::from(token_request.to_string()))?,
        )
        .await?;

    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await?;
    let token_response: serde_json::Value = serde_json::from_slice(&body)?;

    assert!(token_response["access_token"].is_string());
    assert!(token_response["refresh_token"].is_string());
    assert_eq!(token_response["scope"], "mcp:read"); // Should be reduced scope

    Ok(())
}
