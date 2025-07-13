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
pub struct DeletePageForm {
    pub confirm: String,
}

/// Display page delete confirmation
pub async fn delete_page_handler(
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

    // Root pages cannot be deleted
    if page.parent_page_id.is_none() {
        return Err(StatusCode::FORBIDDEN);
    }

    // Check if page has children
    let has_children = page_repo
        .has_children(page_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if has_children {
        // Page has children, cannot delete
        // TODO: Add flash message about page having children
        return Ok(Redirect::to(&format!("/{}", page.slug)).into_response());
    }

    // Get current page breadcrumb for display
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
    context.insert("current_path", &current_path);
    context.insert("user", &user.user);
    context.insert("can_edit", &true);
    context.insert("action", ".delete");

    let html = state
        .templates
        .render("page_delete.html", &context)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Html(html).into_response())
}

/// Handle page delete confirmation
pub async fn do_delete_page_handler(
    state: AppState,
    site: Site,
    page: Page,
    user: CurrentUser,
    Form(form): Form<DeletePageForm>,
) -> Result<Response, StatusCode> {
    // Verify confirmation
    if form.confirm != "DELETE" {
        // User didn't confirm properly, redirect back
        return Ok(Redirect::to(&format!("/{}", page.slug)).into_response());
    }

    // Check permissions again
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

    // Root pages cannot be deleted
    if page.parent_page_id.is_none() {
        return Err(StatusCode::FORBIDDEN);
    }

    // Check if page has children again (in case something changed)
    let has_children = page_repo
        .has_children(page_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if has_children {
        return Err(StatusCode::FORBIDDEN);
    }

    // Get parent page for redirect
    let parent_page = if let Some(parent_id) = page.parent_page_id {
        page_repo
            .find_by_id(parent_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
    } else {
        // This shouldn't happen as we checked above
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    };

    // Build parent URL for redirect
    let breadcrumb = page_repo
        .get_breadcrumb_trail(parent_page.id.unwrap())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let parent_url = if breadcrumb.len() <= 1 {
        "/".to_string()
    } else {
        let path_parts: Vec<&str> = breadcrumb[1..].iter().map(|p| p.slug.as_str()).collect();
        format!("/{}/", path_parts.join("/"))
    };

    // Perform the deletion
    page_repo.delete(page_id).await.map_err(|e| {
        tracing::error!("Failed to delete page: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Redirect to parent page
    Ok(Redirect::to(&parent_url).into_response())
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
    ) -> Result<(Page, Page), anyhow::Error> {
        let page_repo = PageRepository::new(state.db.clone());

        // Get root page (created with site)
        let root = page_repo
            .get_root_page(site_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;

        // Create test page
        let page = Page::new_with_parent(
            site_id,
            root.id.unwrap(),
            "test-page".to_string(),
            "Test Page".to_string(),
        );
        let page_id = page_repo.create(&page).await?;
        let mut page = page;
        page.id = Some(page_id);

        Ok((root, page))
    }

    #[tokio::test]
    async fn test_delete_page_handler_shows_confirmation() -> Result<()> {
        let state = create_test_app_state().await?;
        let user = create_test_user(&state.db, "testuser", "test@example.com", false).await?;
        let site = create_test_site(&state.db, "localhost", "Test Site").await?;

        // Grant editor permission
        let site_user_repo = SiteUserRepository::new(state.db.clone());
        let site_user = SiteUser::new(site.id.unwrap(), user.id.unwrap(), SiteRole::Editor);
        site_user_repo.create(&site_user).await?;

        let (_root, page) = setup_test_pages(&state, site.id.unwrap()).await?;

        // Create current user with session
        let session = create_test_session(&state.db, user.id.unwrap()).await?;
        let current_user = CurrentUser {
            user: user.clone(),
            session,
        };

        // Call the handler
        let response = delete_page_handler(state.clone(), site.clone(), page.clone(), current_user)
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

                // Should show warning about permanent deletion
                assert!(body_str.contains("permanently delete"));
                assert!(body_str.contains(&page.title));
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_page_handler_blocked_for_root() -> Result<()> {
        let state = create_test_app_state().await?;
        let user = create_test_user(&state.db, "admin", "admin@example.com", true).await?;
        let site = create_test_site(&state.db, "localhost", "Test Site").await?;

        let page_repo = PageRepository::new(state.db.clone());
        let root = page_repo
            .get_root_page(site.id.unwrap())
            .await?
            .ok_or_else(|| anyhow::anyhow!("Root page not found"))?;

        // Create current user with session
        let session = create_test_session(&state.db, user.id.unwrap()).await?;
        let current_user = CurrentUser { user, session };

        // Call the handler with root page - should return forbidden
        let result = delete_page_handler(state, site, root, current_user).await;

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), StatusCode::FORBIDDEN);

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_page_handler_blocked_with_children() -> Result<()> {
        let state = create_test_app_state().await?;
        let user = create_test_user(&state.db, "admin", "admin@example.com", true).await?;
        let site = create_test_site(&state.db, "localhost", "Test Site").await?;

        let (_root, parent) = setup_test_pages(&state, site.id.unwrap()).await?;

        // Create a child page
        let page_repo = PageRepository::new(state.db.clone());
        let child = Page::new_with_parent(
            site.id.unwrap(),
            parent.id.unwrap(),
            "child".to_string(),
            "Child Page".to_string(),
        );
        page_repo.create(&child).await?;

        // Create current user with session
        let session = create_test_session(&state.db, user.id.unwrap()).await?;
        let current_user = CurrentUser { user, session };

        // Call the handler - should redirect since page has children
        let response = delete_page_handler(state, site, parent.clone(), current_user)
            .await
            .map_err(|e| anyhow::anyhow!("Handler failed with status: {:?}", e))?;

        // Check response is redirect
        match response.into_response() {
            response => {
                let (parts, _) = response.into_parts();
                assert_eq!(parts.status, StatusCode::SEE_OTHER);
                assert_eq!(
                    parts.headers.get("location").unwrap(),
                    &format!("/{}", parent.slug)
                );
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_do_delete_page_handler() -> Result<()> {
        let state = create_test_app_state().await?;
        let user = create_test_user(&state.db, "editor", "editor@example.com", false).await?;
        let site = create_test_site(&state.db, "localhost", "Test Site").await?;

        // Grant editor permission
        let site_user_repo = SiteUserRepository::new(state.db.clone());
        let site_user = SiteUser::new(site.id.unwrap(), user.id.unwrap(), SiteRole::Editor);
        site_user_repo.create(&site_user).await?;

        let (_root, page) = setup_test_pages(&state, site.id.unwrap()).await?;
        let page_id = page.id.unwrap();

        // Create current user with session
        let session = create_test_session(&state.db, user.id.unwrap()).await?;
        let current_user = CurrentUser { user, session };

        // Create form with proper confirmation
        let form = DeletePageForm {
            confirm: "DELETE".to_string(),
        };

        // Call the handler
        let response = do_delete_page_handler(
            state.clone(),
            site.clone(),
            page.clone(),
            current_user,
            Form(form),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Handler failed with status: {:?}", e))?;

        // Check response is redirect to parent (root)
        match response.into_response() {
            response => {
                let (parts, _) = response.into_parts();
                assert_eq!(parts.status, StatusCode::SEE_OTHER);
                assert_eq!(parts.headers.get("location").unwrap(), "/");
            }
        }

        // Verify page was actually deleted
        let page_repo = PageRepository::new(state.db);
        let deleted_page = page_repo.find_by_id(page_id).await?;
        assert!(deleted_page.is_none());

        Ok(())
    }

    #[tokio::test]
    async fn test_do_delete_page_requires_confirmation() -> Result<()> {
        let state = create_test_app_state().await?;
        let user = create_test_user(&state.db, "admin", "admin@example.com", true).await?;
        let site = create_test_site(&state.db, "localhost", "Test Site").await?;

        let (_root, page) = setup_test_pages(&state, site.id.unwrap()).await?;
        let page_id = page.id.unwrap();

        // Create current user with session
        let session = create_test_session(&state.db, user.id.unwrap()).await?;
        let current_user = CurrentUser { user, session };

        // Create form with wrong confirmation
        let form = DeletePageForm {
            confirm: "wrong".to_string(),
        };

        // Call the handler
        let response = do_delete_page_handler(
            state.clone(),
            site.clone(),
            page.clone(),
            current_user,
            Form(form),
        )
        .await
        .map_err(|e| anyhow::anyhow!("Handler failed with status: {:?}", e))?;

        // Check response is redirect back to page
        match response.into_response() {
            response => {
                let (parts, _) = response.into_parts();
                assert_eq!(parts.status, StatusCode::SEE_OTHER);
                assert_eq!(
                    parts.headers.get("location").unwrap(),
                    &format!("/{}", page.slug)
                );
            }
        }

        // Verify page was NOT deleted
        let page_repo = PageRepository::new(state.db);
        let still_exists = page_repo.find_by_id(page_id).await?;
        assert!(still_exists.is_some());

        Ok(())
    }
}
