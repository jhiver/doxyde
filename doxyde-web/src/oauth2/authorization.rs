use anyhow::Result;
use axum::{
    extract::{Host, Query, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect},
    Form,
};
use serde::Deserialize;
use tera::Context;

use crate::{
    auth::{CurrentUser, OptionalUser},
    error::AppError,
    state::AppState,
};

use super::{errors::AuthorizationError, models::AuthorizationCode};

/// Authorization request parameters
#[derive(Debug, Deserialize)]
pub struct AuthorizationRequest {
    pub response_type: String,
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: Option<String>,
    pub state: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
}

/// Consent form submission
#[derive(Debug, Deserialize)]
pub struct ConsentForm {
    pub client_id: String,
    pub redirect_uri: String,
    pub scope: Option<String>,
    pub state: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
    pub mcp_token_id: String,
    pub action: String, // "approve" or "deny"
}

/// OAuth2 authorization endpoint
pub async fn authorization_handler(
    State(state): State<AppState>,
    Host(host): Host,
    headers: HeaderMap,
    user: OptionalUser,
    Query(params): Query<AuthorizationRequest>,
) -> Result<impl IntoResponse, AppError> {
    // If user is not authenticated, redirect to login
    if user.0.is_none() {
        let current_url = build_authorize_url(&host, &headers, &params);
        let login_url = format!("/.login?return_to={}", urlencoding::encode(&current_url));
        return Ok(Redirect::to(&login_url).into_response());
    }

    let user = user.0.unwrap();
    // Validate response_type
    if params.response_type != "code" {
        let error = AuthorizationError::unsupported_response_type(
            "Only 'code' response_type is supported",
            params.state,
        );
        return Ok(Redirect::to(&error.to_redirect_url(&params.redirect_uri)).into_response());
    }

    // Validate client
    let oauth_repo = doxyde_db::repositories::OAuthClientRepository::new(state.db.clone());
    let db_client = match oauth_repo.find_by_id(&params.client_id).await? {
        Some(client) => client,
        None => {
            let error = AuthorizationError::unauthorized_client("Unknown client_id", params.state);
            return Ok(Redirect::to(&error.to_redirect_url(&params.redirect_uri)).into_response());
        }
    };

    // Convert to web model
    let client = super::models::OAuthClient {
        client_id: db_client.client_id,
        client_secret_hash: db_client.client_secret_hash,
        client_name: db_client.client_name,
        redirect_uris: db_client.redirect_uris,
        grant_types: db_client.grant_types,
        response_types: db_client.response_types,
        scope: db_client.scope,
        token_endpoint_auth_method: db_client.token_endpoint_auth_method,
        created_at: db_client.created_at,
        created_by_token_id: db_client.created_by_token_id,
    };

    // Validate redirect_uri
    if !client.validate_redirect_uri(&params.redirect_uri) {
        // Can't redirect to invalid URI, show error page
        return Ok((
            StatusCode::BAD_REQUEST,
            Html("Invalid redirect_uri for this client"),
        )
            .into_response());
    }

    // Validate PKCE if present
    if let Some(ref method) = params.code_challenge_method {
        if method != "S256" {
            let error = AuthorizationError::invalid_request(
                "Only S256 code_challenge_method is supported",
                params.state,
            );
            return Ok(Redirect::to(&error.to_redirect_url(&params.redirect_uri)).into_response());
        }
        if params.code_challenge.is_none() {
            let error = AuthorizationError::invalid_request(
                "code_challenge is required when code_challenge_method is specified",
                params.state,
            );
            return Ok(Redirect::to(&error.to_redirect_url(&params.redirect_uri)).into_response());
        }
    }

    // Get user's MCP tokens for this site
    let token_repo = doxyde_db::repositories::McpTokenRepository::new(state.db.clone());
    let tokens = token_repo.find_by_user(user.user.id.unwrap()).await?;

    // Filter tokens that are valid and have sites
    let site_repo = doxyde_db::repositories::SiteRepository::new(state.db.clone());
    let mut token_sites = Vec::new();

    for token in tokens {
        if token.is_valid() {
            if let Ok(Some(site)) = site_repo.find_by_id(token.site_id).await {
                token_sites.push((token, site));
            }
        }
    }

    // Render consent page
    let mut context = Context::new();
    context.insert("user", &user.user);
    context.insert("client", &client);
    context.insert("tokens", &token_sites);
    context.insert("redirect_uri", &params.redirect_uri);
    context.insert("scope", &params.scope);
    context.insert("state", &params.state);
    context.insert("code_challenge", &params.code_challenge);
    context.insert("code_challenge_method", &params.code_challenge_method);

    let html = state
        .templates
        .render("oauth2/consent.html", &context)
        .map_err(|e| {
            tracing::error!("Failed to render oauth2/consent.html: {}", e);
            AppError::internal_server_error(format!("Template error: {}", e))
        })?;

    Ok(Html(html).into_response())
}

