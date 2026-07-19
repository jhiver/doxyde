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

//! First-party ad-attribution capture (marketing conversion spine, J0.1/J0.2).
//!
//! Any request landing with `gclid` / `fbclid` / `ttclid` / `utm_*` query
//! parameters gets a first-party `_dx_attr` cookie (90 days, last-touch per
//! key) so the click id survives the browse → `/.stay` → `/.book` funnel even
//! when the ad lands on a content page. The booking POST then merges explicit
//! form/URL parameters (highest priority) with this cookie to build the
//! `attribution` object forwarded to sejours-api.

use axum::{
    extract::Request,
    http::{header::SET_COOKIE, HeaderValue},
    middleware::Next,
    response::Response,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::Serialize;

/// Name of the first-party attribution cookie.
pub const ATTR_COOKIE: &str = "_dx_attr";

/// Attribution keys we capture; anything else in the query string is ignored.
pub const KNOWN_KEYS: [&str; 6] = [
    "gclid",
    "fbclid",
    "ttclid",
    "utm_source",
    "utm_medium",
    "utm_campaign",
];

/// Hard bound on a single captured value (defensive: ad-platform ids are short).
const MAX_VALUE_LEN: usize = 256;

/// Cookie lifetime: 90 days, matching typical ad-platform attribution windows.
const COOKIE_MAX_AGE_DAYS: i64 = 90;

/// Trim an incoming value and bound it to `MAX_VALUE_LEN` characters.
/// Returns `None` for missing or blank input so callers can chain fallbacks.
pub fn clean_value(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.chars().take(MAX_VALUE_LEN).collect())
}

/// Extract the known attribution pairs from a raw query string. Later
/// duplicates win (consistent with last-touch semantics). Unknown keys and
/// blank values are dropped; values are trimmed and bounded.
pub fn capture_from_query(query: &str) -> Vec<(String, String)> {
    let parsed: Vec<(String, String)> = serde_urlencoded::from_str(query).unwrap_or_default();
    let mut out: Vec<(String, String)> = Vec::new();
    for (key, value) in parsed {
        if !KNOWN_KEYS.contains(&key.as_str()) {
            continue;
        }
        let Some(value) = clean_value(Some(&value)) else {
            continue;
        };
        out.retain(|(k, _)| *k != key);
        out.push((key, value));
    }
    out
}

/// Parse a `_dx_attr` cookie payload (`k=v|k=v`, percent-encoded values) back
/// into pairs. Unknown keys and malformed segments are silently dropped so a
/// stale or tampered cookie can never break a request.
pub fn parse_cookie_value(raw: &str) -> Vec<(String, String)> {
    raw.split('|')
        .filter_map(|segment| {
            let (key, value) = segment.split_once('=')?;
            if !KNOWN_KEYS.contains(&key) {
                return None;
            }
            let decoded = urlencoding::decode(value).ok()?;
            let cleaned = clean_value(Some(&decoded))?;
            Some((key.to_string(), cleaned))
        })
        .collect()
}

/// Serialize pairs into the compact `k=v|k=v` cookie payload. Values are
/// percent-encoded so `|`, `=`, `;` or spaces inside a value cannot corrupt
/// the format.
pub fn encode_cookie_value(pairs: &[(String, String)]) -> String {
    pairs
        .iter()
        .map(|(k, v)| format!("{k}={}", urlencoding::encode(v)))
        .collect::<Vec<_>>()
        .join("|")
}

/// Last-touch merge: for every known key, an incoming value overwrites the
/// existing one; keys absent from the incoming set keep their stored value.
/// Output is in [`KNOWN_KEYS`] order for determinism.
pub fn merge_last_touch(
    existing: &[(String, String)],
    incoming: &[(String, String)],
) -> Vec<(String, String)> {
    let get = |pairs: &[(String, String)], key: &str| {
        pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
    };
    KNOWN_KEYS
        .iter()
        .filter_map(|key| {
            get(incoming, key)
                .or_else(|| get(existing, key))
                .map(|v| (key.to_string(), v))
        })
        .collect()
}

