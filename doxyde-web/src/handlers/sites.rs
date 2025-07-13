use axum::http::StatusCode;

/// List all sites
pub async fn list_sites() -> Result<&'static str, StatusCode> {
    Ok("Sites list placeholder")
}

/// Create a new site
pub async fn create_site() -> Result<&'static str, StatusCode> {
    Ok("Create site placeholder")
}
