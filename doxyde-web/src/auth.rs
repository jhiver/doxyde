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
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Extract session ID from cookie or Authorization header
        let session_id = extract_session_id(parts).await?;

        // Get database pool
        let pool = SqlitePool::from_ref(state);

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

/// Optional authenticated user
#[derive(Debug, Clone)]
pub struct OptionalUser(pub Option<CurrentUser>);

#[async_trait]
impl<S> FromRequestParts<S> for OptionalUser
where
    SqlitePool: FromRef<S>,
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
