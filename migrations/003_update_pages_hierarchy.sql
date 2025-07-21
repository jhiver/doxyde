-- SQLite doesn't support dropping constraints, so we need to recreate the table
-- Create new pages table with hierarchical structure
CREATE TABLE pages_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    site_id INTEGER NOT NULL,
    parent_page_id INTEGER,
    slug TEXT NOT NULL,
    title TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (site_id) REFERENCES sites(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_page_id) REFERENCES pages(id) ON DELETE CASCADE,
    UNIQUE(site_id, parent_page_id, slug)
);

-- Copy data from old table
INSERT INTO pages_new (id, site_id, slug, title, created_at, updated_at)
SELECT id, site_id, slug, title, created_at, updated_at FROM pages;

-- Drop old table
DROP TABLE pages;

-- Rename new table
ALTER TABLE pages_new RENAME TO pages;

-- Recreate indexes
CREATE INDEX idx_pages_site_id ON pages(site_id);
CREATE INDEX idx_pages_parent_page_id ON pages(parent_page_id);