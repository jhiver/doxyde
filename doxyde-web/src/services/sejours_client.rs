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

//! HTTP client for the per-site `sejours-api` microservice (lot 2).
//!
//! doxyde speaks this stable v1 contract and never touches Hostaway directly.
//! The service URL and shared secret are stored per-site in `booking_config`
//! (see `doxyde_db::repositories::BookingRepository`). Every call carries the
//! `X-Sejours-Secret` header. Response structs are `Serialize` so handlers can
//! drop them straight into the Tera context.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

const SECRET_HEADER: &str = "X-Sejours-Secret";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Listing {
    pub listing_id: i64,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub internal_name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub person_capacity: Option<i64>,
    #[serde(default)]
    pub guests_included: Option<i64>,
    #[serde(default)]
    pub bedrooms: Option<i64>,
    #[serde(default)]
    pub bathrooms: Option<f64>,
    #[serde(default)]
    pub currency_code: Option<String>,
    #[serde(default)]
    pub min_nights: Option<i64>,
    #[serde(default)]
    pub city: Option<String>,
    #[serde(default)]
    pub country: Option<String>,
    #[serde(default)]
    pub thumbnail_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StayLeg {
    pub listing_id: i64,
    #[serde(default)]
    pub name: Option<String>,
    pub check_in: String,
    pub check_out: String,
    pub nights: i64,
    #[serde(default)]
    pub price: Option<f64>,
    #[serde(default)]
    pub currency_code: Option<String>,
    #[serde(default)]
    pub images: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailabilityResult {
    pub name: String,
    pub is_multi_stay: bool,
    pub leg_count: i64,
    pub check_in: String,
    pub check_out: String,
    pub nights: i64,
    #[serde(default)]
    pub person_capacity: Option<i64>,
    #[serde(default)]
    pub currency_code: Option<String>,
    #[serde(default)]
    pub total_price: Option<f64>,
    #[serde(default)]
    pub price_is_estimate: bool,
    #[serde(default)]
    pub images: Vec<String>,
    pub legs: Vec<StayLeg>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailabilityResponse {
    pub check_in: String,
    pub check_out: String,
    pub nights: i64,
    pub adults: i64,
    pub children: i64,
    pub infants: i64,
    pub results: Vec<AvailabilityResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteResponse {
    pub listing_id: i64,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub person_capacity: Option<i64>,
    #[serde(default)]
    pub images: Vec<String>,
    pub check_in: String,
    pub check_out: String,
    pub nights: i64,
    pub available: bool,
    #[serde(default)]
    pub currency_code: Option<String>,
    #[serde(default)]
    pub total_price: Option<f64>,
    #[serde(default)]
    pub components: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReservationResponse {
    #[serde(default)]
    pub reservation_id: Option<i64>,
    #[serde(default)]
    pub confirmation_code: Option<String>,
    pub listing_id: i64,
    pub check_in: String,
    pub check_out: String,
    pub nights: i64,
    #[serde(default)]
    pub total_price: Option<f64>,
    #[serde(default)]
    pub currency_code: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub payment_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarResponse {
    pub listing_id: i64,
    pub min_date: String,
    pub max_date: String,
    #[serde(default)]
    pub blocked: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Contact {
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone: Option<String>,
}

pub struct SejoursClient {
    client: reqwest::Client,
    base_url: String,
    secret: String,
}

impl SejoursClient {
    /// Build a client for a site's configured service. `base_url` is the service
    /// root (e.g. `http://127.0.0.1:8200`); a trailing slash is tolerated.
    pub fn new(base_url: &str, secret: &str) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .build()
            .unwrap_or_default();
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            secret: secret.to_string(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    pub async fn listings(&self) -> Result<Vec<Listing>> {
        let resp = self
            .client
            .get(self.url("/v1/listings"))
            .header(SECRET_HEADER, &self.secret)
            .send()
            .await
            .context("sejours-api /v1/listings request failed")?;
        Self::json(resp).await
    }

    pub async fn availability(
        &self,
        check_in: &str,
        check_out: &str,
        adults: i64,
        children: i64,
        infants: i64,
        listing_ids: &[i64],
    ) -> Result<AvailabilityResponse> {
        let body = serde_json::json!({
            "from": check_in,
            "to": check_out,
            "adults": adults,
            "children": children,
            "infants": infants,
            "listing_ids": listing_ids,
        });
        let resp = self
            .client
            .post(self.url("/v1/availability"))
            .header(SECRET_HEADER, &self.secret)
            .json(&body)
            .send()
            .await
            .context("sejours-api /v1/availability request failed")?;
        Self::json(resp).await
    }

    pub async fn calendar(&self, listing_id: i64) -> Result<CalendarResponse> {
        let resp = self
            .client
            .get(self.url(&format!("/v1/calendar/{listing_id}")))
            .header(SECRET_HEADER, &self.secret)
            .send()
            .await
            .context("sejours-api /v1/calendar request failed")?;
        Self::json(resp).await
    }

    pub async fn quote(
        &self,
        listing_id: i64,
        check_in: &str,
        check_out: &str,
        adults: i64,
        children: i64,
        infants: i64,
    ) -> Result<QuoteResponse> {
        let body = serde_json::json!({
            "listing_id": listing_id,
            "from": check_in,
            "to": check_out,
            "adults": adults,
            "children": children,
            "infants": infants,
        });
        let resp = self
            .client
            .post(self.url("/v1/quote"))
            .header(SECRET_HEADER, &self.secret)
            .json(&body)
            .send()
            .await
            .context("sejours-api /v1/quote request failed")?;
        Self::json(resp).await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn create_reservation(
        &self,
        listing_id: i64,
        check_in: &str,
        check_out: &str,
        adults: i64,
        children: i64,
        infants: i64,
        contact: &Contact,
        note: Option<&str>,
    ) -> Result<ReservationResponse> {
        let body = serde_json::json!({
            "listing_id": listing_id,
            "from": check_in,
            "to": check_out,
            "adults": adults,
            "children": children,
            "infants": infants,
            "contact": contact,
            "note": note,
        });
        let resp = self
            .client
            .post(self.url("/v1/reservations"))
            .header(SECRET_HEADER, &self.secret)
            .json(&body)
            .send()
            .await
            .context("sejours-api /v1/reservations request failed")?;
        Self::json(resp).await
    }

    async fn json<T: for<'de> Deserialize<'de>>(resp: reqwest::Response) -> Result<T> {
        let status = resp.status();
        let text = resp
            .text()
            .await
            .context("read sejours-api response body")?;
        if !status.is_success() {
            return Err(anyhow!("sejours-api returned {}: {}", status, text));
        }
        serde_json::from_str(&text).with_context(|| format!("decode sejours-api response: {text}"))
    }
}
