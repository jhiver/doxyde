-- i18n (lot 0) per-site translation cache, UI-label cache and config.
-- NOTE the migration runner (doxyde-db/src/init.rs) naively splits statements
-- on the semicolon character and skips comment-only fragments, so every
-- statement must be standalone (no embedded semicolons, including inside
-- comments) and idempotent (re-runnable across many site DBs).

-- Editorial + raw-label translation cache, keyed by (lang, content_hash).
CREATE TABLE IF NOT EXISTS translation_cache (
    lang TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    translated_content TEXT NOT NULL,
    is_manual_override INTEGER NOT NULL DEFAULT 0,
    is_failed INTEGER NOT NULL DEFAULT 0,
    cached_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (lang, content_hash)
);

-- UI-label catalog cache: maps a dotted key + English source snapshot to its
-- per-language translation. The source snapshot lets us re-translate when the
-- canonical English label changes (yatoo i18n-load pattern).
CREATE TABLE IF NOT EXISTS ui_label_cache (
    lang TEXT NOT NULL,
    label_key TEXT NOT NULL,
    source_text TEXT NOT NULL,
    translated_text TEXT NOT NULL,
    cached_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (lang, label_key)
);

-- i18n configuration singleton (one row per site DB).
CREATE TABLE IF NOT EXISTS i18n_config (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    default_lang TEXT NOT NULL DEFAULT 'en',
    source_lang TEXT NOT NULL DEFAULT 'en'
);

INSERT INTO i18n_config (id, default_lang, source_lang)
SELECT 1, 'en', 'en'
WHERE NOT EXISTS (SELECT 1 FROM i18n_config WHERE id = 1);

-- Enabled target languages (one row per enabled language). Default {en, fr}.
CREATE TABLE IF NOT EXISTS i18n_enabled_lang (
    lang TEXT PRIMARY KEY,
    position INTEGER NOT NULL DEFAULT 0
);

INSERT INTO i18n_enabled_lang (lang, position)
SELECT 'en', 0
WHERE NOT EXISTS (SELECT 1 FROM i18n_enabled_lang WHERE lang = 'en');

INSERT INTO i18n_enabled_lang (lang, position)
SELECT 'fr', 1
WHERE NOT EXISTS (SELECT 1 FROM i18n_enabled_lang WHERE lang = 'fr');