/// Build the `Set-Cookie` header value for a request, or `None` when the query
/// string carries no attribution parameter (the common case: zero overhead).
fn build_set_cookie(request: &Request) -> Option<HeaderValue> {
    let captured = request.uri().query().map(capture_from_query)?;
    if captured.is_empty() {
        return None;
    }
    let existing = CookieJar::from_headers(request.headers())
        .get(ATTR_COOKIE)
        .map(|c| parse_cookie_value(c.value()))
        .unwrap_or_default();
    let merged = merge_last_touch(&existing, &captured);
    let cookie = Cookie::build((ATTR_COOKIE, encode_cookie_value(&merged)))
        .path("/")
        .same_site(SameSite::Lax)
        .max_age(cookie::time::Duration::days(COOKIE_MAX_AGE_DAYS))
        .http_only(false)
        .build();
    cookie.to_string().parse().ok()
}

/// Axum middleware: capture ad click ids / UTM parameters from any landing
/// URL into the `_dx_attr` first-party cookie (90 days, last-touch per key).
pub async fn attribution_capture_middleware(request: Request, next: Next) -> Response {
    let set_cookie = build_set_cookie(&request);
    let mut response = next.run(request).await;
    if let Some(value) = set_cookie {
        response.headers_mut().append(SET_COOKIE, value);
    }
    response
}

/// The merged attribution of a booking request, sent to sejours-api under the
/// `attribution` key. All fields optional; absent fields are omitted from JSON.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct Attribution {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gclid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fbclid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttclid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utm_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utm_medium: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utm_campaign: Option<String>,
}

impl Attribution {
    /// True when no field is set (nothing worth forwarding).
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// Fill every unset field from cookie pairs (explicit URL/form parameters
    /// keep priority over the `_dx_attr` fallback).
    pub fn or_from_pairs(mut self, pairs: &[(String, String)]) -> Self {
        for (key, value) in pairs {
            let slot = match key.as_str() {
                "gclid" => &mut self.gclid,
                "fbclid" => &mut self.fbclid,
                "ttclid" => &mut self.ttclid,
                "utm_source" => &mut self.utm_source,
                "utm_medium" => &mut self.utm_medium,
                "utm_campaign" => &mut self.utm_campaign,
                _ => continue,
            };
            if slot.is_none() {
                *slot = clean_value(Some(value));
            }
        }
        self
    }

