-- Remove site_id from all remaining tables in multi-database architecture
-- Each database represents exactly one site, so site_id is redundant

-- Remove site_id from pages table
CREATE TABLE pages_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    parent_page_id INTEGER,
    slug TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    keywords TEXT,
    template TEXT NOT NULL DEFAULT 'default',
    meta_robots TEXT NOT NULL DEFAULT 'index,follow',
    canonical_url TEXT,
    og_image_url TEXT,
    structured_data_type TEXT NOT NULL DEFAULT 'WebPage',
    position INTEGER NOT NULL DEFAULT 0,
    sort_mode TEXT NOT NULL DEFAULT 'created_at_asc',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (parent_page_id) REFERENCES pages(id) ON DELETE CASCADE,
    UNIQUE(parent_page_id, slug)
);

-- Copy data from old table (excluding site_id)
-- Use COALESCE to provide default values for new columns that might not exist in old table
INSERT INTO pages_new (id, parent_page_id, slug, title, description, keywords, template, meta_robots, canonical_url, og_image_url, structured_data_type, position, sort_mode, created_at, updated_at)
SELECT
    id,
    parent_page_id,
    slug,
    title,
    COALESCE(description, ''),
    COALESCE(keywords, ''),
    COALESCE(template, 'default'),
    COALESCE(meta_robots, 'index,follow'),
    canonical_url,
    COALESCE(og_image_url, NULL),
    COALESCE(structured_data_type, 'WebPage'),
    COALESCE(position, 0),
    COALESCE(sort_mode, 'created_at_asc'),
    created_at,
    updated_at
FROM pages;

-- Drop old table and rename new one
DROP TABLE pages;
ALTER TABLE pages_new RENAME TO pages;

-- Create root page if it doesn't exist
-- In single-database architecture, every site needs exactly one root page
INSERT INTO pages (id, parent_page_id, slug, title, description, keywords, template, meta_robots, canonical_url, og_image_url, structured_data_type, position, sort_mode, created_at, updated_at)
SELECT 1, NULL, '', 'Home', '', '', 'default', 'index,follow', '/', NULL, 'WebPage', 0, 'created_at_asc', datetime('now'), datetime('now')
WHERE NOT EXISTS (SELECT 1 FROM pages WHERE id = 1);

-- This migration completes the multi-database architecture