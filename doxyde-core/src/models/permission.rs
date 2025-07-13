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

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SiteRole {
    Viewer, // Read-only access
    Editor, // Can edit pages and components
    Owner,  // Full control
}

impl SiteRole {
    /// Check if this role has at least the permissions of the given role
    pub fn has_permission(&self, required: SiteRole) -> bool {
        *self >= required
    }

    /// Check if this role can edit content
    pub fn can_edit(&self) -> bool {
        self.has_permission(SiteRole::Editor)
    }

    /// Check if this role can manage users
    pub fn can_manage_users(&self) -> bool {
        *self == SiteRole::Owner
    }

    /// Get all available roles
    pub fn all() -> Vec<SiteRole> {
        vec![SiteRole::Viewer, SiteRole::Editor, SiteRole::Owner]
    }

    /// Convert role to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            SiteRole::Viewer => "viewer",
            SiteRole::Editor => "editor",
            SiteRole::Owner => "owner",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SiteUser {
    pub site_id: i64,
    pub user_id: i64,
    pub role: SiteRole,
    pub created_at: DateTime<Utc>,
}

impl SiteUser {
    /// Create a new site-user association
    pub fn new(site_id: i64, user_id: i64, role: SiteRole) -> Self {
        Self {
            site_id,
            user_id,
            role,
            created_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_site_role_ordering() {
        assert!(SiteRole::Owner > SiteRole::Editor);
        assert!(SiteRole::Editor > SiteRole::Viewer);
        assert!(SiteRole::Owner > SiteRole::Viewer);

        // Equality
        assert_eq!(SiteRole::Owner, SiteRole::Owner);
        assert_eq!(SiteRole::Editor, SiteRole::Editor);
        assert_eq!(SiteRole::Viewer, SiteRole::Viewer);
    }

    #[test]
    fn test_has_permission() {
        // Owner has all permissions
        assert!(SiteRole::Owner.has_permission(SiteRole::Owner));
        assert!(SiteRole::Owner.has_permission(SiteRole::Editor));
        assert!(SiteRole::Owner.has_permission(SiteRole::Viewer));

        // Editor has editor and viewer permissions
        assert!(!SiteRole::Editor.has_permission(SiteRole::Owner));
        assert!(SiteRole::Editor.has_permission(SiteRole::Editor));
        assert!(SiteRole::Editor.has_permission(SiteRole::Viewer));

        // Viewer only has viewer permissions
        assert!(!SiteRole::Viewer.has_permission(SiteRole::Owner));
        assert!(!SiteRole::Viewer.has_permission(SiteRole::Editor));
        assert!(SiteRole::Viewer.has_permission(SiteRole::Viewer));
    }

    #[test]
    fn test_site_role_serialization() {
        // Test JSON serialization
        assert_eq!(
            serde_json::to_string(&SiteRole::Owner).unwrap(),
            "\"owner\""
        );
        assert_eq!(
            serde_json::to_string(&SiteRole::Editor).unwrap(),
            "\"editor\""
        );
        assert_eq!(
            serde_json::to_string(&SiteRole::Viewer).unwrap(),
            "\"viewer\""
        );

        // Test deserialization
        assert_eq!(
            serde_json::from_str::<SiteRole>("\"owner\"").unwrap(),
            SiteRole::Owner
        );
        assert_eq!(
            serde_json::from_str::<SiteRole>("\"editor\"").unwrap(),
            SiteRole::Editor
        );
        assert_eq!(
            serde_json::from_str::<SiteRole>("\"viewer\"").unwrap(),
            SiteRole::Viewer
        );
    }

    #[test]
    fn test_new_site_user() {
        let site_id = 123;
        let user_id = 456;
        let role = SiteRole::Editor;

        let before = Utc::now();
        let site_user = SiteUser::new(site_id, user_id, role);
        let after = Utc::now();

        assert_eq!(site_user.site_id, site_id);
        assert_eq!(site_user.user_id, user_id);
        assert_eq!(site_user.role, role);
        assert!(site_user.created_at >= before);
        assert!(site_user.created_at <= after);
    }

    #[test]
    fn test_site_user_serialization() {
        let site_user = SiteUser::new(1, 2, SiteRole::Owner);

        // Serialize to JSON
        let json = serde_json::to_string(&site_user).unwrap();

        // Deserialize back
        let deserialized: SiteUser = serde_json::from_str(&json).unwrap();

        assert_eq!(site_user, deserialized);
    }

    #[test]
    fn test_can_edit() {
        assert!(SiteRole::Owner.can_edit());
        assert!(SiteRole::Editor.can_edit());
        assert!(!SiteRole::Viewer.can_edit());
    }

    #[test]
    fn test_can_manage_users() {
        assert!(SiteRole::Owner.can_manage_users());
        assert!(!SiteRole::Editor.can_manage_users());
        assert!(!SiteRole::Viewer.can_manage_users());
    }

    #[test]
    fn test_all_roles() {
        let all = SiteRole::all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&SiteRole::Viewer));
        assert!(all.contains(&SiteRole::Editor));
        assert!(all.contains(&SiteRole::Owner));
    }
}
