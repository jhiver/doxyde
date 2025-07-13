-- Sites table
CREATE TABLE IF NOT EXISTS sites (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    domain TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Pages table
CREATE TABLE IF NOT EXISTS pages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    site_id INTEGER NOT NULL,
    slug TEXT NOT NULL,
    title TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (site_id) REFERENCES sites(id) ON DELETE CASCADE,
    UNIQUE(site_id, slug)
);

-- Page versions table
CREATE TABLE IF NOT EXISTS page_versions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    page_id INTEGER NOT NULL,
    version_number INTEGER NOT NULL,
    created_by TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (page_id) REFERENCES pages(id) ON DELETE CASCADE,
    UNIQUE(page_id, version_number)
);

-- Components table
CREATE TABLE IF NOT EXISTS components (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    page_version_id INTEGER NOT NULL,
    component_type TEXT NOT NULL,
    position INTEGER NOT NULL,
    content TEXT NOT NULL, -- JSON content
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (page_version_id) REFERENCES page_versions(id) ON DELETE CASCADE
);

-- Indexes
CREATE INDEX idx_pages_site_id ON pages(site_id);
CREATE INDEX idx_page_versions_page_id ON page_versions(page_id);
CREATE INDEX idx_components_page_version_id ON components(page_version_id);