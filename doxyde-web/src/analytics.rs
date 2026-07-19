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

//! Per-site analytics tags (GA4 + Meta Pixel) injected into the `<head>` of
//! public pages.
//!
//! The host -> GA4 measurement ID mapping lives here as the single source of
//! truth. Hosts are normalized (lowercase, port stripped, leading `www.`
//! stripped) before lookup, so `www.rusty-pelican.com` and
//! `rusty-pelican.com:443` both resolve to the same site. Unknown hosts get
//! no tags at all.
//!
//! Consent gating is intentionally absent: owner decision recorded in issue
//! aios#0032 (Jalon 0).

use tera::Context;

/// Meta Pixel ID shared by all tracked sites (Twakila + Rusty Pelican).
const META_PIXEL_ID: &str = "1717075462893862";

/// Host (normalized, apex form) -> GA4 measurement ID.
const GA4_BY_HOST: &[(&str, &str)] = &[
    ("twaki.la", "G-V88ZYB7CE0"),
    ("rusty-pelican.com", "G-K86YTR5R76"),
];

/// Normalize a request host for lookup: lowercase, strip the port, strip a
/// leading `www.`.
fn normalize_host(host: &str) -> String {
    let no_port = host.split(':').next().unwrap_or(host);
    let lower = no_port.trim().to_lowercase();
    lower.strip_prefix("www.").unwrap_or(&lower).to_string()
}

/// Return the GA4 measurement ID for a request host, if the site is tracked.
fn ga4_id_for_host(host: &str) -> Option<&'static str> {
    let normalized = normalize_host(host);
    GA4_BY_HOST
        .iter()
        .find(|(h, _)| *h == normalized)
        .map(|(_, id)| *id)
}

/// Build the `<head>` snippet (GA4 gtag + Meta Pixel base code) for a request
/// host. Returns `None` for hosts without a configured GA4 ID: no tags are
/// injected for unknown sites.
pub fn head_snippet(host: &str) -> Option<String> {
    let ga4_id = ga4_id_for_host(host)?;
    Some(format!(
        r#"<!-- Google tag (gtag.js) -->
<script async src="https://www.googletagmanager.com/gtag/js?id={ga4_id}"></script>
<script>window.dataLayer=window.dataLayer||[];function gtag(){{dataLayer.push(arguments);}}gtag('js',new Date());gtag('config','{ga4_id}');</script>
<!-- Meta Pixel -->
<script>!function(f,b,e,v,n,t,s){{if(f.fbq)return;n=f.fbq=function(){{n.callMethod?n.callMethod.apply(n,arguments):n.queue.push(arguments)}};if(!f._fbq)f._fbq=n;n.push=n;n.loaded=!0;n.version='2.0';n.queue=[];t=b.createElement(e);t.async=!0;t.src=v;s=b.getElementsByTagName(e)[0];s.parentNode.insertBefore(t,s)}}(window,document,'script','https://connect.facebook.net/en_US/fbevents.js');fbq('init','{META_PIXEL_ID}');fbq('track','PageView');</script>
<noscript><img height="1" width="1" style="display:none" src="https://www.facebook.com/tr?id={META_PIXEL_ID}&ev=PageView&noscript=1" alt=""/></noscript>"#
    ))
}

/// Insert the `analytics_head` template variable for public page rendering.
/// No-op (variable left undefined) when the host is not a tracked site.
pub fn add_analytics_context(context: &mut Context, host: &str) {
    if let Some(snippet) = head_snippet(host) {
        context.insert("analytics_head", &snippet);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TWAKILA_GA4: &str = "G-V88ZYB7CE0";
    const PELICAN_GA4: &str = "G-K86YTR5R76";

    #[test]
    fn test_twakila_gets_its_ga4_id_and_pixel() {
        let snippet = head_snippet("twaki.la").expect("twaki.la must be tracked");
        assert!(snippet.contains(TWAKILA_GA4));
        assert!(!snippet.contains(PELICAN_GA4));
        assert!(snippet.contains(META_PIXEL_ID));
        assert!(snippet.contains("googletagmanager.com/gtag/js"));
        assert!(snippet.contains("connect.facebook.net"));
        assert!(snippet.contains("<noscript>"));
        assert!(snippet.contains("facebook.com/tr?id="));
    }

    #[test]
    fn test_rusty_pelican_gets_its_ga4_id_and_pixel() {
        let snippet = head_snippet("rusty-pelican.com").expect("rusty-pelican.com must be tracked");
        assert!(snippet.contains(PELICAN_GA4));
        assert!(!snippet.contains(TWAKILA_GA4));
        assert!(snippet.contains(META_PIXEL_ID));
    }

    #[test]
    fn test_www_rusty_pelican_maps_to_same_ga4_id() {
        let snippet =
            head_snippet("www.rusty-pelican.com").expect("www.rusty-pelican.com must be tracked");
        assert!(snippet.contains(PELICAN_GA4));
        assert!(snippet.contains(META_PIXEL_ID));
    }

    #[test]
    fn test_host_normalization_port_and_case() {
        assert_eq!(ga4_id_for_host("TWAKI.LA"), Some(TWAKILA_GA4));
        assert_eq!(ga4_id_for_host("twaki.la:443"), Some(TWAKILA_GA4));
        assert_eq!(
            ga4_id_for_host("WWW.Rusty-Pelican.com:8443"),
            Some(PELICAN_GA4)
        );
    }

    #[test]
    fn test_unknown_host_gets_nothing() {
        assert!(head_snippet("doxyde.com").is_none());
        assert!(head_snippet("strowger.io").is_none());
        assert!(head_snippet("localhost").is_none());
        assert!(head_snippet("").is_none());
        // Suffix/prefix lookalikes must not match.
        assert!(head_snippet("eviltwaki.la").is_none());
        assert!(head_snippet("twaki.la.evil.com").is_none());
    }

    #[test]
    fn test_add_analytics_context_inserts_for_tracked_host() {
        let mut context = Context::new();
        add_analytics_context(&mut context, "twaki.la");
        let value = context
            .get("analytics_head")
            .expect("analytics_head must be set for twaki.la")
            .as_str()
            .expect("analytics_head must be a string");
        assert!(value.contains(TWAKILA_GA4));
    }

    #[test]
    fn test_add_analytics_context_noop_for_unknown_host() {
        let mut context = Context::new();
        add_analytics_context(&mut context, "doxyde.com");
        assert!(context.get("analytics_head").is_none());
    }
}