    /// JSON object with only the present fields, or `None` when empty so the
    /// caller can omit the `attribution` key entirely.
    pub fn to_json(&self) -> Option<serde_json::Value> {
        if self.is_empty() {
            return None;
        }
        serde_json::to_value(self).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(items: &[(&str, &str)]) -> Vec<(String, String)> {
        items
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn capture_keeps_known_keys_only() {
        let got = capture_from_query("gclid=abc&foo=bar&utm_source=google&page=2");
        assert_eq!(got, pairs(&[("gclid", "abc"), ("utm_source", "google")]));
    }

    #[test]
    fn capture_ignores_blank_values_and_trims() {
        let got = capture_from_query("gclid=%20%20&fbclid=%20xy%20");
        assert_eq!(got, pairs(&[("fbclid", "xy")]));
    }

    #[test]
    fn capture_bounds_values_to_256_chars() {
        let long = "x".repeat(500);
        let got = capture_from_query(&format!("ttclid={long}"));
        assert_eq!(got.len(), 1);
        let (key, value) = &got[0];
        assert_eq!(key, "ttclid");
        assert_eq!(value.chars().count(), 256);
    }

    #[test]
    fn capture_duplicate_key_last_wins() {
        let got = capture_from_query("gclid=first&gclid=second");
        assert_eq!(got, pairs(&[("gclid", "second")]));
    }

    #[test]
    fn capture_empty_query_is_empty() {
        assert!(capture_from_query("").is_empty());
        assert!(capture_from_query("page=2&sort=asc").is_empty());
    }

    #[test]
    fn cookie_roundtrip_preserves_special_chars() {
        let original = pairs(&[("gclid", "a|b=c; d"), ("utm_campaign", "été 2026")]);
        let encoded = encode_cookie_value(&original);
        assert!(!encoded.contains(';'));
        assert_eq!(parse_cookie_value(&encoded), original);
    }

    #[test]
    fn parse_cookie_drops_unknown_and_malformed_segments() {
        let got = parse_cookie_value("gclid=ok|evil=1|garbage|=x|fbclid=fb1");
        assert_eq!(got, pairs(&[("gclid", "ok"), ("fbclid", "fb1")]));
    }

    #[test]
    fn merge_overwrites_last_touch_and_keeps_others() {
        let existing = pairs(&[("gclid", "old"), ("utm_source", "google")]);
        let incoming = pairs(&[("gclid", "new"), ("fbclid", "fb")]);
        let merged = merge_last_touch(&existing, &incoming);
        assert_eq!(
            merged,
            pairs(&[("gclid", "new"), ("fbclid", "fb"), ("utm_source", "google")])
        );
    }

    #[test]
    fn attribution_url_params_beat_cookie() {
        let from_form = Attribution {
            gclid: Some("form-gclid".to_string()),
            ..Default::default()
        };
        let cookie = pairs(&[("gclid", "cookie-gclid"), ("utm_source", "google")]);
        let merged = from_form.or_from_pairs(&cookie);
        assert_eq!(merged.gclid.as_deref(), Some("form-gclid"));
        assert_eq!(merged.utm_source.as_deref(), Some("google"));
        assert!(merged.fbclid.is_none());
    }

    #[test]
    fn attribution_json_omits_absent_fields() {
        let attr = Attribution {
            gclid: Some("g1".to_string()),
            utm_source: Some("google".to_string()),
            ..Default::default()
        };
        let json = attr.to_json().expect("non-empty attribution serializes");
        assert_eq!(
            json,
            serde_json::json!({"gclid": "g1", "utm_source": "google"})
        );
    }

    #[test]
    fn attribution_empty_yields_no_json() {
        assert!(Attribution::default().to_json().is_none());
        assert!(Attribution::default().is_empty());
    }

    #[tokio::test]
    async fn middleware_sets_cookie_on_click_id_landing() {
        use axum::http::StatusCode;
        use axum_test::TestServer;

        let state = crate::test_helpers::create_test_app_state()
            .await
            .expect("test state");
        let app = crate::routes::create_router(state);
        let server = TestServer::new(app).expect("test server");

        let response = server
            .get("/.health")
            .add_query_param("gclid", "G123")
            .await;
        response.assert_status(StatusCode::OK);
        let cookie = response.cookie(ATTR_COOKIE);
        assert_eq!(cookie.value(), "gclid=G123");
    }

    #[tokio::test]
    async fn middleware_merges_with_existing_cookie_last_touch() {
        use axum::http::StatusCode;
        use axum_test::TestServer;

        let state = crate::test_helpers::create_test_app_state()
            .await
            .expect("test state");
        let app = crate::routes::create_router(state);
        let mut server = TestServer::new(app).expect("test server");
        server.save_cookies();

        // First landing: Google click.
        server
            .get("/.health")
            .add_query_param("gclid", "G1")
            .add_query_param("utm_source", "google")
            .await
            .assert_status(StatusCode::OK);
        // Second landing: Meta click overwrites nothing shared, adds fbclid,
        // and a fresh gclid overwrites the stored one (last touch).
        let response = server
            .get("/.health")
            .add_query_param("gclid", "G2")
            .add_query_param("fbclid", "F1")
            .await;
        response.assert_status(StatusCode::OK);
        let cookie = response.cookie(ATTR_COOKIE);
        assert_eq!(cookie.value(), "gclid=G2|fbclid=F1|utm_source=google");
    }

    #[tokio::test]
    async fn middleware_no_cookie_without_attribution_params() {
        use axum::http::StatusCode;
        use axum_test::TestServer;

        let state = crate::test_helpers::create_test_app_state()
            .await
            .expect("test state");
        let app = crate::routes::create_router(state);
        let server = TestServer::new(app).expect("test server");

        let response = server.get("/.health").await;
        response.assert_status(StatusCode::OK);
        assert!(response.maybe_cookie(ATTR_COOKIE).is_none());
    }
}
