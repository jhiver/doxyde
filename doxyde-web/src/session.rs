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
use doxyde_core::User;
use doxyde_db::repositories::{SessionRepository, UserRepository};
use sqlx::SqlitePool;

pub async fn get_current_user(
    db: &SqlitePool,
    session: &axum_extra::extract::CookieJar,
) -> Result<Option<User>> {
    if let Some(session_cookie) = session.get("session_id") {
        let session_repo = SessionRepository::new(db.clone());
        let user_repo = UserRepository::new(db.clone());

        if let Some(session_data) = session_repo.find_by_id(session_cookie.value()).await? {
            if !session_data.is_expired() {
                return user_repo.find_by_id(session_data.user_id).await;
            }
        }
    }
    Ok(None)
}
