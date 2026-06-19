// UI-label resolution (layer 1 of i18n).
//
// Resolves the whole catalog (`ui_catalog::UI_LABELS`) into a map for the active
// language, injected into the Tera context as `labels` (referenced in templates
// as `labels["key"]`). Resolution happens here, in async Rust, because Tera
// functions are synchronous and cannot touch the DB during rendering.
//
// Non-source languages go through the deferred translation path: a cached
// translation is served if present, otherwise the English source is returned and
// a background job is enqueued (the translation appears on a later load). Because
// the cache key is the hash of the English source, changing a label's source
// text naturally invalidates its translation.

use std::collections::HashMap;

use sqlx::SqlitePool;

use crate::services::deferred_translation::try_translate;
use crate::state::AppState;
use crate::ui_catalog::UI_LABELS;

const LABEL_CONTEXT: &str = "Short UI label for a travel/hospitality website";

/// Resolve every catalog label for `lang`. In the source language this is a pure
/// passthrough (no DB, no service). `site_key` is a per-site discriminator for
/// the background worker's in-flight dedup (the site domain works well).
pub async fn resolve_labels(
    state: &AppState,
    pool: &SqlitePool,
    lang: &str,
    source_lang: &str,
    site_key: &str,
) -> HashMap<String, String> {
    let mut out = HashMap::with_capacity(UI_LABELS.len());

    if lang == source_lang {
        for (key, src) in UI_LABELS {
            out.insert((*key).to_string(), (*src).to_string());
        }
        return out;
    }

    for (key, src) in UI_LABELS {
        let value = try_translate(
            pool,
            &state.translation.in_flight,
            &state.translation.tx,
            site_key,
            src,
            lang,
            Some(LABEL_CONTEXT),
        )
        .await;
        out.insert((*key).to_string(), value);
    }

    out
}
