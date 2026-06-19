// Bounded-synchronous translation, used by the canonical per-language URLs
// (/.fr, /.en). Unlike the deferred path, this awaits the i18n service (with a
// short timeout) so a crawler receives translated content on the first hit.
// On timeout or error it falls back to the source text WITHOUT writing a failure
// sentinel, so a later deferred pass can still succeed.

use std::time::Duration;

use sqlx::{Row, SqlitePool};

use super::deferred_translation::{content_hash, store_success};
use super::i18n::I18nClient;

/// Look up a single (lang, hash) in the cache. Returns the translation if a
/// non-failed row exists.
async fn cache_lookup(pool: &SqlitePool, lang: &str, hash: &str) -> Option<String> {
    let row = sqlx::query(
        "SELECT translated_content, is_failed FROM translation_cache \
         WHERE lang = ? AND content_hash = ?",
    )
    .bind(lang)
    .bind(hash)
    .fetch_optional(pool)
    .await
    .ok()??;
    let is_failed: i64 = row.try_get("is_failed").unwrap_or(0);
    if is_failed != 0 {
        return None;
    }
    row.try_get::<String, _>("translated_content").ok()
}

/// Translate a single string with a bounded wait. Returns the translation on
/// success (cached or fresh within the timeout), else the source text.
pub async fn translate_sync_bounded(
    client: &I18nClient,
    pool: &SqlitePool,
    lang: &str,
    content: &str,
    context: Option<&str>,
    timeout: Duration,
) -> String {
    let hash = content_hash(content);
    if let Some(hit) = cache_lookup(pool, lang, &hash).await {
        return hit;
    }
    match tokio::time::timeout(timeout, client.translate(content, Some("en"), lang, context)).await
    {
        Ok(Ok(tr)) => {
            store_success(pool, lang, &hash, &tr.translated).await;
            tr.translated
        }
        Ok(Err(e)) => {
            tracing::warn!(error = %e, lang, "bounded-sync translate failed; serving source");
            content.to_string()
        }
        Err(_) => {
            tracing::warn!(lang, "bounded-sync translate timed out; serving source");
            content.to_string()
        }
    }
}

/// Translate many strings with a single bounded wait, batching the cache misses
/// into one `translate_batch` round-trip. Returns a Vec aligned with `items`,
/// each entry being the translation (cached or fresh) or the source on failure.
pub async fn translate_batch_sync_bounded(
    client: &I18nClient,
    pool: &SqlitePool,
    lang: &str,
    items: &[String],
    context: Option<&str>,
    timeout: Duration,
) -> Vec<String> {
    // Resolve hits from cache; collect misses (with their original index).
    let mut out: Vec<Option<String>> = Vec::with_capacity(items.len());
    let mut miss_idx: Vec<usize> = Vec::new();
    let mut miss_hashes: Vec<String> = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let hash = content_hash(item);
        if let Some(hit) = cache_lookup(pool, lang, &hash).await {
            out.push(Some(hit));
        } else {
            out.push(None);
            miss_idx.push(i);
            miss_hashes.push(hash);
        }
    }

    if !miss_idx.is_empty() {
        let miss_refs: Vec<&str> = miss_idx.iter().map(|&i| items[i].as_str()).collect();
        match tokio::time::timeout(
            timeout,
            client.translate_batch(&miss_refs, Some("en"), lang, context),
        )
        .await
        {
            Ok(Ok(results)) if results.len() == miss_idx.len() => {
                for (k, res) in results.into_iter().enumerate() {
                    let idx = miss_idx[k];
                    store_success(pool, lang, &miss_hashes[k], &res.translated).await;
                    out[idx] = Some(res.translated);
                }
            }
            Ok(Ok(_)) => {
                tracing::warn!(lang, "batch translate returned mismatched count; serving source");
            }
            Ok(Err(e)) => {
                tracing::warn!(error = %e, lang, "batch translate failed; serving source");
            }
            Err(_) => {
                tracing::warn!(lang, "batch translate timed out; serving source");
            }
        }
    }

    out.into_iter()
        .enumerate()
        .map(|(i, v)| v.unwrap_or_else(|| items[i].clone()))
        .collect()
}
