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
    async_trait,
    extract::{FromRef, FromRequestParts},
    http::{request::Parts, StatusCode},
    RequestPartsExt,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    typed_header::TypedHeader,
};
use doxyde_core::models::{session::Session, user::User};
use doxyde_db::repositories::{SessionRepository, UserRepository};
use sqlx::SqlitePool;

use crate::{session_activity::check_session_idle_timeout, AppState};

/// Current authenticated user, extracted from request
#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub user: User,
    pub session: Session,
}

#[async_trait]
impl<S> FromRequestParts<S> for CurrentUser
where
    SqlitePool: FromRef<S>,
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Extract session ID from cookie or Authorization header
        let session_id = extract_session_id(parts).await?;

        // Get database pool and app state
        let pool = SqlitePool::from_ref(state);
        let app_state = AppState::from_ref(state);

        // Look up session
        let session_repo = SessionRepository::new(pool.clone());
        let session = session_repo
            .find_by_id(&session_id)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?
            .ok_or((StatusCode::UNAUTHORIZED, "Invalid session"))?;

        // Check if session is expired
        if session.is_expired() {
            return Err((StatusCode::UNAUTHORIZED, "Session expired"));
        }

        // Check idle timeout
        let is_active = check_session_idle_timeout(
            &pool,
            &session_id,
            app_state.config.session_timeout_minutes,
        )
        .await
        .unwrap_or(true); // Default to active if check fails

        if !is_active {
            return Err((
                StatusCode::UNAUTHORIZED,
                "Session timed out due to inactivity",
            ));
        }

        // Look up user
        let user_repo = UserRepository::new(pool);
        let user = user_repo
            .find_by_id(session.user_id)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Database error"))?
            .ok_or((StatusCode::UNAUTHORIZED, "User not found"))?;

        // Check if user is active
        if !user.is_active {
            return Err((StatusCode::FORBIDDEN, "Account disabled"));
        }

        Ok(CurrentUser { user, session })
    }
}

/// Session user with just session ID for CSRF token lookup
#[derive(Debug, Clone)]
pub struct SessionUser {
    pub session_id: String,
}

#[async_trait]
impl<S> FromRequestParts<S> for SessionUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let session_id = extract_session_id(parts).await?;
        Ok(SessionUser { session_id })
    }
}

/// Optional authenticated user
#[derive(Debug, Clone)]
pub struct OptionalUser(pub Option<CurrentUser>);

#[async_trait]
impl<S> FromRequestParts<S> for OptionalUser
where
    SqlitePool: FromRef<S>,
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match CurrentUser::from_request_parts(parts, state).await {
            Ok(user) => Ok(OptionalUser(Some(user))),
            Err((StatusCode::UNAUTHORIZED, _)) => Ok(OptionalUser(None)),
            Err(e) => Err(e),
        }
    }
}

async fn extract_session_id(parts: &mut Parts) -> Result<String, (StatusCode, &'static str)> {
    // First try cookie
    let cookies = parts.extract::<axum_extra::extract::CookieJar>().await.ok();

    if let Some(cookies) = cookies {
        if let Some(session_cookie) = cookies.get("session_id") {
            return Ok(session_cookie.value().to_string());
        }
    }

    // Then try Authorization header
    if let Ok(TypedHeader(Authorization(bearer))) =
        parts.extract::<TypedHeader<Authorization<Bearer>>>().await
    {
        return Ok(bearer.token().to_string());
    }

    Err((StatusCode::UNAUTHORIZED, "No session found"))
}

/// Require admin user
#[derive(Debug, Clone)]
pub struct RequireAdmin(pub User);

#[async_trait]
impl<S> FromRequestParts<S> for RequireAdmin
where
    SqlitePool: FromRef<S>,
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let CurrentUser { user, .. } = CurrentUser::from_request_parts(parts, state).await?;

        if !user.is_admin {
            return Err((StatusCode::FORBIDDEN, "Admin access required"));
        }

        Ok(RequireAdmin(user))
    }
}
