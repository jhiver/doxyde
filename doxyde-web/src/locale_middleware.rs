// Locale resolution middleware.
//
// Resolves the serving language for each request from (in precedence order):
//   cookie `lang` -> Accept-Language header -> the site's default language,
// clamped to the site's enabled language set. The resolved `RequestLocale` is
// injected into request extensions for handlers and `add_base_context`.
//
// Mirrors yatoo.travel's precedence (frontend/src/hooks.server.ts) but reads the
// enabled set + default from the per-site SQLite `i18n_config` /
// `i18n_enabled_lang` tables.

use axum::{extract::Request, middleware::Next, response::Response};
use axum_extra::extract::cookie::CookieJar;
use sqlx::{Row, SqlitePool};

use crate::db_middleware::SiteDatabase;
use crate::languages::{get_direction, label_for};
use crate::site_resolver::RequestSiteExt;

/// Cookie that stores the user's preferred language.
pub const LANG_COOKIE: &str = "lang";

/// An enabled language with display metadata (for the switcher / hreflang).
#[derive(Debug, Clone)]
pub struct EnabledLang {
    pub code: String,
    pub label: String,
    pub dir: &'static str,
}

/// Per-site i18n configuration loaded from the site DB.
#[derive(Debug, Clone)]
pub struct I18nSiteConfig {
    pub default_lang: String,
    pub source_lang: String,
    pub enabled: Vec<EnabledLang>,
}

impl I18nSiteConfig {
    fn is_enabled(&self, code: &str) -> bool {
        self.enabled.iter().any(|l| l.code == code)
    }
}

/// Resolved locale for the current request.
#[derive(Debug, Clone)]
pub struct RequestLocale {
    pub lang: String,
    pub dir: &'static str,
    pub source_lang: String,
    pub default_lang: String,
    pub enabled: Vec<EnabledLang>,
}

impl RequestLocale {
    /// A safe default (single source language) used when no site config is
    /// available (e.g. admin contexts or misconfigured sites).
    pub fn source_default(source_lang: &str) -> Self {
        Self {
            lang: source_lang.to_string(),
            dir: get_direction(source_lang),
            source_lang: source_lang.to_string(),
            default_lang: source_lang.to_string(),
            enabled: vec![EnabledLang {
                code: source_lang.to_string(),
                label: label_for(source_lang).to_string(),
                dir: get_direction(source_lang),
            }],
        }
    }
}

/// Load the per-site i18n config, falling back to {en, source en, [en]} if the
/// tables are missing or empty (defensive; they exist after migration 021).
pub async fn load_site_i18n(pool: &SqlitePool) -> I18nSiteConfig {
    let fallback = || I18nSiteConfig {
        default_lang: "en".to_string(),
        source_lang: "en".to_string(),
        enabled: vec![EnabledLang {
            code: "en".to_string(),
            label: label_for("en").to_string(),
            dir: get_direction("en"),
        }],
    };

    let (default_lang, source_lang) = match sqlx::query(
        "SELECT default_lang, source_lang FROM i18n_config WHERE id = 1",
    )
    .fetch_optional(pool)
    .await
    {
        Ok(Some(row)) => (
            row.try_get::<String, _>("default_lang")
                .unwrap_or_else(|_| "en".to_string()),
            row.try_get::<String, _>("source_lang")
                .unwrap_or_else(|_| "en".to_string()),
        ),
        Ok(None) => return fallback(),
        Err(e) => {
            tracing::warn!(error = %e, "i18n_config read failed; using defaults");
            return fallback();
        }
    };

    let enabled = match sqlx::query(
        "SELECT lang FROM i18n_enabled_lang ORDER BY position, lang",
    )
    .fetch_all(pool)
    .await
    {
        Ok(rows) => rows
            .into_iter()
            .filter_map(|r| r.try_get::<String, _>("lang").ok())
            .map(|code| EnabledLang {
                label: label_for(&code).to_string(),
                dir: get_direction(&code),
                code,
            })
            .collect::<Vec<_>>(),
        Err(e) => {
            tracing::warn!(error = %e, "i18n_enabled_lang read failed; using defaults");
            return fallback();
        }
    };

    if enabled.is_empty() {
        return fallback();
    }

    I18nSiteConfig {
        default_lang,
        source_lang,
        enabled,
    }
}

