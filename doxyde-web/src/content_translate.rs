// Editorial-content translation (layer 2 of i18n).
//
// Translates a page's title/description and its components' translatable text
// fields in memory, BEFORE they are inserted into the Tera context. This mirrors
// the existing in-place `blog_summary` mutation, so no template changes are
// needed. Code components are never translated.
//
// Two policies:
//   * Deferred — serve the cached translation or the source immediately, and
//     enqueue a background job (used on the cookie-served bare URL).
//   * BoundedSync — await the translation with a short timeout, batching all
//     strings into one round-trip, falling back to source (used on the canonical
//     /.fr, /.en URLs so crawlers get translated content on the first hit).

use std::time::Duration;

use doxyde_core::models::{component::Component, page::Page};
use sqlx::SqlitePool;

use crate::locale_middleware::RequestLocale;
use crate::services::deferred_translation::try_translate;
use crate::services::translation::translate_batch_sync_bounded;
use crate::state::AppState;

const CONTENT_CONTEXT: &str = "Editorial content for a travel/hospitality website";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranslationPolicy {
    Deferred,
    BoundedSync,
}

/// Translatable text fields inside a component's `content` JSON, by type.
/// `code` (and anything not listed) is never translated.
fn translatable_fields(component_type: &str) -> &'static [&'static str] {
    match component_type {
        "markdown" | "text" => &["text"],
        "html" => &["html"],
        "image" => &["alt_text", "title"],
        _ => &[],
    }
}

/// Where a translated string should be written back.
enum Slot {
    PageTitle,
    PageDescription,
    ComponentTitle(usize),
    ComponentField(usize, &'static str),
}

/// Translate page + component text into `locale.lang`, in place. No-op when the
/// active language is the source language. `site_key` is a per-site discriminator
/// for the deferred worker's in-flight dedup (the site domain works well).
pub async fn translate_page_content(
    state: &AppState,
    db: &SqlitePool,
    locale: &RequestLocale,
    policy: TranslationPolicy,
    site_key: &str,
    page: &mut Page,
    components: &mut [Component],
) {
    if locale.lang == locale.source_lang {
        return;
    }
    let lang = locale.lang.as_str();

    // Pass 1: gather (slot, source string) for every non-empty translatable field.
    let mut slots: Vec<Slot> = Vec::new();
    let mut sources: Vec<String> = Vec::new();

    let push = |slot: Slot, text: &str, slots: &mut Vec<Slot>, sources: &mut Vec<String>| {
        if !text.trim().is_empty() {
            slots.push(slot);
            sources.push(text.to_string());
        }
    };

    push(Slot::PageTitle, &page.title.clone(), &mut slots, &mut sources);
    if let Some(desc) = page.description.clone() {
        push(Slot::PageDescription, &desc, &mut slots, &mut sources);
    }
    for (i, comp) in components.iter().enumerate() {
        if let Some(title) = comp.title.clone() {
            push(Slot::ComponentTitle(i), &title, &mut slots, &mut sources);
        }
        for field in translatable_fields(&comp.component_type) {
            if let Some(text) = comp.content.get(*field).and_then(|v| v.as_str()) {
                push(
                    Slot::ComponentField(i, field),
                    text,
                    &mut slots,
                    &mut sources,
                );
            }
        }
    }

    if sources.is_empty() {
        return;
    }

    // Pass 2: resolve translations per policy.
    let translated: Vec<String> = match policy {
        TranslationPolicy::BoundedSync => {
            let timeout = Duration::from_millis(state.config.i18n_sync_timeout_ms);
            translate_batch_sync_bounded(
                &state.i18n,
                db,
                lang,
                &sources,
                Some(CONTENT_CONTEXT),
                timeout,
            )
            .await
        }
        TranslationPolicy::Deferred => {
            let mut out = Vec::with_capacity(sources.len());
            for src in &sources {
                out.push(
                    try_translate(
                        db,
                        &state.translation.in_flight,
                        &state.translation.tx,
                        site_key,
                        src,
                        lang,
                        Some(CONTENT_CONTEXT),
                    )
                    .await,
                );
            }
            out
        }
    };

    // Pass 3: write translations back into the page/components.
    for (slot, value) in slots.into_iter().zip(translated.into_iter()) {
        match slot {
            Slot::PageTitle => page.title = value,
            Slot::PageDescription => page.description = Some(value),
            Slot::ComponentTitle(i) => {
                if let Some(comp) = components.get_mut(i) {
                    comp.title = Some(value);
                }
            }
            Slot::ComponentField(i, field) => {
                if let Some(comp) = components.get_mut(i) {
                    if let Some(obj) = comp.content.as_object_mut() {
                        obj.insert(field.to_string(), serde_json::Value::String(value));
                    }
                }
            }
        }
    }
}
