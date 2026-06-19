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

//! Per-site booking configuration and unit selection (lot 2).
//!
//! Stores which sejours-api service this site talks to and which Hostaway units
//! it offers, grouped into `primary` (shown first) and `secondary` (sister-house
//! alternative) roles. See migration `022_booking.sql`.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

/// Singleton booking-service connection for a site.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BookingConfig {
    pub service_url: String,
    pub service_secret: String,
}

impl BookingConfig {
    /// True once an admin has configured a service endpoint.
    pub fn is_configured(&self) -> bool {
        !self.service_url.trim().is_empty()
    }
}

/// A Hostaway unit this site offers, with its display role and ordering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookingListing {
    pub listing_id: i64,
    pub role: String,
    pub position: i64,
}

pub struct BookingRepository {
    pool: SqlitePool,
}

impl BookingRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Read the singleton config row (seeded empty by the migration).
    pub async fn get_config(&self) -> Result<BookingConfig> {
        let row = sqlx::query_as::<_, (String, String)>(
            "SELECT service_url, service_secret FROM booking_config WHERE id = 1",
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch booking config")?;

        Ok(row
            .map(|(service_url, service_secret)| BookingConfig {
                service_url,
                service_secret,
            })
            .unwrap_or_default())
    }

    /// Upsert the singleton config row.
    pub async fn set_config(&self, service_url: &str, service_secret: &str) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO booking_config (id, service_url, service_secret, updated_at)
            VALUES (1, ?, ?, datetime('now'))
            ON CONFLICT(id) DO UPDATE SET
                service_url = excluded.service_url,
                service_secret = excluded.service_secret,
                updated_at = datetime('now')
            "#,
        )
        .bind(service_url)
        .bind(service_secret)
        .execute(&self.pool)
        .await
        .context("Failed to save booking config")?;
        Ok(())
    }

    /// All configured units, ordered primary-first then by position.
    pub async fn list_listings(&self) -> Result<Vec<BookingListing>> {
        let rows = sqlx::query_as::<_, (i64, String, i64)>(
            r#"
            SELECT listing_id, role, position
            FROM booking_listing
            ORDER BY CASE role WHEN 'primary' THEN 0 ELSE 1 END, position, listing_id
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to list booking listings")?;

        Ok(rows
            .into_iter()
            .map(|(listing_id, role, position)| BookingListing {
                listing_id,
                role,
                position,
            })
            .collect())
    }

    /// Units of a given role, ordered by position. role is 'primary' or 'secondary'.
    pub async fn listings_by_role(&self, role: &str) -> Result<Vec<i64>> {
        let rows = sqlx::query_as::<_, (i64,)>(
            "SELECT listing_id FROM booking_listing WHERE role = ? ORDER BY position, listing_id",
        )
        .bind(role)
        .fetch_all(&self.pool)
        .await
        .context("Failed to list booking listings by role")?;
        Ok(rows.into_iter().map(|(id,)| id).collect())
    }

    /// Replace the whole unit selection in one transaction.
    pub async fn replace_listings(&self, listings: &[BookingListing]) -> Result<()> {
        let mut tx = self.pool.begin().await.context("begin tx")?;
        sqlx::query("DELETE FROM booking_listing")
            .execute(&mut *tx)
            .await
            .context("clear booking listings")?;
        for l in listings {
            sqlx::query(
                "INSERT INTO booking_listing (listing_id, role, position) VALUES (?, ?, ?)",
            )
            .bind(l.listing_id)
            .bind(&l.role)
            .bind(l.position)
            .execute(&mut *tx)
            .await
            .context("insert booking listing")?;
        }
        tx.commit().await.context("commit tx")?;
        Ok(())
    }
}
