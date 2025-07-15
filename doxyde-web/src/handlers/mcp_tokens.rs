use anyhow::Result;
use axum::{
    extract::{Path, State},
    response::{Html, IntoResponse, Redirect},
    Form,
};
use doxyde_core::models::McpToken;
use doxyde_db::repositories::McpTokenRepository;
use serde::Deserialize;
use tera::Context;

use crate::{
    auth::CurrentUser,
    error::AppError,
    state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct CreateTokenForm {
    pub name: String,
    pub site_id: i64,
}

pub async fn list_tokens_handler(
    State(state): State<AppState>,
    user: CurrentUser,
) -> Result<impl IntoResponse, AppError> {
    let token_repo = McpTokenRepository::new(state.db.clone());
    let tokens = token_repo.find_by_user(user.user.id.unwrap()).await?;

    // Get site information for each token
    let site_repo = doxyde_db::repositories::SiteRepository::new(state.db.clone());
    let mut token_sites = Vec::new();
    
    for token in &tokens {
        if let Ok(Some(site)) = site_repo.find_by_id(token.site_id).await {
            token_sites.push((token.clone(), site));
        }
    }

    let mut context = Context::new();
    context.insert("user", &user.user);
    context.insert("tokens", &token_sites);
    
    // Get sites the user has access to for the creation form
    let site_user_repo = doxyde_db::repositories::SiteUserRepository::new(state.db.clone());
    let site_users = site_user_repo.list_by_user(user.user.id.unwrap()).await?;
    
    // Get site details for each SiteUser
    let mut user_sites = Vec::new();
    for su in site_users {
        if let Ok(Some(site)) = site_repo.find_by_id(su.site_id).await {
            user_sites.push((site, su.role));
        }
    }
    context.insert("sites", &user_sites);
    
    // Debug: log context keys
    tracing::debug!("Template context keys: user, tokens, sites");
    tracing::debug!("Number of tokens: {}", token_sites.len());
    tracing::debug!("Number of sites: {}", user_sites.len());

    let html = match state.templates.render("mcp/list.html", &context) {
        Ok(html) => html,
        Err(e) => {
            tracing::error!("Template render error: {:?}", e);
            tracing::error!("Error source: {:?}", e.source());
            return Err(AppError::internal_server_error(format!("Template error: {}", e)));
        }
    };

    Ok(Html(html))
}

pub async fn create_token_handler(
    State(state): State<AppState>,
    user: CurrentUser,
    Form(form): Form<CreateTokenForm>,
) -> Result<impl IntoResponse, AppError> {
    // Validate the user has access to the site
    let site_user_repo = doxyde_db::repositories::SiteUserRepository::new(state.db.clone());
    let site_users = site_user_repo.list_by_user(user.user.id.unwrap()).await?;
    
    if !site_users.iter().any(|su| su.site_id == form.site_id) {
        return Err(AppError::not_found("Not found"));
    }

    // Validate token name
    McpToken::validate_name(&form.name)
        .map_err(|e| AppError::bad_request(e.to_string()))?;

    // Create the token
    let token = McpToken::new(user.user.id.unwrap(), form.site_id, form.name);
    let token_id = token.id.clone();
    
    let token_repo = McpTokenRepository::new(state.db.clone());
    token_repo.create(&token).await?;

    // Redirect to show the newly created token
    Ok(Redirect::to(&format!("/.settings/mcp/{}", token_id)))
}

pub async fn show_token_handler(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(token_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let token_repo = McpTokenRepository::new(state.db.clone());
    let token = token_repo.find_by_id(&token_id).await?
        .ok_or(AppError::not_found("Not found"))?;

    // Check if the token belongs to the current user
    if token.user_id != user.user.id.unwrap() {
        return Err(AppError::not_found("Not found"));
    }

    // Get site information
    let site_repo = doxyde_db::repositories::SiteRepository::new(state.db.clone());
    let site = site_repo.find_by_id(token.site_id).await?
        .ok_or(AppError::not_found("Not found"))?;

    let mut context = Context::new();
    context.insert("user", &user.user);
    context.insert("token", &token);
    context.insert("site", &site);
    
    // Generate the MCP URL
    let mcp_url = format!("{}/.mcp/{}", site.domain, token.id);
    context.insert("mcp_url", &mcp_url);

    let html = state
        .templates
        .render("mcp/show.html", &context)
        .map_err(|e| {
            tracing::error!("Failed to render mcp/show.html: {}", e);
            AppError::internal_server_error(format!("Failed to render 'mcp/show.html': {}", e))
        })?;

    Ok(Html(html))
}

pub async fn revoke_token_handler(
    State(state): State<AppState>,
    user: CurrentUser,
    Path(token_id): Path<String>,
) -> Result<impl IntoResponse, AppError> {
    let token_repo = McpTokenRepository::new(state.db.clone());
    
    // Verify token exists and belongs to user
    let token = token_repo.find_by_id(&token_id).await?
        .ok_or(AppError::not_found("Not found"))?;
    
    if token.user_id != user.user.id.unwrap() {
        return Err(AppError::not_found("Not found"));
    }

    // Revoke the token
    token_repo.revoke(&token_id).await?;

    Ok(Redirect::to("/.settings/mcp"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{create_test_app_state, create_test_user, create_test_site};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_list_tokens_empty() -> Result<()> {
        let state = create_test_app_state().await?;
        let pool = state.db.clone();
        let user = create_test_user(&pool, "testuser", "test@example.com", false).await?;
        
        // Create session for user
        let session_repo = doxyde_db::repositories::SessionRepository::new(pool.clone());
        let session = doxyde_core::models::Session::new(user.id.unwrap());
        session_repo.create(&session).await?;
        
        let app = crate::routes::create_router(state);
        
        let request = Request::builder()
            .uri("/.settings/mcp")
            .header("cookie", format!("session_id={}", session.id))
            .body(Body::empty())?;
            
        let response = app.oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::OK);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_create_and_show_token() -> Result<()> {
        let state = create_test_app_state().await?;
        let pool = state.db.clone();
        let user = create_test_user(&pool, "testuser", "test@example.com", false).await?;
        let site = create_test_site(&pool, "example.com", "Example Site").await?;
        
        // Grant user access to site
        let site_user_repo = doxyde_db::repositories::SiteUserRepository::new(pool.clone());
        let site_user = doxyde_core::models::permission::SiteUser {
            site_id: site.id.unwrap(),
            user_id: user.id.unwrap(),
            role: doxyde_core::models::permission::SiteRole::Owner,
            created_at: chrono::Utc::now(),
        };
        site_user_repo.create(&site_user).await?;
        
        // Create session for user
        let session_repo = doxyde_db::repositories::SessionRepository::new(pool.clone());
        let session = doxyde_core::models::Session::new(user.id.unwrap());
        session_repo.create(&session).await?;
        
        let app = crate::routes::create_router(state.clone());
        
        // Create token
        let request = Request::builder()
            .method("POST")
            .uri("/.settings/mcp")
            .header("cookie", format!("session_id={}", session.id))
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(format!("name=Test+Token&site_id={}", site.id.unwrap())))?;
            
        let response = app.clone().oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
        
        // Extract token ID from redirect
        let location = response.headers().get("location").unwrap().to_str()?;
        let token_id = location.trim_start_matches("/.settings/mcp/");
        
        // Show token
        let request = Request::builder()
            .uri(&format!("/.settings/mcp/{}", token_id))
            .header("cookie", format!("session_id={}", session.id))
            .body(Body::empty())?;
            
        let response = app.oneshot(request).await?;
        assert_eq!(response.status(), StatusCode::OK);
        
        Ok(())
    }
}