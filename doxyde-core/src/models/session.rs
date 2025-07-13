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

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Session {
    pub id: String,
    pub user_id: i64,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl Session {
    /// Create a new session with default expiration (24 hours)
    pub fn new(user_id: i64) -> Self {
        let now = Utc::now();
        let expires_at = now + Duration::hours(24);

        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            expires_at,
            created_at: now,
        }
    }

    /// Create a new session with custom expiration
    pub fn new_with_expiry(user_id: i64, expiry_duration: Duration) -> Self {
        let now = Utc::now();
        let expires_at = now + expiry_duration;

        Self {
            id: Uuid::new_v4().to_string(),
            user_id,
            expires_at,
            created_at: now,
        }
    }

    /// Check if the session has expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_session() {
        let user_id = 123;
        let before = Utc::now();
        let session = Session::new(user_id);
        let after = Utc::now();

        // Check ID is UUID v4 format
        assert_eq!(session.id.len(), 36); // UUID v4 string length
        assert!(Uuid::parse_str(&session.id).is_ok());

        // Check user_id
        assert_eq!(session.user_id, user_id);

        // Check timestamps
        assert!(session.created_at >= before);
        assert!(session.created_at <= after);

        // Check expiration is 24 hours from creation
        let expected_expiry = session.created_at + Duration::hours(24);
        let diff = session.expires_at - expected_expiry;
        assert!(diff.num_seconds().abs() < 1); // Within 1 second
    }

    #[test]
    fn test_new_session_unique_ids() {
        let session1 = Session::new(1);
        let session2 = Session::new(1);
        let session3 = Session::new(2);

        // All sessions should have unique IDs
        assert_ne!(session1.id, session2.id);
        assert_ne!(session1.id, session3.id);
        assert_ne!(session2.id, session3.id);
    }

    #[test]
    fn test_session_serialization() {
        let session = Session::new(42);

        // Serialize to JSON
        let json = serde_json::to_string(&session).unwrap();

        // Deserialize back
        let deserialized: Session = serde_json::from_str(&json).unwrap();

        assert_eq!(session, deserialized);
    }

    #[test]
    fn test_new_with_expiry() {
        let user_id = 456;
        let expiry = Duration::hours(48);
        let before = Utc::now();
        let session = Session::new_with_expiry(user_id, expiry);
        let after = Utc::now();

        // Check basic properties
        assert_eq!(session.user_id, user_id);
        assert!(session.created_at >= before);
        assert!(session.created_at <= after);

        // Check custom expiration
        let expected_expiry = session.created_at + expiry;
        let diff = session.expires_at - expected_expiry;
        assert!(diff.num_seconds().abs() < 1); // Within 1 second
    }

    #[test]
    fn test_is_expired() {
        // Create a session that expires in 1 second
        let session = Session::new_with_expiry(1, Duration::seconds(1));

        // Should not be expired immediately
        assert!(!session.is_expired());

        // Wait 2 seconds
        std::thread::sleep(std::time::Duration::from_secs(2));

        // Should be expired now
        assert!(session.is_expired());
    }

    #[test]
    fn test_is_expired_far_future() {
        // Create a session that expires in 100 years
        let session = Session::new_with_expiry(1, Duration::days(365 * 100));

        // Should not be expired
        assert!(!session.is_expired());
    }

    #[test]
    fn test_is_expired_past() {
        // Create a session with manual past expiration
        let session = Session {
            id: Uuid::new_v4().to_string(),
            user_id: 1,
            expires_at: Utc::now() - Duration::hours(1), // Expired 1 hour ago
            created_at: Utc::now() - Duration::hours(2), // Created 2 hours ago
        };

        // Should be expired
        assert!(session.is_expired());
    }
}
