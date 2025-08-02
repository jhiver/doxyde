use axum::{
    body::Body,
    extract::{FromRequestParts, State},
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use chrono::Utc;
use std::sync::Arc;

use crate::{auth::SessionUser, AppState};

/// Middleware to update session last activity time
pub async fn update_session_activity(
    State(state): State<Arc<AppState>>,
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Try to extract session user from request parts
    let session_user = {
        let (mut parts, body) = request.into_parts();
        let session_user = SessionUser::from_request_parts(&mut parts, &state)
            .await
            .ok();
        request = Request::from_parts(parts, body);
        session_user
    };

    // Update last activity if user is authenticated
    if let Some(session_user) = session_user {
        let now = Utc::now().to_rfc3339();

        // Fire and forget - don't wait for update or handle errors
        let pool = state.db.clone();
        let session_id = session_user.session_id.clone();

        tokio::spawn(async move {
            let _ = sqlx::query!(
                "UPDATE sessions SET last_activity = ? WHERE id = ?",
                now,
                session_id
            )
            .execute(&pool)
            .await;
        });
    }

    Ok(next.run(request).await)
}

/// Check if session has been idle for too long
pub async fn check_session_idle_timeout(
    pool: &sqlx::SqlitePool,
    session_id: &str,
    timeout_minutes: i64,
) -> Result<bool, sqlx::Error> {
    let row = sqlx::query!(
        r#"
        SELECT last_activity
        FROM sessions
        WHERE id = ?
        "#,
        session_id
    )
    .fetch_optional(pool)
    .await?;

    if let Some(row) = row {
        if let Some(last_activity) = row.last_activity {
            // Parse the timestamp
            let last_activity_time = if last_activity.contains('T') {
                chrono::DateTime::parse_from_rfc3339(&last_activity)
                    .ok()
                    .map(|dt| dt.with_timezone(&chrono::Utc))
            } else {
                chrono::NaiveDateTime::parse_from_str(&last_activity, "%Y-%m-%d %H:%M:%S")
                    .ok()
                    .map(|dt| dt.and_utc())
            };

            if let Some(last_time) = last_activity_time {
                let idle_duration = Utc::now() - last_time;
                return Ok(idle_duration.num_minutes() <= timeout_minutes);
            }
        }
    }

    // If no last activity or parsing fails, consider it valid
    Ok(true)
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_session_activity_middleware() {
        // This module primarily contains middleware that updates session activity
        // The actual testing is done through integration tests
        // This test ensures the module compiles and can be imported
        let _ = super::update_session_activity;
    }
}