/// Parse an Accept-Language header and return the first enabled language.
/// Ported from yatoo's detectLanguage (q-value aware).
pub fn detect_accept_language(header: &str, cfg: &I18nSiteConfig) -> Option<String> {
    let mut entries: Vec<(String, f32)> = header
        .split(',')
        .filter_map(|part| {
            let mut bits = part.trim().split(';');
            let tag = bits.next()?.trim();
            if tag.is_empty() {
                return None;
            }
            // primary subtag, lowercased (e.g. "fr-CH" -> "fr")
            let lang = tag.split('-').next()?.to_lowercase();
            let q = bits
                .find_map(|p| {
                    let p = p.trim();
                    p.strip_prefix("q=").and_then(|v| v.trim().parse::<f32>().ok())
                })
                .unwrap_or(1.0);
            Some((lang, q))
        })
        .collect();

    // Stable sort by descending q-value.
    entries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    entries
        .into_iter()
        .map(|(lang, _)| lang)
        .find(|lang| cfg.is_enabled(lang))
}

/// Resolve the serving language from cookie -> Accept-Language -> default,
/// clamped to the enabled set.
pub fn resolve_lang(
    cfg: &I18nSiteConfig,
    cookie_lang: Option<&str>,
    accept_language: Option<&str>,
) -> String {
    if let Some(c) = cookie_lang {
        if cfg.is_enabled(c) {
            return c.to_string();
        }
    }
    if let Some(al) = accept_language {
        if let Some(lang) = detect_accept_language(al, cfg) {
            return lang;
        }
    }
    if cfg.is_enabled(&cfg.default_lang) {
        return cfg.default_lang.clone();
    }
    cfg.enabled
        .first()
        .map(|l| l.code.clone())
        .unwrap_or_else(|| "en".to_string())
}

/// Middleware: resolve the locale and inject `RequestLocale` into extensions.
/// Must run AFTER `database_injection_middleware` (needs the site pool).
pub async fn locale_resolver_middleware(mut request: Request, next: Next) -> Response {
    let pool = request
        .extensions()
        .get::<SiteDatabase>()
        .map(|db| db.0.clone());

    if let Some(pool) = pool {
        let cfg = load_site_i18n(&pool).await;

        let cookie_lang = CookieJar::from_headers(request.headers())
            .get(LANG_COOKIE)
            .map(|c| c.value().to_string());

        let accept_language = request
            .headers()
            .get(axum::http::header::ACCEPT_LANGUAGE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let lang = resolve_lang(&cfg, cookie_lang.as_deref(), accept_language.as_deref());
        let dir = get_direction(&lang);

        let locale = RequestLocale {
            lang,
            dir,
            source_lang: cfg.source_lang,
            default_lang: cfg.default_lang,
            enabled: cfg.enabled,
        };
        request.extensions_mut().insert(locale);
    }

    // Touch site context so the import is always used (and to keep the helper
    // available for future host-aware resolution).
    let _ = request.site_context();

    next.run(request).await
}

/// Extractor: pull the resolved `RequestLocale` from request extensions,
/// falling back to the English source default if absent.
impl<S> axum::extract::FromRequestParts<S> for RequestLocale
where
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        Ok(parts
            .extensions
            .get::<RequestLocale>()
            .cloned()
            .unwrap_or_else(|| RequestLocale::source_default("en")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(langs: &[&str], default: &str) -> I18nSiteConfig {
        I18nSiteConfig {
            default_lang: default.to_string(),
            source_lang: "en".to_string(),
            enabled: langs
                .iter()
                .map(|c| EnabledLang {
                    code: c.to_string(),
                    label: label_for(c).to_string(),
                    dir: get_direction(c),
                })
                .collect(),
        }
    }

    #[test]
    fn accept_language_q_values() {
        let c = cfg(&["en", "fr"], "en");
        assert_eq!(
            detect_accept_language("fr-CH, fr;q=0.9, en;q=0.8", &c),
            Some("fr".to_string())
        );
        assert_eq!(
            detect_accept_language("de;q=0.9, en;q=0.8", &c),
            Some("en".to_string())
        );
        assert_eq!(detect_accept_language("de, es", &c), None);
    }

    #[test]
    fn precedence_cookie_over_accept_language() {
        let c = cfg(&["en", "fr"], "en");
        // cookie wins when enabled
        assert_eq!(resolve_lang(&c, Some("fr"), Some("en")), "fr");
        // disabled cookie is ignored, falls to AL
        assert_eq!(resolve_lang(&c, Some("de"), Some("fr")), "fr");
        // no cookie, no AL -> default
        assert_eq!(resolve_lang(&c, None, None), "en");
        // AL miss -> default
        assert_eq!(resolve_lang(&c, None, Some("de, es")), "en");
    }
}
