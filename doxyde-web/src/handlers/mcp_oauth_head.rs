use axum::{
    http::{header, StatusCode},
    response::IntoResponse,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};

/// OAuth2-protected MCP endpoint HEAD handler
/// Used by Claude Desktop to check if the endpoint exists
pub async fn mcp_oauth_head_handler(
    auth_header: Option<TypedHeader<Authorization<Bearer>>>,
) -> impl IntoResponse {
    // If no Bearer token, return WWW-Authenticate header
    if auth_header.is_none() {
        return (
            StatusCode::UNAUTHORIZED,
            [(header::WWW_AUTHENTICATE, "Bearer")],
        )
            .into_response();
    }
    
    // If Bearer token is present, return OK
    StatusCode::OK.into_response()
}