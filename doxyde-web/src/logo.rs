use anyhow::{Context, Result};
use doxyde_db::repositories::{ComponentRepository, PageRepository, PageVersionRepository};
use sqlx::SqlitePool;

/// Logo data returned by get_logo_data
pub type LogoData = (String, Option<String>, Option<String>);

/// Get logo data for a site if a logo component exists
/// Returns (logo_url, logo_width, logo_height) or None if no logo
pub async fn get_logo_data(db: &SqlitePool, site_id: i64) -> Result<Option<LogoData>> {
    let page_repo = PageRepository::new(db.clone());
    let version_repo = PageVersionRepository::new(db.clone());
    let component_repo = ComponentRepository::new(db.clone());

    // Get the root page
    let root_page = page_repo
        .get_root_page(site_id)
        .await
        .context("Failed to get root page")?;

    let root_page = match root_page {
        Some(page) => page,
        None => return Ok(None), // No root page, no logo
    };

    // Get the published version of the root page
    let root_version = version_repo
        .get_published(root_page.id.unwrap())
        .await
        .context("Failed to get published version")?;

    let root_version = match root_version {
        Some(version) => version,
        None => return Ok(None), // No published version, no logo
    };

    // Get components for root page
    let components = component_repo
        .list_by_page_version(root_version.id.unwrap())
        .await
        .context("Failed to list components")?;

    // Look for logo image component
    for component in components {
        if component.component_type != "image" {
            continue;
        }

        // Check if this is logo
        if let Some(slug) = component.content.get("slug").and_then(|s| s.as_str()) {
            if slug == "logo" {
                // Get the format (default to png)
                let format = component
                    .content
                    .get("format")
                    .and_then(|f| f.as_str())
                    .unwrap_or("png");

                let logo_url = format!("/{}.{}", slug, format);

                // Get display dimensions if set
                let logo_width = component
                    .content
                    .get("display_width")
                    .and_then(|w| w.as_str())
                    .filter(|w| !w.is_empty())
                    .map(|w| w.to_string());

                let logo_height = component
                    .content
                    .get("display_height")
                    .and_then(|h| h.as_str())
                    .filter(|h| !h.is_empty())
                    .map(|h| h.to_string());

                return Ok(Some((logo_url, logo_width, logo_height)));
            }
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[sqlx::test]
    async fn test_get_logo_data_no_site(pool: SqlitePool) -> Result<()> {
        let result = get_logo_data(&pool, 999).await?;
        assert!(result.is_none());
        Ok(())
    }
}