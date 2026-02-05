-- Fix: Remove site_id and FK to non-existent sites table from pages and site_users
-- Migration 017 was recorded as applied but did not fully execute because
-- the migration runner only executed the first SQL statement.
-- This migration re-applies the table recreation idempotently.

-- Disable foreign key checks for this migration (required to drop/recreate tables with FKs)
PRAGMA foreign_keys = OFF;

-- First, drop pages_new if it exists from a partial 017 run
DROP TABLE IF EXISTS pages_new;

-- Check if pages still has site_id by trying to create the new table
-- If site_id doesn't exist, the SELECT will fail and we skip
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
    og_image_url,
    COALESCE(structured_data_type, 'WebPage'),
    COALESCE(position, 0),
    COALESCE(sort_mode, 'created_at_asc'),
    created_at,
    updated_at
FROM pages;

DROP TABLE pages;

ALTER TABLE pages_new RENAME TO pages;

-- Recreate indexes on pages
CREATE INDEX IF NOT EXISTS idx_pages_parent_page_id ON pages(parent_page_id);
CREATE INDEX IF NOT EXISTS idx_pages_template ON pages(template);
CREATE INDEX IF NOT EXISTS idx_pages_sort_mode ON pages(sort_mode);

-- Also fix site_users: remove FK to sites table
DROP TABLE IF EXISTS site_users_new;

CREATE TABLE site_users_new (
    site_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    role TEXT NOT NULL CHECK (role IN ('viewer', 'editor', 'owner')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (site_id, user_id),
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

INSERT INTO site_users_new SELECT * FROM site_users;

DROP TABLE site_users;

ALTER TABLE site_users_new RENAME TO site_users;

CREATE INDEX IF NOT EXISTS idx_site_users_user_id ON site_users(user_id);

-- Re-enable foreign key checks
PRAGMA foreign_keys = ON;
