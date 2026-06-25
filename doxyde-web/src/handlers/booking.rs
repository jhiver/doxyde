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

//! Booking controllers (lot 2): `/.stay` (search), `/.book` (quote + create),
//! `/.booking-config` (admin). They translate the stable per-site config in
//! `booking_config` / `booking_listing` into calls against the sejours-api
//! microservice and render Tera pages that extend `base.html` (so they inherit
//! the site chrome and i18n). doxyde never talks to Hostaway directly.

use std::collections::HashMap;

use axum::{
    extract::{Form, Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Redirect, Response},
};
use axum_extra::extract::Host;
use doxyde_core::models::site::Site;
use doxyde_db::repositories::{BookingListing, BookingRepository};
use serde::Deserialize;
use tera::Context;

use crate::{
    auth::CurrentUser,
    content_translate::TranslationPolicy,
    csrf::get_or_create_csrf_token,
    db_middleware::SiteDatabase,
    locale_middleware::RequestLocale,
    services::sejours_client::{Contact, SejoursClient},
    site_config::get_site_config,
    template_context::{add_base_context, add_locale_context},
    AppState,
};

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// True when an ISO `YYYY-MM-DD` date is strictly before today in Mauritius time.
/// Lenient by design: a same-day link stays valid (low season check-in tonight is
/// real). Unparseable input returns false so the normal flow handles it.
fn is_past_date(date: &str) -> bool {
    let today = chrono::Utc::now()
        .with_timezone(&chrono_tz::Indian::Mauritius)
        .date_naive();
    match chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d") {
        Ok(d) => d < today,
        Err(_) => false,
    }
}

/// 302 to the site home. Used when a deep-link targets a past date (e.g. an expired
/// last-minute ad link) so visitors land on the homepage, not an empty/broken form.
fn redirect_home() -> Response {
    (StatusCode::FOUND, [(axum::http::header::LOCATION, "/")]).into_response()
}

async fn load_site(db: &sqlx::SqlitePool, host: &str) -> Result<Site, StatusCode> {
    get_site_config(db, host)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)
}

/// Base + locale context for a public booking page. Booking pages are
/// transactional, not canonical content: we keep the translated `labels` and the
/// language switcher but suppress hreflang (there is no `/.stay/.fr` URL).
async fn booking_context(
    state: &AppState,
    db: &sqlx::SqlitePool,
    site: &Site,
    locale: &RequestLocale,
) -> Result<Context, StatusCode> {
    let mut context = Context::new();
    add_base_context(&mut context, db, site, None)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    // current_path "" -> language switch URLs resolve to site root ("/.fr").
    add_locale_context(
        &mut context,
        state,
        db,
        site,
        locale,
        "",
        TranslationPolicy::Deferred,
    )
    .await;
    // Suppress hreflang on transactional pages (no per-language canonical URL).
    context.insert("hreflang_alternates", &Vec::<serde_json::Value>::new());
    Ok(context)
}

/// Attach the CMS page URL (if mapped) to each availability result and its legs
/// so the search cards can deep-link to the rich apartment page.
fn enrich_with_pages(
    results: &[crate::services::sejours_client::AvailabilityResult],
    page_map: &HashMap<i64, String>,
) -> Vec<serde_json::Value> {
    results
        .iter()
        .map(|r| {
            let mut value = serde_json::to_value(r).unwrap_or(serde_json::Value::Null);
            if let Some(legs) = value.get_mut("legs").and_then(|l| l.as_array_mut()) {
                for leg in legs.iter_mut() {
                    if let Some(id) = leg.get("listing_id").and_then(|v| v.as_i64()) {
                        if let Some(path) = page_map.get(&id) {
                            leg["page_url"] = serde_json::json!(path);
                        }
                    }
                }
            }
            // Single-stay convenience: surface the (only) leg's page at result level.
            if !r.is_multi_stay {
                if let Some(first) = r.legs.first() {
                    if let Some(path) = page_map.get(&first.listing_id) {
                        value["page_url"] = serde_json::json!(path);
                    }
                }
            }
            value
        })
        .collect()
}

