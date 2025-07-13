use anyhow::Result;
use axum::{
    extract::Form,
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use doxyde_core::models::{page::Page, permission::SiteRole, site::Site};
use doxyde_db::repositories::{PageRepository, SiteUserRepository};
use serde::Deserialize;
use tera::Context;

use crate::{auth::CurrentUser, template_context::add_base_context, AppState};

#[derive(Debug, Deserialize)]
pub struct MovePageForm {
    pub target_parent_id: i64,
}

/// Display page move form
pub async fn move_page_handler(
    state: AppState,
    site: Site,
    page: Page,
    user: CurrentUser,
) -> Result<Response, StatusCode> {
    // Check permissions
    if !user.user.is_admin {
        let site_user_repo = SiteUserRepository::new(state.db.clone());
        if let Ok(Some(site_user)) = site_user_repo
            .find_by_site_and_user(site.id.unwrap(), user.user.id.unwrap())
            .await
        {
            if site_user.role != SiteRole::Editor && site_user.role != SiteRole::Owner {
                return Err(StatusCode::FORBIDDEN);
            }
        } else {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    let page_id = page.id.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let page_repo = PageRepository::new(state.db.clone());

    // Root pages cannot be moved
    if page.parent_page_id.is_none() {
        // TODO: Add flash message about root pages cannot be moved
        return Ok(Redirect::to("/").into_response());
    }

    // Get valid move targets
    let valid_targets = page_repo
        .get_valid_move_targets(page_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // If no valid targets, redirect back with error
    if valid_targets.is_empty() {
        // TODO: Add flash message about no valid move targets
        return Ok(Redirect::to(&format!("/{}", page.slug)).into_response());
    }

    // Build target pages with full paths
    let mut target_data = Vec::new();
    for target in valid_targets {
        let target_id = target.id.unwrap();

        // Get breadcrumb for this target to build full path
        let breadcrumb = page_repo
            .get_breadcrumb_trail(target_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // Build full path
        let path = if breadcrumb.len() <= 1 {
            "/".to_string()
        } else {
            let path_parts: Vec<&str> = breadcrumb[1..].iter().map(|p| p.slug.as_str()).collect();
            format!("/{}", path_parts.join("/"))
        };

        target_data.push(serde_json::json!({
            "id": target_id,
            "title": target.title,
            "path": path,
            "is_root": target.parent_page_id.is_none()
        }));
    }

    // Sort targets by path length first, then alphabetically
    target_data.sort_by(|a, b| {
        let path_a = a["path"].as_str().unwrap_or("");
        let path_b = b["path"].as_str().unwrap_or("");

        // First compare by length
        match path_a.len().cmp(&path_b.len()) {
            std::cmp::Ordering::Equal => {
                // If same length, sort alphabetically
                path_a.cmp(path_b)
            }
            other => other,
        }
    });

    // Get current page breadcrumb
    let breadcrumb = page_repo
        .get_breadcrumb_trail(page_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Build current page path
    let current_path = if breadcrumb.len() <= 1 {
        "/".to_string()
    } else {
        let path_parts: Vec<&str> = breadcrumb[1..].iter().map(|p| p.slug.as_str()).collect();
        format!("/{}", path_parts.join("/"))
    };

    let mut context = Context::new();
    
    // Add base context (site_title, root_page_title, logo data)
    add_base_context(&mut context, &state, &site)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    context.insert("page", &page);
    context.insert("targets", &target_data);
    context.insert("current_path", &current_path);
    context.insert("user", &user.user);
    context.insert("can_edit", &true);
    context.insert("action", ".move");

    let html = state
        .templates
        .render("page_move.html", &context)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Html(html).into_response())
}

/// Handle page move submission
pub async fn do_move_page_handler(
    state: AppState,
    site: Site,
    page: Page,
    user: CurrentUser,
    Form(form): Form<MovePageForm>,
) -> Result<Response, StatusCode> {
    // Check permissions
    if !user.user.is_admin {
        let site_user_repo = SiteUserRepository::new(state.db.clone());
        if let Ok(Some(site_user)) = site_user_repo
            .find_by_site_and_user(site.id.unwrap(), user.user.id.unwrap())
            .await
        {
            if site_user.role != SiteRole::Editor && site_user.role != SiteRole::Owner {
                return Err(StatusCode::FORBIDDEN);
            }
        } else {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    let page_id = page.id.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
    let page_repo = PageRepository::new(state.db.clone());

    // Root pages cannot be moved
    if page.parent_page_id.is_none() {
        return Err(StatusCode::FORBIDDEN);
    }

    // Perform the move
    page_repo
        .move_page(page_id, form.target_parent_id)
        .await
        .map_err(|e| {
            tracing::error!("Failed to move page: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Get the new parent to build redirect URL
    let _new_parent = page_repo
        .find_by_id(form.target_parent_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Build new page URL
    let breadcrumb = page_repo
        .get_breadcrumb_trail(form.target_parent_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let new_url = if breadcrumb.is_empty() {
        format!("/{}", page.slug)
    } else if breadcrumb.len() == 1 {
        // Moving under root
        format!("/{}", page.slug)
    } else {
        // Build path from breadcrumb
        let path_parts: Vec<&str> = breadcrumb[1..].iter().map(|p| p.slug.as_str()).collect();
        format!("/{}/{}", path_parts.join("/"), page.slug)
    };

    Ok(Redirect::to(&new_url).into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{
        create_test_app_state, create_test_session, create_test_site, create_test_user,
    };
    use axum::body::to_bytes;
    use doxyde_core::models::permission::SiteUser;

    async fn setup_test_pages(
        state: &AppState,
        site_id: i64,
    ) -> Result<(Page, Page, Page), anyhow::Error> {
        let page_repo = PageRepository::new(state.db.clone());

        // Get root page (created with site)
        let root = page_repo
            .get_root_page(site_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;

        // Create test pages
        let page1 = Page::new_with_parent(
            site_id,
            root.id.unwrap(),
            "page1".to_string(),
            "Page 1".to_string(),
        );
        let page1_id = page_repo.create(&page1).await?;
        let mut page1 = page1;
        page1.id = Some(page1_id);

        let page2 = Page::new_with_parent(
            site_id,
            root.id.unwrap(),
            "page2".to_string(),
            "Page 2".to_string(),
        );
        let page2_id = page_repo.create(&page2).await?;
        let mut page2 = page2;
        page2.id = Some(page2_id);

        Ok((root, page1, page2))
    }

    #[tokio::test]
    async fn test_move_page_handler_shows_valid_targets() -> Result<()> {
        let state = create_test_app_state().await?;
        let user = create_test_user(&state.db, "testuser", "test@example.com", false).await?;
        let site = create_test_site(&state.db, "localhost", "Test Site").await?;

        // Grant editor permission
        let site_user_repo = SiteUserRepository::new(state.db.clone());
        let site_user = SiteUser::new(site.id.unwrap(), user.id.unwrap(), SiteRole::Editor);
        site_user_repo.create(&site_user).await?;

        let (_root, page1, page2) = setup_test_pages(&state, site.id.unwrap()).await?;

        // Create current user with session
        let session = create_test_session(&state.db, user.id.unwrap()).await?;
        let current_user = CurrentUser {
            user: user.clone(),
            session,
        };

        // Call the handler
        let response = move_page_handler(state.clone(), site.clone(), page1.clone(), current_user)
            .await
            .map_err(|e| anyhow::anyhow!("Handler failed with status: {:?}", e))?;

        // Check response is HTML
        match response.into_response() {
            response => {
                let (parts, body) = response.into_parts();
                assert_eq!(parts.status, StatusCode::OK);
                assert!(parts
                    .headers
                    .get("content-type")
                    .unwrap()
                    .to_str()?
                    .contains("text/html"));

                // Convert body to string
                let body_bytes = to_bytes(body, usize::MAX).await.unwrap();
                let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();

                // Should show page2 as a valid target
                assert!(body_str.contains(&page2.title));
                // Should not show page1 itself
                assert!(!body_str.contains(&format!("value=\"{}\"", page1.id.unwrap())));
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_move_page_handler_root_page_redirects() -> Result<()> {
        let state = create_test_app_state().await?;
        let user = create_test_user(&state.db, "testuser", "test@example.com", false).await?;
        let site = create_test_site(&state.db, "localhost", "Test Site").await?;

        // Grant editor permission
        let site_user_repo = SiteUserRepository::new(state.db.clone());
        let site_user = SiteUser::new(site.id.unwrap(), user.id.unwrap(), SiteRole::Editor);
        site_user_repo.create(&site_user).await?;

        // Get the root page (which cannot be moved)
        let page_repo = PageRepository::new(state.db.clone());
        let root = page_repo
            .get_root_page(site.id.unwrap())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;

        // Create current user with session
        let session = create_test_session(&state.db, user.id.unwrap()).await?;
        let current_user = CurrentUser { user, session };

        // Call the handler with root page - should redirect since root pages can't be moved
        let response = move_page_handler(state, site, root.clone(), current_user)
            .await
            .map_err(|e| anyhow::anyhow!("Handler failed with status: {:?}", e))?;

        // Check response is redirect to root
        match response.into_response() {
            response => {
                let (parts, _) = response.into_parts();
                assert_eq!(parts.status, StatusCode::SEE_OTHER);
                assert_eq!(parts.headers.get("location").unwrap(), "/");
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_move_page_requires_permission() -> Result<()> {
        let state = create_test_app_state().await?;
        let user = create_test_user(&state.db, "viewer", "viewer@example.com", false).await?;
        let site = create_test_site(&state.db, "localhost", "Test Site").await?;

        // Grant only viewer permission
        let site_user_repo = SiteUserRepository::new(state.db.clone());
        let site_user = SiteUser::new(site.id.unwrap(), user.id.unwrap(), SiteRole::Viewer);
        site_user_repo.create(&site_user).await?;

        let (_root, page1, _page2) = setup_test_pages(&state, site.id.unwrap()).await?;

        // Create current user with session
        let session = create_test_session(&state.db, user.id.unwrap()).await?;
        let current_user = CurrentUser { user, session };

        // Call the handler - should return forbidden
        let result = move_page_handler(state, site, page1, current_user).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::FORBIDDEN);

        Ok(())
    }

    #[tokio::test]
    async fn test_do_move_page_handler() -> Result<()> {
        let state = create_test_app_state().await?;
        let user = create_test_user(&state.db, "editor", "editor@example.com", false).await?;
        let site = create_test_site(&state.db, "localhost", "Test Site").await?;

        // Grant editor permission
        let site_user_repo = SiteUserRepository::new(state.db.clone());
        let site_user = SiteUser::new(site.id.unwrap(), user.id.unwrap(), SiteRole::Editor);
        site_user_repo.create(&site_user).await?;

        let (_root, page1, page2) = setup_test_pages(&state, site.id.unwrap()).await?;

        // Create current user with session
        let session = create_test_session(&state.db, user.id.unwrap()).await?;
        let current_user = CurrentUser { user, session };

        // Create form to move page1 under page2
        let form = MovePageForm {
            target_parent_id: page2.id.unwrap(),
        };

        // Call the handler
        let response = do_move_page_handler(
            state.clone(),
            site.clone(),
            page1.clone(),
            current_user,
            Form(form),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Handler failed with status: {:?}", e))?;

        // Check response is redirect to new location
        match response.into_response() {
            response => {
                let (parts, _) = response.into_parts();
                assert_eq!(parts.status, StatusCode::SEE_OTHER);
                assert_eq!(
                    parts.headers.get("location").unwrap(),
                    &format!("/{}/{}", page2.slug, page1.slug)
                );
            }
        }

        // Verify page was actually moved
        let page_repo = PageRepository::new(state.db);
        let moved_page = page_repo.find_by_id(page1.id.unwrap()).await?.unwrap();
        assert_eq!(moved_page.parent_page_id, Some(page2.id.unwrap()));

        Ok(())
    }

    #[tokio::test]
    async fn test_admin_can_always_move() -> Result<()> {
        let state = create_test_app_state().await?;
        let admin = create_test_user(&state.db, "admin", "admin@example.com", true).await?;
        let site = create_test_site(&state.db, "localhost", "Test Site").await?;

        // No site permissions needed for admin
        let (_root, page1, _page2) = setup_test_pages(&state, site.id.unwrap()).await?;

        // Create current user with session
        let session = create_test_session(&state.db, admin.id.unwrap()).await?;
        let current_user = CurrentUser {
            user: admin,
            session,
        };

        // Call the handler - admin should have access
        let response = move_page_handler(state, site, page1, current_user)
            .await
            .map_err(|e| anyhow::anyhow!("Handler failed with status: {:?}", e))?;

        // Check response is OK
        match response.into_response() {
            response => {
                let (parts, _) = response.into_parts();
                assert_eq!(parts.status, StatusCode::OK);
            }
        }

        Ok(())
    }
}
