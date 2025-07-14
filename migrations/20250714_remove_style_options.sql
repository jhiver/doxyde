-- Remove style_options column from components table
-- This migration removes the broken style_options feature

-- SQLite doesn't support DROP COLUMN directly, so we need to:
-- 1. Create a new table without the style_options column
-- 2. Copy data from the old table
-- 3. Drop the old table
-- 4. Rename the new table

CREATE TABLE components_new (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    page_version_id INTEGER NOT NULL,
    component_type TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,
    content TEXT NOT NULL,
    title TEXT,
    template TEXT NOT NULL DEFAULT 'default',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (page_version_id) REFERENCES page_versions (id) ON DELETE CASCADE
);

-- Copy data from old table (excluding style_options)
INSERT INTO components_new (id, page_version_id, component_type, position, content, title, template, created_at, updated_at)
SELECT id, page_version_id, component_type, position, content, title, template, created_at, updated_at
FROM components;

-- Drop the old table
DROP TABLE components;

-- Rename the new table
ALTER TABLE components_new RENAME TO components;

-- Recreate indexes
CREATE INDEX idx_components_page_version ON components(page_version_id);
CREATE INDEX idx_components_position ON components(position);