/// Build a compact `src=…|med=…|camp=…` attribution string from utm params, or
/// `None` when none are present. Used to log ad landings and to stamp the Hostaway
/// reservation note so a booking can be traced back to the campaign that drove it.
fn utm_tag(source: Option<&str>, medium: Option<&str>, campaign: Option<&str>) -> Option<String> {
    let parts: Vec<String> = [("src", source), ("med", medium), ("camp", campaign)]
        .into_iter()
        .filter_map(|(k, v)| {
            v.map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|s| format!("{k}={s}"))
        })
        .collect();
    (!parts.is_empty()).then(|| parts.join("|"))
}

/// URL-encoded query fragment (`&utm_source=…&utm_campaign=…`) used to thread
/// attribution through booking links so a campaign survives /.stay → /.book → POST.
/// Empty when no utm params are set. Values are percent-encoded (safe in href + JS).
fn utm_query(source: Option<&str>, medium: Option<&str>, campaign: Option<&str>) -> String {
    let mut out = String::new();
    for (key, value) in [
        ("utm_source", source),
        ("utm_medium", medium),
        ("utm_campaign", campaign),
    ] {
        if let Some(v) = value.map(str::trim).filter(|s| !s.is_empty()) {
            out.push('&');
            out.push_str(key);
            out.push('=');
            out.push_str(&urlencoding::encode(v));
        }
    }
    out
}

