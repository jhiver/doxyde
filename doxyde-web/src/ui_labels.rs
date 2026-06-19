// UI-label resolution (layer 1 of i18n).
//
// Resolves the whole catalog (`ui_catalog::UI_LABELS`) into a map for the active
// language, injected into the Tera context as `labels` (referenced in templates
// as `labels["key"]`). Resolution happens here, in async Rust, because Tera
// functions are synchronous and cannot touch the DB during rendering.
//
// Non-source languages follow the same two policies as the page content. On the
// cookie-served bare URL (`Deferred`) a cached translation is served if present,
// otherwise the English source is returned and a background job is enqueued (the
// translation appears on a later load). On the canonical `/.fr` `/.en` URLs
// (`BoundedSync`) the labels are translated synchronously in one batch round-trip
// so crawlers see them translated on the first hit. Because the cache key is the
// hash of the English source, changing a label's source text naturally
// invalidates its translation.

use std::collections::HashMap;

use sqlx::SqlitePool;

use crate::content_translate::TranslationPolicy;
use crate::services::deferred_translation::try_translate;
use crate::services::translation::translate_batch_sync_bounded;
use crate::state::AppState;
use crate::ui_catalog::UI_LABELS;

const LABEL_CONTEXT: &str = "Short UI label for a travel/hospitality website";

/// Resolve every catalog label for `lang`. In the source language this is a pure
/// passthrough (no DB, no service). `site_key` is a per-site discriminator for
/// the background worker's in-flight dedup (the site domain works well). `policy`
/// selects bounded-synchronous (canonical URLs) vs deferred (bare URL) resolution.
pub async fn resolve_labels(
    state: &AppState,
    pool: &SqlitePool,
    lang: &str,
    source_lang: &str,
    site_key: &str,
    policy: TranslationPolicy,
) -> HashMap<String, String> {
    let mut out = HashMap::with_capacity(UI_LABELS.len());

    if lang == source_lang {
        for (key, src) in UI_LABELS {
            out.insert((*key).to_string(), (*src).to_string());
        }
        return out;
    }

    match policy {
        TranslationPolicy::BoundedSync => {
            let sources: Vec<String> =
                UI_LABELS.iter().map(|(_, src)| (*src).to_string()).collect();
            let translated = translate_batch_sync_bounded(
                state,
                pool,
                lang,
                &sources,
                Some(LABEL_CONTEXT),
                site_key,
            )
            .await;
            for ((key, _), value) in UI_LABELS.iter().zip(translated) {
                out.insert((*key).to_string(), value);
            }
        }
        TranslationPolicy::Deferred => {
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
        }
    }

    out
}