/// Handle consent form submission
pub async fn consent_handler(
    State(state): State<AppState>,
    user: CurrentUser,
    Form(form): Form<ConsentForm>,
) -> Result<impl IntoResponse, AppError> {
    // Check if user denied access
    if form.action == "deny" {
        let error = AuthorizationError::access_denied("User denied access", form.state);
        return Ok(Redirect::to(&error.to_redirect_url(&form.redirect_uri)));
    }

    // Validate client again
    let oauth_repo = doxyde_db::repositories::OAuthClientRepository::new(state.db.clone());
    let client = oauth_repo
        .find_by_id(&form.client_id)
        .await?
        .ok_or_else(|| AppError::bad_request("Invalid client_id"))?;

    // Validate redirect_uri again
    if !client.validate_redirect_uri(&form.redirect_uri) {
        return Err(AppError::bad_request("Invalid redirect_uri"));
    }

    // Validate MCP token belongs to user
    let token_repo = doxyde_db::repositories::McpTokenRepository::new(state.db.clone());
    let mcp_token = token_repo
        .find_by_id(&form.mcp_token_id)
        .await?
        .ok_or_else(|| AppError::bad_request("Invalid token"))?;

    if mcp_token.user_id != user.user.id.unwrap() || !mcp_token.is_valid() {
        return Err(AppError::forbidden("Access denied"));
    }

    // Create authorization code
    let auth_code = AuthorizationCode::new(
        form.client_id,
        user.user.id.unwrap(),
        form.mcp_token_id,
        form.redirect_uri.clone(),
        form.scope,
        form.code_challenge,
        form.code_challenge_method,
    );

    // Convert to DB model
    let db_auth_code = doxyde_db::repositories::AuthorizationCode {
        code: auth_code.code.clone(),
        client_id: auth_code.client_id,
        user_id: auth_code.user_id,
        mcp_token_id: auth_code.mcp_token_id,
        redirect_uri: auth_code.redirect_uri.clone(),
        scope: auth_code.scope,
        code_challenge: auth_code.code_challenge,
        code_challenge_method: auth_code.code_challenge_method,
        expires_at: auth_code.expires_at,
        used_at: auth_code.used_at,
        created_at: auth_code.created_at,
    };

    // Save authorization code
    let auth_repo = doxyde_db::repositories::AuthorizationCodeRepository::new(state.db.clone());
    auth_repo.create(&db_auth_code).await?;

    // Build redirect URL with code
    let mut redirect_url = url::Url::parse(&form.redirect_uri)
        .map_err(|_| AppError::bad_request("Invalid redirect_uri"))?;

    redirect_url
        .query_pairs_mut()
        .append_pair("code", &auth_code.code);

    if let Some(state) = form.state {
        redirect_url.query_pairs_mut().append_pair("state", &state);
    }

    Ok(Redirect::to(redirect_url.as_str()))
}

/// Build the full authorize URL from request parameters
fn build_authorize_url(host: &str, headers: &HeaderMap, params: &AuthorizationRequest) -> String {
    // Determine the scheme from X-Forwarded-Proto header or default to https
    let scheme = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("https");

    // Build the full URL
    let mut url = format!("{}://{}/.oauth/authorize?", scheme, host);
    let mut params_vec = vec![];

    params_vec.push(format!(
        "response_type={}",
        urlencoding::encode(&params.response_type)
    ));
    params_vec.push(format!(
        "client_id={}",
        urlencoding::encode(&params.client_id)
    ));
    params_vec.push(format!(
        "redirect_uri={}",
        urlencoding::encode(&params.redirect_uri)
    ));

    if let Some(ref scope) = params.scope {
        params_vec.push(format!("scope={}", urlencoding::encode(scope)));
    }
    if let Some(ref state) = params.state {
        params_vec.push(format!("state={}", urlencoding::encode(state)));
    }
    if let Some(ref code_challenge) = params.code_challenge {
        params_vec.push(format!(
            "code_challenge={}",
            urlencoding::encode(code_challenge)
        ));
    }
    if let Some(ref code_challenge_method) = params.code_challenge_method {
        params_vec.push(format!(
            "code_challenge_method={}",
            urlencoding::encode(code_challenge_method)
        ));
    }

    url.push_str(&params_vec.join("&"));
    url
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authorization_error_redirect() {
        let error = AuthorizationError::access_denied("Test error", Some("state123".to_string()));
        let redirect = error.to_redirect_url("http://localhost:3000/callback");

        assert!(redirect.contains("error=access_denied"));
        assert!(redirect.contains("error_description=Test+error"));
        assert!(redirect.contains("state=state123"));
    }
}
