// Background translation pre-warmer.
//
// At startup, for every known site DB, translate all published page content
// (titles, descriptions, component text) plus the UI-label catalog into each
// enabled non-source language, so a translated page is served warm on the first
// visit instead of flashing the English source.
//
// It reuses the deferred worker: each string goes through `try_translate`, which
// returns immediately and enqueues a job only on a cache miss. This is
// idempotent (cache hits are skipped, the store is ON CONFLICT) and naturally
// throttled by the worker semaphore and the bounded job channel.

use std::collections::HashSet;

use sqlx::SqlitePool;

use crate::content_translate::collect_warmable_sources;
use crate::locale_middleware::{load_site_i18n, I18nSiteConfig};
use crate::services::deferred_translation::try_translate;
use crate::state::AppState;
use crate::ui_catalog::UI_LABELS;
use doxyde_db::repositories::{ComponentRepository, PageRepository, PageVersionRepository};

const CONTENT_CONTEXT: &str = "Editorial content for a travel/hospitality website";
const LABEL_CONTEXT: &str = "Short UI label for a travel/hospitality website";

/// Spawn the background pre-warm over every known site (no-op if disabled).
/// `site_pools_snapshot` already contains every existing site after startup
/// migrations, so no filesystem walk is needed here.
pub fn spawn_prewarm(state: AppState) {
    if !state.config.i18n_prewarm {
        return;
    }
    tokio::spawn(async move {
        let sites = state.db_router.site_pools_snapshot().await;
        tracing::info!(sites = sites.len(), "i18n pre-warm: starting");
        for (site_key, pool) in sites {
            warm_site(&state, &site_key, &pool).await;
        }
        tracing::info!("i18n pre-warm: all sites enqueued");
    });
}

/// Collect every unique editorial source string across a site's published pages.
async fn collect_site_sources(pool: &SqlitePool) -> HashSet<String> {
    let page_repo = PageRepository::new(pool.clone());
    let version_repo = PageVersionRepository::new(pool.clone());
    let component_repo = ComponentRepository::new(pool.clone());

    let mut sources = HashSet::new();
    let pages = page_repo.list_all().await.unwrap_or_default();
    for page in &pages {
        let Some(page_id) = page.id else { continue };
        let components = published_components(&version_repo, &component_repo, page_id).await;
        for s in collect_warmable_sources(page, &components) {
            sources.insert(s);
        }
    }
    sources
}

/// Components of a page's published version (empty if none/error).
async fn published_components(
    version_repo: &PageVersionRepository,
    component_repo: &ComponentRepository,
    page_id: i64,
) -> Vec<doxyde_core::models::component::Component> {
    match version_repo.get_published(page_id).await {
        Ok(Some(version)) => match version.id {
            Some(vid) => component_repo
                .list_by_page_version(vid)
                .await
                .unwrap_or_default(),
            None => Vec::new(),
        },
        _ => Vec::new(),
    }
}

/// Pre-warm one site: translate all content + UI labels into each enabled
/// non-source language by enqueueing cache misses to the deferred worker.
async fn warm_site(state: &AppState, site_key: &str, pool: &SqlitePool) {
    let cfg: I18nSiteConfig = load_site_i18n(pool).await;
    let targets: Vec<String> = cfg
        .enabled
        .iter()
        .map(|l| l.code.clone())
        .filter(|c| *c != cfg.source_lang)
        .collect();
    if targets.is_empty() {
        return;
    }

    let content = collect_site_sources(pool).await;
    let labels: Vec<&str> = UI_LABELS.iter().map(|(_, src)| *src).collect();
    tracing::info!(
        site = %site_key,
        langs = ?targets,
        content = content.len(),
        labels = labels.len(),
        "i18n pre-warm: site"
    );

    for lang in &targets {
        for src in &content {
            warm_one(state, pool, site_key, src, lang, CONTENT_CONTEXT).await;
        }
        for src in &labels {
            warm_one(state, pool, site_key, src, lang, LABEL_CONTEXT).await;
        }
    }
}

/// Enqueue a single (string, lang) for translation if not already cached.
async fn warm_one(
    state: &AppState,
    pool: &SqlitePool,
    site_key: &str,
    source: &str,
    lang: &str,
    context: &str,
) {
    let _ = try_translate(
        pool,
        &state.translation.in_flight,
        &state.translation.tx,
        site_key,
        source,
        lang,
        Some(context),
    )
    .await;
}
