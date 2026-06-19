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

/// Every source string on a page that the renderer would translate: page
/// title/description, each component's title + translatable text fields, and a
/// `blog_summary`'s stored `display_title`. Used by the background pre-warmer.
///
/// A `blog_summary`'s injected child cards (`pages[].title`/`description`) are
/// NOT included here — they are borrowed from child pages, which the warmer
/// visits in their own right, so warming every page covers the card data too.
pub fn collect_warmable_sources(page: &Page, components: &[Component]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut add = |s: &str| {
        if !s.trim().is_empty() {
            out.push(s.to_string());
        }
    };

    add(&page.title);
    if let Some(desc) = &page.description {
        add(desc);
    }
    for comp in components {
        if let Some(title) = &comp.title {
            add(title);
        }
        for field in translatable_fields(&comp.component_type) {
            if let Some(text) = comp.content.get(*field).and_then(|v| v.as_str()) {
                add(text);
            }
        }
        if comp.component_type == "blog_summary" {
            if let Some(dt) = comp.content.get("display_title").and_then(|v| v.as_str()) {
                add(dt);
            }
        }
    }
    out
}

/// Where a translated string should be written back.
enum Slot {
    PageTitle,
    PageDescription,
    ComponentTitle(usize),
    ComponentField(usize, &'static str),
    /// A `blog_summary`'s `display_title` (the listing heading).
    ComponentDisplayTitle(usize),
    /// A field of a `blog_summary`'s injected `pages[j]` entry (`title` /
    /// `description`), borrowed from a child page.
    ComponentPageField(usize, usize, &'static str),
}

/// Collect the translatable strings that `render_page` injects into a
/// `blog_summary` component (`display_title` + each child `pages[].title` /
/// `pages[].description`). These are borrowed from other pages and are not
/// covered by `translatable_fields`, so they are gathered separately.
fn push_blog_summary_slots(i: usize, comp: &Component, push: &mut dyn FnMut(Slot, &str)) {
    if comp.component_type != "blog_summary" {
        return;
    }
    if let Some(dt) = comp.content.get("display_title").and_then(|v| v.as_str()) {
        push(Slot::ComponentDisplayTitle(i), dt);
    }
    if let Some(pages) = comp.content.get("pages").and_then(|v| v.as_array()) {
        for (j, pg) in pages.iter().enumerate() {
            if let Some(t) = pg.get("title").and_then(|v| v.as_str()) {
                push(Slot::ComponentPageField(i, j, "title"), t);
            }
            if let Some(d) = pg.get("description").and_then(|v| v.as_str()) {
                push(Slot::ComponentPageField(i, j, "description"), d);
            }
        }
    }
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

    push(
        Slot::PageTitle,
        &page.title.clone(),
        &mut slots,
        &mut sources,
    );
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
        // blog_summary listings carry child page titles/descriptions injected by
        // render_page; gather them so the cards translate too.
        let mut push_one = |slot: Slot, text: &str| push(slot, text, &mut slots, &mut sources);
        push_blog_summary_slots(i, comp, &mut push_one);
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
    for (slot, value) in slots.into_iter().zip(translated) {
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
            Slot::ComponentDisplayTitle(i) => {
                if let Some(comp) = components.get_mut(i) {
                    if let Some(obj) = comp.content.as_object_mut() {
                        obj.insert(
                            "display_title".to_string(),
                            serde_json::Value::String(value),
                        );
                    }
                }
            }
            Slot::ComponentPageField(i, j, field) => {
                if let Some(comp) = components.get_mut(i) {
                    if let Some(pages) =
                        comp.content.get_mut("pages").and_then(|v| v.as_array_mut())
                    {
                        if let Some(obj) = pages.get_mut(j).and_then(|p| p.as_object_mut()) {
                            obj.insert(field.to_string(), serde_json::Value::String(value));
                        }
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
    uniques.into_iter().zip(translations).collect()
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
    let note =
        |t: &str, uniques: &mut Vec<String>, seen: &mut std::collections::HashSet<String>| {
            if !t.trim().is_empty() && seen.insert(t.to_string()) {
                uniques.push(t.to_string());
            }
        };

    if let Some(s) = context.get("root_page_title").and_then(|v| v.as_str()) {
        note(s, &mut uniques, &mut seen);
    }
    for key in ARRAY_KEYS {
        if let Some(arr) = context.get(key).and_then(|v| v.as_array()) {
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
        v.as_str()
            .map(|s| map.get(s).cloned().unwrap_or_else(|| s.to_string()))
    };

    // Pass 2: rewrite each structure with translated titles, re-inserting.
    if let Some(s) = context.get("root_page_title").and_then(|v| v.as_str()) {
        let t = map.get(s).cloned().unwrap_or_else(|| s.to_string());
        context.insert("root_page_title", &t);
    }
    for key in ARRAY_KEYS {
        if let Some(arr) = context.get(key).and_then(|v| v.as_array()).cloned() {
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
    if let Some(arr) = context
        .get("navigation_levels")
        .and_then(|v| v.as_array())
        .cloned()
    {
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn slot_label(slot: &Slot) -> String {
        match slot {
            Slot::ComponentDisplayTitle(i) => format!("display@{i}"),
            Slot::ComponentPageField(i, j, f) => format!("page@{i}:{j}:{f}"),
            _ => "other".to_string(),
        }
    }

    /// `push_blog_summary_slots` gathers the listing heading and each child
    /// page's title/description, mirroring what `render_page` injects. Empty
    /// strings are filtered by the caller's `push` (replicated here); null
    /// fields are skipped at the source.
    #[test]
    fn blog_summary_slots_collects_heading_and_child_fields() {
        let comp = Component::new(
            1,
            "blog_summary".to_string(),
            0,
            json!({
                "display_title": "Our Apartments",
                "pages": [
                    {"title": "Romantic Retreat", "description": "Cozy 1-bedroom."},
                    {"title": "Private Apt", "description": null},
                    {"title": "", "description": "Only desc."}
                ]
            }),
        );

        let mut collected: Vec<(String, String)> = Vec::new();
        let mut push_one = |slot: Slot, text: &str| {
            if !text.trim().is_empty() {
                collected.push((slot_label(&slot), text.to_string()));
            }
        };
        push_blog_summary_slots(5, &comp, &mut push_one);

        assert_eq!(
            collected,
            vec![
                ("display@5".to_string(), "Our Apartments".to_string()),
                ("page@5:0:title".to_string(), "Romantic Retreat".to_string()),
                (
                    "page@5:0:description".to_string(),
                    "Cozy 1-bedroom.".to_string()
                ),
                ("page@5:1:title".to_string(), "Private Apt".to_string()),
                // page 1 description is null -> skipped at source
                // page 2 title is empty -> filtered by push
                ("page@5:2:description".to_string(), "Only desc.".to_string()),
            ]
        );
    }

    /// Non-`blog_summary` components contribute nothing here (handled by the
    /// generic `translatable_fields` path instead).
    #[test]
    fn blog_summary_slots_ignores_other_components() {
        let comp = Component::new(1, "text".to_string(), 0, json!({"text": "Hello"}));
        let mut count = 0;
        push_blog_summary_slots(0, &comp, &mut |_, _| count += 1);
        assert_eq!(count, 0);
    }
}
