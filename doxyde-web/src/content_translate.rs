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

use std::collections::HashMap;

use doxyde_core::models::{component::Component, page::Page};
use sqlx::SqlitePool;

use crate::locale_middleware::RequestLocale;
use crate::services::deferred_translation::try_translate;
use crate::services::translation::translate_batch_sync_bounded;
use crate::state::AppState;

const CONTENT_CONTEXT: &str = "Editorial content for a travel/hospitality website";
const TITLE_CONTEXT: &str = "Short navigation/menu title for a travel/hospitality website";

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
            translate_batch_sync_bounded(state, db, lang, &sources, Some(CONTENT_CONTEXT), site_key)
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

/// Translate a set of unique strings into `lang` per policy, returning a
/// source -> translation map. Strings already in the source language are not
/// passed here (callers gate on that).
async fn translate_unique(
    state: &AppState,
    db: &SqlitePool,
    lang: &str,
    policy: TranslationPolicy,
    site_key: &str,
    context_hint: &str,
    uniques: Vec<String>,
) -> HashMap<String, String> {
    let translations = match policy {
        TranslationPolicy::BoundedSync => {
            translate_batch_sync_bounded(state, db, lang, &uniques, Some(context_hint), site_key)
                .await
        }
        TranslationPolicy::Deferred => {
            let mut out = Vec::with_capacity(uniques.len());
            for src in &uniques {
                out.push(
                    try_translate(
                        db,
                        &state.translation.in_flight,
                        &state.translation.tx,
                        site_key,
                        src,
                        lang,
                        Some(context_hint),
                    )
                    .await,
                );
            }
            out
        }
    };
    uniques.into_iter().zip(translations.into_iter()).collect()
}

/// Translate the navigation/breadcrumb title strings already inserted into the
/// Tera context (`root_page_title`, `nav_items`, `breadcrumbs`,
/// `navigation_levels`, `children`). These titles are borrowed from other pages
/// and are not covered by `translate_page_content`. No-op in the source language.
pub async fn translate_context_titles(
    context: &mut tera::Context,
    state: &AppState,
    db: &SqlitePool,
    locale: &RequestLocale,
    policy: TranslationPolicy,
    site_key: &str,
) {
    if locale.lang == locale.source_lang {
        return;
    }
    let lang = locale.lang.as_str();

    // Keys holding arrays of objects with a "title" field.
    const ARRAY_KEYS: &[&str] = &["nav_items", "breadcrumbs", "children"];

    // Pass 1: collect unique non-empty titles across all structures.
    let mut uniques: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let note = |t: &str, uniques: &mut Vec<String>, seen: &mut std::collections::HashSet<String>| {
        if !t.trim().is_empty() && seen.insert(t.to_string()) {
            uniques.push(t.to_string());
        }
    };

    if let Some(s) = context.get("root_page_title").and_then(|v| v.as_str()) {
        note(s, &mut uniques, &mut seen);
    }
    for key in ARRAY_KEYS {
        if let Some(arr) = context.get(*key).and_then(|v| v.as_array()) {
            for item in arr {
                if let Some(t) = item.get("title").and_then(|v| v.as_str()) {
                    note(t, &mut uniques, &mut seen);
                }
            }
        }
    }
    if let Some(arr) = context.get("navigation_levels").and_then(|v| v.as_array()) {
        for level in arr {
            if let Some(t) = level.get("title").and_then(|v| v.as_str()) {
                note(t, &mut uniques, &mut seen);
            }
            if let Some(pages) = level.get("pages").and_then(|v| v.as_array()) {
                for p in pages {
                    if let Some(t) = p.get("title").and_then(|v| v.as_str()) {
                        note(t, &mut uniques, &mut seen);
                    }
                }
            }
        }
    }

    if uniques.is_empty() {
        return;
    }

    let map = translate_unique(state, db, lang, policy, site_key, TITLE_CONTEXT, uniques).await;
    let tr = |v: &serde_json::Value| -> Option<String> {
        v.as_str().map(|s| map.get(s).cloned().unwrap_or_else(|| s.to_string()))
    };

    // Pass 2: rewrite each structure with translated titles, re-inserting.
    if let Some(s) = context.get("root_page_title").and_then(|v| v.as_str()) {
        let t = map.get(s).cloned().unwrap_or_else(|| s.to_string());
        context.insert("root_page_title", &t);
    }
    for key in ARRAY_KEYS {
        if let Some(arr) = context.get(*key).and_then(|v| v.as_array()).cloned() {
            let rewritten: Vec<serde_json::Value> = arr
                .into_iter()
                .map(|mut item| {
                    let new_title = item.get("title").and_then(&tr);
                    if let (Some(obj), Some(t)) = (item.as_object_mut(), new_title) {
                        obj.insert("title".to_string(), serde_json::Value::String(t));
                    }
                    item
                })
                .collect();
            context.insert(*key, &rewritten);
        }
    }
    if let Some(arr) = context.get("navigation_levels").and_then(|v| v.as_array()).cloned() {
        let rewritten: Vec<serde_json::Value> = arr
            .into_iter()
            .map(|mut level| {
                let new_title = level.get("title").and_then(&tr);
                if let Some(t) = new_title {
                    if let Some(obj) = level.as_object_mut() {
                        obj.insert("title".to_string(), serde_json::Value::String(t));
                    }
                }
                if let Some(pages) = level.get("pages").and_then(|v| v.as_array()).cloned() {
                    let new_pages: Vec<serde_json::Value> = pages
                        .into_iter()
                        .map(|mut p| {
                            let pt = p.get("title").and_then(&tr);
                            if let (Some(obj), Some(t)) = (p.as_object_mut(), pt) {
                                obj.insert("title".to_string(), serde_json::Value::String(t));
                            }
                            p
                        })
                        .collect();
                    if let Some(obj) = level.as_object_mut() {
                        obj.insert("pages".to_string(), serde_json::Value::Array(new_pages));
                    }
                }
                level
            })
            .collect();
        context.insert("navigation_levels", &rewritten);
    }
}
