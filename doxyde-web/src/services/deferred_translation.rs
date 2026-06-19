// Deferred (background) translation worker, SQLite-backed.
// Ported from yatoo.travel backend/src/services/deferred_translation.rs,
// adapting Redis -> per-site SQLite. Each job carries its own SqlitePool so the
// worker writes the cached translation into the correct site DB.
//
// Server-side rendering cannot poll, so the read path (`try_translate`) returns
// the string to serve *now*: the translation on a cache hit, otherwise the
// source text (and enqueues a background job). The translation then appears on a
// later page load. Failed rows are retried after a short cooldown.

use std::sync::Arc;

use dashmap::DashSet;
use sha2::{Digest, Sha256};
use sqlx::{Row, SqlitePool};
use tokio::sync::{mpsc, Semaphore};

use super::i18n::I18nClient;

/// Cooldown before a failed translation is retried (seconds).
const FAILED_COOLDOWN_SECS: i64 = 60;

pub struct TranslationJob {
    pub pool: SqlitePool,
    pub site_key: String,
    pub content: String,
    pub target_lang: String,
    pub context: Option<String>,
    pub hash: String,
}

/// SHA-256 (hex) of the source content — the cache key within a language.
pub fn content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

fn in_flight_key(site_key: &str, lang: &str, hash: &str) -> String {
    format!("{site_key}:{lang}:{hash}")
}

/// Resolve a translation for the deferred (cookie-served) path.
///
/// Returns the string to render immediately: the cached translation on a fresh
/// hit, otherwise the source text. On a miss (or stale failure) a background job
/// is enqueued so the translation is available on a later load.
#[allow(clippy::too_many_arguments)]
pub async fn try_translate(
    pool: &SqlitePool,
    in_flight: &DashSet<String>,
    tx: &mpsc::Sender<TranslationJob>,
    site_key: &str,
    content: &str,
    lang: &str,
    context: Option<&str>,
) -> String {
    let hash = content_hash(content);

    let row = sqlx::query(
        "SELECT translated_content, is_failed, \
         (cached_at < datetime('now', ?)) AS stale \
         FROM translation_cache WHERE lang = ? AND content_hash = ?",
    )
    .bind(format!("-{FAILED_COOLDOWN_SECS} seconds"))
    .bind(lang)
    .bind(&hash)
    .fetch_optional(pool)
    .await;

    match row {
        Ok(Some(r)) => {
            let translated: String = r.try_get("translated_content").unwrap_or_default();
            let is_failed: i64 = r.try_get("is_failed").unwrap_or(0);
            let stale: i64 = r.try_get("stale").unwrap_or(0);
            if is_failed == 0 {
                return translated;
            }
            // Failed row: serve source now; re-enqueue if cooldown elapsed.
            if stale == 1 {
                enqueue(in_flight, tx, pool, site_key, content, lang, context, &hash).await;
            }
            return content.to_string();
        }
        Ok(None) => {
            // Cache miss — enqueue a background job (best-effort).
            enqueue(in_flight, tx, pool, site_key, content, lang, context, &hash).await;
            content.to_string()
        }
        Err(e) => {
            tracing::warn!(error = %e, "translation_cache read failed; serving source");
            content.to_string()
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn enqueue(
    in_flight: &DashSet<String>,
    tx: &mpsc::Sender<TranslationJob>,
    pool: &SqlitePool,
    site_key: &str,
    content: &str,
    lang: &str,
    context: Option<&str>,
    hash: &str,
) {
    let key = in_flight_key(site_key, lang, hash);
    if in_flight.insert(key) {
        let job = TranslationJob {
            pool: pool.clone(),
            site_key: site_key.to_string(),
            content: content.to_string(),
            target_lang: lang.to_string(),
            context: context.map(|s| s.to_string()),
            hash: hash.to_string(),
        };
        if let Err(e) = tx.send(job).await {
            tracing::warn!(error = %e, lang, "failed to enqueue translation job");
        }
    }
}

/// Persist a successful translation, never clobbering a manual override.
pub async fn store_success(pool: &SqlitePool, lang: &str, hash: &str, translated: &str) {
    let res = sqlx::query(
        "INSERT INTO translation_cache \
         (lang, content_hash, translated_content, is_manual_override, is_failed, cached_at) \
         VALUES (?, ?, ?, 0, 0, datetime('now')) \
         ON CONFLICT(lang, content_hash) DO UPDATE SET \
         translated_content = excluded.translated_content, \
         is_failed = 0, cached_at = datetime('now') \
         WHERE is_manual_override = 0",
    )
    .bind(lang)
    .bind(hash)
    .bind(translated)
    .execute(pool)
    .await;
    if let Err(e) = res {
        tracing::warn!(error = %e, lang, "failed to store translation");
    }
}

/// Mark a translation as failed (stores the source so reads can serve it),
/// never clobbering a manual override.
async fn store_failure(pool: &SqlitePool, lang: &str, hash: &str, source: &str) {
    let res = sqlx::query(
        "INSERT INTO translation_cache \
         (lang, content_hash, translated_content, is_manual_override, is_failed, cached_at) \
         VALUES (?, ?, ?, 0, 1, datetime('now')) \
         ON CONFLICT(lang, content_hash) DO UPDATE SET \
         is_failed = 1, cached_at = datetime('now') \
         WHERE is_manual_override = 0",
    )
    .bind(lang)
    .bind(hash)
    .bind(source)
    .execute(pool)
    .await;
    if let Err(e) = res {
        tracing::warn!(error = %e, lang, "failed to store translation failure");
    }
}

/// Background worker: drains the job queue, translating with bounded concurrency.
/// Tolerates the i18n service being down (per-job errors, never panics).
pub async fn run_worker(
    mut rx: mpsc::Receiver<TranslationJob>,
    i18n: I18nClient,
    in_flight: Arc<DashSet<String>>,
    semaphore: Arc<Semaphore>,
) {
    tracing::info!("Translation worker started");
    while let Some(job) = rx.recv().await {
        let i18n = i18n.clone();
        let in_flight = in_flight.clone();
        let semaphore = semaphore.clone();

        tokio::spawn(async move {
            let _permit = match semaphore.acquire().await {
                Ok(p) => p,
                Err(_) => return, // semaphore closed
            };

            let key = in_flight_key(&job.site_key, &job.target_lang, &job.hash);

            let result = i18n
                .translate(
                    &job.content,
                    Some("en"),
                    &job.target_lang,
                    job.context.as_deref(),
                )
                .await;

            match result {
                Ok(tr) => {
                    store_success(&job.pool, &job.target_lang, &job.hash, &tr.translated).await;
                    tracing::debug!(lang = %job.target_lang, hash = %job.hash, "Translation completed");
                }
                Err(e) if e.is_transient() => {
                    // Service unreachable: leave the row absent so it is retried
                    // freely on the next request/warm, not gated by the cooldown.
                    tracing::warn!(error = %e, lang = %job.target_lang, "Translation unavailable (transient); not caching failure");
                }
                Err(e) => {
                    tracing::warn!(error = %e, lang = %job.target_lang, "Translation failed");
                    store_failure(&job.pool, &job.target_lang, &job.hash, &job.content).await;
                }
            }

            in_flight.remove(&key);
        });
    }
    tracing::info!("Translation worker stopped");
}
