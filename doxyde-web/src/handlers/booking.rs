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
use axum_extra::extract::{cookie::CookieJar, Host};
use doxyde_core::models::site::Site;
use doxyde_db::repositories::{BookingListing, BookingRepository};
use serde::Deserialize;
use tera::Context;

use crate::{
    attribution::{clean_value, parse_cookie_value, Attribution, ATTR_COOKIE},
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

/// Determine if the client explicitly requested a JSON response.
fn wants_json(headers: &axum::http::HeaderMap, format: Option<&str>) -> bool {
    let accept_json = headers
        .get(axum::http::header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.contains("application/json"))
        .unwrap_or(false);
    accept_json || format == Some("json")
}

fn json_error(code: &str) -> Response {
    axum::Json(serde_json::json!({
        "status": "error",
        "code": code
    }))
    .into_response()
}

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
                        if let (Some(path), Some(obj)) = (page_map.get(&id), leg.as_object_mut()) {
                            obj.insert("page_url".to_string(), serde_json::json!(path));
                        }
                    }
                }
            }
            // Single-stay convenience: surface the (only) leg's page at result level.
            if !r.is_multi_stay {
                if let Some(first) = r.legs.first() {
                    if let (Some(path), Some(obj)) =
                        (page_map.get(&first.listing_id), value.as_object_mut())
                    {
                        obj.insert("page_url".to_string(), serde_json::json!(path));
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

/// URL-encoded query fragment (`&utm_source=…&gclid=…`) used to thread
/// attribution through booking links so a campaign survives /.stay → /.book → POST.
/// Empty when no attribution is set. Values are percent-encoded (safe in href + JS).
fn attribution_query(attr: &Attribution) -> String {
    let mut out = String::new();
    for (key, value) in [
        ("utm_source", attr.utm_source.as_deref()),
        ("utm_medium", attr.utm_medium.as_deref()),
        ("utm_campaign", attr.utm_campaign.as_deref()),
        ("gclid", attr.gclid.as_deref()),
        ("fbclid", attr.fbclid.as_deref()),
        ("ttclid", attr.ttclid.as_deref()),
    ] {
        if let Some(v) = value {
            out.push('&');
            out.push_str(key);
            out.push('=');
            out.push_str(&urlencoding::encode(v));
        }
    }
    out
}

/// Insert the `q_*` attribution echoes + the `utm_qs` link fragment into a
/// booking page context so templates re-thread the campaign through the funnel.
fn insert_attribution_context(context: &mut Context, attr: &Attribution) {
    context.insert("q_utm_source", &attr.utm_source);
    context.insert("q_utm_medium", &attr.utm_medium);
    context.insert("q_utm_campaign", &attr.utm_campaign);
    context.insert("q_gclid", &attr.gclid);
    context.insert("q_fbclid", &attr.fbclid);
    context.insert("q_ttclid", &attr.ttclid);
    context.insert("utm_qs", &attribution_query(attr));
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
    #[serde(default)]
    pub gclid: Option<String>,
    #[serde(default)]
    pub fbclid: Option<String>,
    #[serde(default)]
    pub ttclid: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
}

impl From<&StayQuery> for Attribution {
    fn from(q: &StayQuery) -> Self {
        Self {
            gclid: clean_value(q.gclid.as_deref()),
            fbclid: clean_value(q.fbclid.as_deref()),
            ttclid: clean_value(q.ttclid.as_deref()),
            utm_source: clean_value(q.utm_source.as_deref()),
            utm_medium: clean_value(q.utm_medium.as_deref()),
            utm_campaign: clean_value(q.utm_campaign.as_deref()),
        }
    }
}

pub async fn stay_handler(
    Host(host): Host,
    State(state): State<AppState>,
    SiteDatabase(db): SiteDatabase,
    locale: RequestLocale,
    headers: axum::http::HeaderMap,
    Query(q): Query<StayQuery>,
) -> Result<Response, StatusCode> {
    let json = wants_json(&headers, q.format.as_deref());
    let site = load_site(&db, &host).await?;
    let mut context = booking_context(&state, &db, &site, &locale).await?;

    let utm = utm_tag(
        q.utm_source.as_deref(),
        q.utm_medium.as_deref(),
        q.utm_campaign.as_deref(),
    );
    if let Some(ref utm) = utm {
        tracing::info!(target: "attribution", host = %host, path = "/.stay", utm = %utm);
    }
    insert_attribution_context(&mut context, &Attribution::from(&q));

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
        if json {
            return Ok(json_error("not_configured"));
        }
        context.insert("not_configured", &true);
        return Ok(render(&state, "booking/stay.html", &context)?.into_response());
    }

    let (from, to) = match (q.from.as_deref(), q.to.as_deref()) {
        (Some(f), Some(t)) if !f.is_empty() && !t.is_empty() => (f, t),
        _ => {
            if json {
                return Ok(axum::Json(serde_json::json!({
                    "status": "ok",
                    "results": Vec::<serde_json::Value>::new(),
                    "secondary_results": Vec::<serde_json::Value>::new(),
                    "hint": "provide from/to dates"
                }))
                .into_response());
            }
            return Ok(render(&state, "booking/stay.html", &context)?.into_response());
        }
    };
    if to <= from {
        if json {
            return Ok(json_error("invalid_dates"));
        }
        context.insert("invalid_dates", &true);
        return Ok(render(&state, "booking/stay.html", &context)?.into_response());
    }
    if is_past_date(from) {
        if json {
            return Ok(json_error("past_date"));
        }
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

    let resp = match client
        .availability(from, to, adults, children, infants, &primary_ids)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("availability (primary) failed: {:?}", e);
            if json {
                return Ok(json_error("service_error"));
            }
            context.insert("service_error", &true);
            return Ok(render(&state, "booking/stay.html", &context)?.into_response());
        }
    };

    let primary_results = enrich_with_pages(&resp.results, &page_map);
    context.insert("nights", &resp.nights);
    context.insert("primary_results", &primary_results);

    let mut secondary_results = Vec::new();
    if !secondary_ids.is_empty() {
        match client
            .availability(from, to, adults, children, infants, &secondary_ids)
            .await
        {
            Ok(sec_resp) => {
                secondary_results = enrich_with_pages(&sec_resp.results, &page_map);
                context.insert("secondary_results", &secondary_results);
            }
            Err(e) => {
                tracing::warn!("availability (secondary) failed: {:?}", e);
            }
        }
    }

    if json {
        let currency_code = primary_results
            .first()
            .or_else(|| secondary_results.first())
            .and_then(|v| v.get("currency_code").and_then(|c| c.as_str()))
            .map(|s| s.to_string());
        return Ok(axum::Json(serde_json::json!({
            "status": "ok",
            "nights": resp.nights,
            "currency_code": currency_code,
            "results": primary_results,
            "secondary_results": secondary_results
        }))
        .into_response());
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
    #[serde(default)]
    pub gclid: Option<String>,
    #[serde(default)]
    pub fbclid: Option<String>,
    #[serde(default)]
    pub ttclid: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
}

impl From<&BookQuery> for Attribution {
    fn from(q: &BookQuery) -> Self {
        Self {
            gclid: clean_value(q.gclid.as_deref()),
            fbclid: clean_value(q.fbclid.as_deref()),
            ttclid: clean_value(q.ttclid.as_deref()),
            utm_source: clean_value(q.utm_source.as_deref()),
            utm_medium: clean_value(q.utm_medium.as_deref()),
            utm_campaign: clean_value(q.utm_campaign.as_deref()),
        }
    }
}

pub async fn book_quote_handler(
    Host(host): Host,
    State(state): State<AppState>,
    SiteDatabase(db): SiteDatabase,
    locale: RequestLocale,
    headers: axum::http::HeaderMap,
    Query(q): Query<BookQuery>,
) -> Result<Response, StatusCode> {
    let json = wants_json(&headers, q.format.as_deref());
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
        if json {
            return Ok(json_error("not_configured"));
        }
        context.insert("not_configured", &true);
        return Ok(render(&state, "booking/book.html", &context)?.into_response());
    }
    if q.to <= q.from && json {
        return Ok(json_error("invalid_dates"));
    }
    if is_past_date(&q.from) {
        if json {
            return Ok(json_error("past_date"));
        }
        return Ok(redirect_home());
    }

    let client = SejoursClient::new(&config.service_url, &config.service_secret);
    let quote = match client
        .quote(q.listing, &q.from, &q.to, adults, children, infants)
        .await
    {
        Ok(quote) => quote,
        Err(e) => {
            tracing::error!("quote failed: {:?}", e);
            if json {
                return Ok(json_error("service_error"));
            }
            context.insert("service_error", &true);
            return Ok(render(&state, "booking/book.html", &context)?.into_response());
        }
    };

    let blocked_dates = match client.calendar(q.listing).await {
        Ok(cal) => {
            context.insert("blocked_dates", &cal.blocked);
            context.insert("cal_min", &cal.min_date);
            context.insert("cal_max", &cal.max_date);
            cal.blocked
        }
        Err(e) => {
            tracing::warn!("calendar failed: {:?}", e);
            Vec::new()
        }
    };

    if json {
        return Ok(axum::Json(serde_json::json!({
            "status": "ok",
            "quote": quote,
            "blocked_dates": blocked_dates
        }))
        .into_response());
    }

    context.insert("quote", &quote);
    context.insert("listing_id", &q.listing);
    context.insert("q_from", &q.from);
    context.insert("q_to", &q.to);
    context.insert("q_adults", &adults);
    context.insert("q_children", &children);
    context.insert("q_infants", &infants);
    insert_attribution_context(&mut context, &Attribution::from(&q));

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
    #[serde(default)]
    pub gclid: Option<String>,
    #[serde(default)]
    pub fbclid: Option<String>,
    #[serde(default)]
    pub ttclid: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
}

impl From<&BookForm> for Attribution {
    fn from(f: &BookForm) -> Self {
        Self {
            gclid: clean_value(f.gclid.as_deref()),
            fbclid: clean_value(f.fbclid.as_deref()),
            ttclid: clean_value(f.ttclid.as_deref()),
            utm_source: clean_value(f.utm_source.as_deref()),
            utm_medium: clean_value(f.utm_medium.as_deref()),
            utm_campaign: clean_value(f.utm_campaign.as_deref()),
        }
    }
}

pub async fn book_create_handler(
    Host(host): Host,
    State(state): State<AppState>,
    SiteDatabase(db): SiteDatabase,
    locale: RequestLocale,
    headers: axum::http::HeaderMap,
    jar: CookieJar,
    Form(form): Form<BookForm>,
) -> Result<Response, StatusCode> {
    let json = wants_json(&headers, form.format.as_deref());
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
        if json {
            return Ok(json_error("not_configured"));
        }
        context.insert("not_configured", &true);
        return Ok(render(&state, "booking/book.html", &context)?.into_response());
    }

    if form.to <= form.from && json {
        return Ok(json_error("invalid_dates"));
    }

    if is_past_date(&form.from) && json {
        return Ok(json_error("past_date"));
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
    // Structured attribution: explicit form/URL parameters win, the _dx_attr
    // landing cookie (J0.1) fills the gaps. Sent alongside the legacy note.
    let cookie_pairs = jar
        .get(ATTR_COOKIE)
        .map(|c| parse_cookie_value(c.value()))
        .unwrap_or_default();
    let attribution = Attribution::from(&form).or_from_pairs(&cookie_pairs);
    let attribution_json = attribution.to_json();
    if let Some(ref attr) = attribution_json {
        tracing::info!(target: "attribution", host = %host, path = "/.book", attribution = %attr, "booking attribution");
    }
    let contact = Contact {
        first_name: form.first_name.trim().to_string(),
        last_name: form.last_name.trim().to_string(),
        email: form.email.trim().to_string(),
        phone,
    };

    let client = SejoursClient::new(&config.service_url, &config.service_secret);
    let reservation = match client
        .create_reservation(
            form.listing_id,
            &form.from,
            &form.to,
            adults,
            children,
            infants,
            &contact,
            note.as_deref(),
            attribution_json.as_ref(),
        )
        .await
    {
        Ok(res) => res,
        Err(e) => {
            tracing::error!("reservation failed: {:?}", e);
            if json {
                return Ok(json_error("booking_error"));
            }
            context.insert("booking_error", &true);
            return Ok(render(&state, "booking/book.html", &context)?.into_response());
        }
    };

    if json {
        return Ok(axum::Json(serde_json::json!({
            "status": "confirmed",
            "reservation": reservation
        }))
        .into_response());
    }

    context.insert("confirmed", &true);
    context.insert("reservation", &reservation);

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
    use super::*;

    fn empty_form() -> BookForm {
        BookForm {
            listing_id: 1,
            from: "2099-01-01".to_string(),
            to: "2099-01-05".to_string(),
            adults: None,
            children: None,
            infants: None,
            first_name: "A".to_string(),
            last_name: "B".to_string(),
            email: "a@b.c".to_string(),
            phone: None,
            note: None,
            utm_source: None,
            utm_medium: None,
            utm_campaign: None,
            gclid: None,
            fbclid: None,
            ttclid: None,
            format: None,
        }
    }

    #[test]
    fn book_form_params_beat_cookie_fallback() {
        let mut form = empty_form();
        form.gclid = Some(" form-gclid ".to_string());
        form.utm_source = Some("google".to_string());
        let cookie = vec![
            ("gclid".to_string(), "cookie-gclid".to_string()),
            ("fbclid".to_string(), "cookie-fb".to_string()),
        ];
        let merged = Attribution::from(&form).or_from_pairs(&cookie);
        // Form wins on gclid (and is trimmed), cookie fills fbclid.
        assert_eq!(merged.gclid.as_deref(), Some("form-gclid"));
        assert_eq!(merged.fbclid.as_deref(), Some("cookie-fb"));
        assert_eq!(merged.utm_source.as_deref(), Some("google"));
        assert!(merged.ttclid.is_none());
    }

    #[test]
    fn attribution_query_threads_click_ids() {
        let attr = Attribution {
            gclid: Some("g 1".to_string()),
            utm_source: Some("meta".to_string()),
            ..Default::default()
        };
        assert_eq!(attribution_query(&attr), "&utm_source=meta&gclid=g%201");
        assert_eq!(attribution_query(&Attribution::default()), "");
    }

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