fn render(state: &AppState, template: &str, context: &Context) -> Result<Html<String>, StatusCode> {
    state
        .templates
        .render(template, context)
        .map(Html)
        .map_err(|e| {
            tracing::error!("Failed to render {}: {:?}", template, e);
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

// ---------------------------------------------------------------------------
// /.stay — availability search
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct StayQuery {
    #[serde(default, alias = "checkin")]
    pub from: Option<String>,
    #[serde(default, alias = "checkout")]
    pub to: Option<String>,
    #[serde(default, alias = "guests")]
    pub adults: Option<i64>,
    #[serde(default, alias = "kids")]
    pub children: Option<i64>,
    #[serde(default)]
    pub infants: Option<i64>,
    #[serde(default)]
    pub utm_source: Option<String>,
    #[serde(default)]
    pub utm_medium: Option<String>,
    #[serde(default)]
    pub utm_campaign: Option<String>,
}

pub async fn stay_handler(
    Host(host): Host,
    State(state): State<AppState>,
    SiteDatabase(db): SiteDatabase,
    locale: RequestLocale,
    Query(q): Query<StayQuery>,
) -> Result<Response, StatusCode> {
    let site = load_site(&db, &host).await?;
    let mut context = booking_context(&state, &db, &site, &locale).await?;

    // Marketing attribution: log ad landings, and carry the utm params through the
    // search form / result links so they survive to /.book (see templates).
    let utm = utm_tag(
        q.utm_source.as_deref(),
        q.utm_medium.as_deref(),
        q.utm_campaign.as_deref(),
    );
    if let Some(ref utm) = utm {
        tracing::info!(target: "attribution", host = %host, path = "/.stay", utm = %utm);
    }
    context.insert("q_utm_source", &q.utm_source);
    context.insert("q_utm_medium", &q.utm_medium);
    context.insert("q_utm_campaign", &q.utm_campaign);
    context.insert(
        "utm_qs",
        &utm_query(
            q.utm_source.as_deref(),
            q.utm_medium.as_deref(),
            q.utm_campaign.as_deref(),
        ),
    );

    let adults = q.adults.unwrap_or(2).max(1);
    let children = q.children.unwrap_or(0).max(0);
    let infants = q.infants.unwrap_or(0).max(0);
    context.insert("q_from", &q.from);
    context.insert("q_to", &q.to);
    context.insert("q_adults", &adults);
    context.insert("q_children", &children);
    context.insert("q_infants", &infants);

    let repo = BookingRepository::new(db.clone());
    let config = repo
        .get_config()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !config.is_configured() {
        context.insert("not_configured", &true);
        return Ok(render(&state, "booking/stay.html", &context)?.into_response());
    }

    // No dates yet -> just show the search form.
    let (from, to) = match (q.from.as_deref(), q.to.as_deref()) {
        (Some(f), Some(t)) if !f.is_empty() && !t.is_empty() => (f, t),
        _ => return Ok(render(&state, "booking/stay.html", &context)?.into_response()),
    };
    // ISO dates compare lexicographically; reject an empty/reversed range with a
    // clear message instead of letting the service error look like an outage.
    if to <= from {
        context.insert("invalid_dates", &true);
        return Ok(render(&state, "booking/stay.html", &context)?.into_response());
    }
    // Expired deep-link (past check-in, e.g. an old last-minute ad) -> home.
    if is_past_date(from) {
        return Ok(redirect_home());
    }
    context.insert("searched", &true);

    let all = repo.list_listings().await.unwrap_or_default();
    let page_map: HashMap<i64, String> = all
        .iter()
        .filter_map(|l| l.page_path.clone().map(|p| (l.listing_id, p)))
        .collect();
    let primary_ids: Vec<i64> = all
        .iter()
        .filter(|l| l.role == "primary")
        .map(|l| l.listing_id)
        .collect();
    let secondary_ids: Vec<i64> = all
        .iter()
        .filter(|l| l.role == "secondary")
        .map(|l| l.listing_id)
        .collect();
    let client = SejoursClient::new(&config.service_url, &config.service_secret);

    match client
        .availability(from, to, adults, children, infants, &primary_ids)
        .await
    {
        Ok(resp) => {
            context.insert("nights", &resp.nights);
            context.insert("primary_results", &enrich_with_pages(&resp.results, &page_map));
        }
        Err(e) => {
            tracing::error!("availability (primary) failed: {:?}", e);
            context.insert("service_error", &true);
            return Ok(render(&state, "booking/stay.html", &context)?.into_response());
        }
    }

    if !secondary_ids.is_empty() {
        match client
            .availability(from, to, adults, children, infants, &secondary_ids)
            .await
        {
            Ok(resp) => {
                context.insert("secondary_results", &enrich_with_pages(&resp.results, &page_map))
            }
            Err(e) => tracing::warn!("availability (secondary) failed: {:?}", e),
        }
    }

    Ok(render(&state, "booking/stay.html", &context)?.into_response())
}

// ---------------------------------------------------------------------------
// /.book — quote (GET) + create (POST)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct BookQuery {
    pub listing: i64,
    pub from: String,
    pub to: String,
    #[serde(default, alias = "guests")]
    pub adults: Option<i64>,
    #[serde(default, alias = "kids")]
    pub children: Option<i64>,
    #[serde(default)]
    pub infants: Option<i64>,
    #[serde(default)]
    pub utm_source: Option<String>,
    #[serde(default)]
    pub utm_medium: Option<String>,
    #[serde(default)]
    pub utm_campaign: Option<String>,
}

pub async fn book_quote_handler(
    Host(host): Host,
    State(state): State<AppState>,
    SiteDatabase(db): SiteDatabase,
    locale: RequestLocale,
    Query(q): Query<BookQuery>,
) -> Result<Response, StatusCode> {
    let site = load_site(&db, &host).await?;
    let mut context = booking_context(&state, &db, &site, &locale).await?;

    let adults = q.adults.unwrap_or(2).max(1);
    let children = q.children.unwrap_or(0).max(0);
    let infants = q.infants.unwrap_or(0).max(0);

    let repo = BookingRepository::new(db.clone());
    let config = repo
        .get_config()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !config.is_configured() {
        context.insert("not_configured", &true);
        return Ok(render(&state, "booking/book.html", &context)?.into_response());
    }
    // Expired deep-link (past check-in) -> home, never an empty quote page.
    if is_past_date(&q.from) {
        return Ok(redirect_home());
    }

    let client = SejoursClient::new(&config.service_url, &config.service_secret);
    match client
        .quote(q.listing, &q.from, &q.to, adults, children, infants)
        .await
    {
        Ok(quote) => context.insert("quote", &quote),
        Err(e) => {
            tracing::error!("quote failed: {:?}", e);
            context.insert("service_error", &true);
        }
    }

    // Availability calendar for the date picker (blocked dates, like Airbnb).
    if let Ok(cal) = client.calendar(q.listing).await {
        context.insert("blocked_dates", &cal.blocked);
        context.insert("cal_min", &cal.min_date);
        context.insert("cal_max", &cal.max_date);
    }

    context.insert("listing_id", &q.listing);
    context.insert("q_from", &q.from);
    context.insert("q_to", &q.to);
    context.insert("q_adults", &adults);
    context.insert("q_children", &children);
    context.insert("q_infants", &infants);
    // Carry attribution into the booking form so it reaches the POST (and the note).
    context.insert("q_utm_source", &q.utm_source);
    context.insert("q_utm_medium", &q.utm_medium);
    context.insert("q_utm_campaign", &q.utm_campaign);
    context.insert(
        "utm_qs",
        &utm_query(
            q.utm_source.as_deref(),
            q.utm_medium.as_deref(),
            q.utm_campaign.as_deref(),
        ),
    );

    Ok(render(&state, "booking/book.html", &context)?.into_response())
}

#[derive(Debug, Deserialize)]
pub struct BookForm {
    pub listing_id: i64,
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub adults: Option<i64>,
    #[serde(default)]
    pub children: Option<i64>,
    #[serde(default)]
    pub infants: Option<i64>,
    pub first_name: String,
    pub last_name: String,
    pub email: String,
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub utm_source: Option<String>,
    #[serde(default)]
    pub utm_medium: Option<String>,
    #[serde(default)]
    pub utm_campaign: Option<String>,
}

pub async fn book_create_handler(
    Host(host): Host,
    State(state): State<AppState>,
    SiteDatabase(db): SiteDatabase,
    locale: RequestLocale,
    Form(form): Form<BookForm>,
) -> Result<Response, StatusCode> {
    let site = load_site(&db, &host).await?;
    let mut context = booking_context(&state, &db, &site, &locale).await?;

    let adults = form.adults.unwrap_or(2).max(1);
    let children = form.children.unwrap_or(0).max(0);
    let infants = form.infants.unwrap_or(0).max(0);

    // Echo the request so the template can re-render the form on error.
    context.insert("listing_id", &form.listing_id);
    context.insert("q_from", &form.from);
    context.insert("q_to", &form.to);
    context.insert("q_adults", &adults);
    context.insert("q_children", &children);
    context.insert("q_infants", &infants);

    let repo = BookingRepository::new(db.clone());
    let config = repo
        .get_config()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !config.is_configured() {
        context.insert("not_configured", &true);
        return Ok(render(&state, "booking/book.html", &context)?.into_response());
    }

    let phone = form
        .phone
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let base_note = form
        .note
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    // Stamp the campaign onto the reservation note so a booking is traceable back to
    // the ad that drove it (persisted on Hostaway — doxyde keeps no analytics table).
    let utm = utm_tag(
        form.utm_source.as_deref(),
        form.utm_medium.as_deref(),
        form.utm_campaign.as_deref(),
    );
    if let Some(ref utm) = utm {
        tracing::info!(target: "attribution", host = %host, path = "/.book", utm = %utm, "booking");
    }
    let note = match (base_note, utm) {
        (Some(n), Some(utm)) => Some(format!("{n}\n[utm: {utm}]")),
        (Some(n), None) => Some(n),
        (None, Some(utm)) => Some(format!("[utm: {utm}]")),
        (None, None) => None,
    };
    let contact = Contact {
        first_name: form.first_name.trim().to_string(),
        last_name: form.last_name.trim().to_string(),
        email: form.email.trim().to_string(),
        phone,
    };

    let client = SejoursClient::new(&config.service_url, &config.service_secret);
    match client
        .create_reservation(
            form.listing_id,
            &form.from,
            &form.to,
            adults,
            children,
            infants,
            &contact,
            note.as_deref(),
        )
        .await
    {
        Ok(reservation) => {
            context.insert("confirmed", &true);
            context.insert("reservation", &reservation);
        }
        Err(e) => {
            tracing::error!("reservation failed: {:?}", e);
            context.insert("booking_error", &true);
        }
    }

    Ok(render(&state, "booking/book.html", &context)?.into_response())
}

// ---------------------------------------------------------------------------
// /.booking-config — admin configuration
// ---------------------------------------------------------------------------

pub async fn booking_config_get(
    Host(host): Host,
    State(state): State<AppState>,
    SiteDatabase(db): SiteDatabase,
    user: CurrentUser,
) -> Result<Response, StatusCode> {
    if !user.user.is_admin {
        return Err(StatusCode::FORBIDDEN);
    }
    let site = load_site(&db, &host).await?;

    let mut context = Context::new();
    add_base_context(&mut context, &db, &site, None)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let csrf = get_or_create_csrf_token(&db, &user.session.id, state.config.csrf_token_length)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    context.insert("csrf_token", &csrf.token);

    let repo = BookingRepository::new(db.clone());
    let config = repo
        .get_config()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    context.insert("service_url", &config.service_url);
    context.insert("service_secret", &config.service_secret);

    // Map current selection: listing_id -> (role, page_path).
    let selection: HashMap<i64, (String, Option<String>)> = repo
        .list_listings()
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|l| (l.listing_id, (l.role, l.page_path)))
        .collect();

    // If configured, fetch the full park from the service so the admin can pick.
    if config.is_configured() {
        let client = SejoursClient::new(&config.service_url, &config.service_secret);
        match client.listings().await {
            Ok(listings) => {
                let rows: Vec<serde_json::Value> = listings
                    .into_iter()
                    .map(|l| {
                        let (role, page_path) = selection
                            .get(&l.listing_id)
                            .cloned()
                            .map(|(r, p)| (Some(r), p))
                            .unwrap_or((None, None));
                        serde_json::json!({
                            "listing_id": l.listing_id,
                            "name": l.name.or(l.internal_name).unwrap_or_default(),
                            "person_capacity": l.person_capacity,
                            "currency_code": l.currency_code,
                            "role": role,
                            "page_path": page_path,
                        })
                    })
                    .collect();
                context.insert("service_listings", &rows);
            }
            Err(e) => {
                tracing::warn!("booking-config: listings fetch failed: {:?}", e);
                context.insert("service_error", &true);
            }
        }
    }

    render(&state, "booking/config.html", &context).map(IntoResponse::into_response)
}

pub async fn booking_config_post(
    Host(host): Host,
    State(state): State<AppState>,
    SiteDatabase(db): SiteDatabase,
    user: CurrentUser,
    Form(form): Form<HashMap<String, String>>,
) -> Result<Response, StatusCode> {
    if !user.user.is_admin {
        return Err(StatusCode::FORBIDDEN);
    }
    let _site = load_site(&db, &host).await?;

    // CSRF: compare the session token against the submitted field.
    let expected = get_or_create_csrf_token(&db, &user.session.id, state.config.csrf_token_length)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let provided = form.get("csrf_token").map(String::as_str).unwrap_or("");
    if !expected.verify(provided) {
        return Err(StatusCode::FORBIDDEN);
    }

    let service_url = form.get("service_url").cloned().unwrap_or_default();
    let service_secret = form.get("service_secret").cloned().unwrap_or_default();

    let repo = BookingRepository::new(db.clone());
    repo.set_config(service_url.trim(), service_secret.trim())
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Listing roles arrive as role_<listing_id> = primary|secondary|none.
    let mut primary_pos = 0i64;
    let mut secondary_pos = 0i64;
    let mut listings: Vec<BookingListing> = Vec::new();
    for (key, value) in &form {
        let Some(id_str) = key.strip_prefix("role_") else {
            continue;
        };
        let Ok(listing_id) = id_str.parse::<i64>() else {
            continue;
        };
        let (role, position) = match value.as_str() {
            "primary" => {
                let p = primary_pos;
                primary_pos += 1;
                ("primary", p)
            }
            "secondary" => {
                let p = secondary_pos;
                secondary_pos += 1;
                ("secondary", p)
            }
            _ => continue, // "none" / unknown -> not offered
        };
        let page_path = form
            .get(&format!("page_{listing_id}"))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        listings.push(BookingListing {
            listing_id,
            role: role.to_string(),
            position,
            page_path,
        });
    }
    repo.replace_listings(&listings)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Redirect::to("/.booking-config").into_response())
}

#[cfg(test)]
mod tests {
    use super::is_past_date;

    #[test]
    fn past_date_is_past() {
        assert!(is_past_date("2020-01-01"));
    }

    #[test]
    fn future_date_is_not_past() {
        assert!(!is_past_date("2099-12-31"));
    }

    #[test]
    fn unparseable_date_is_not_past() {
        // Garbage falls through to the normal flow rather than redirecting.
        assert!(!is_past_date(""));
        assert!(!is_past_date("not-a-date"));
    }
}
