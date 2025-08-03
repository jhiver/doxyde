-- Remove site_id from mcp_tokens table in multi-database architecture
-- Each database represents exactly one site, so site_id is redundant

-- Create new table without site_id
CREATE TABLE mcp_tokens_new (
    id INTEGER PRIMARY KEY,
    token_hash TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    scopes TEXT,
    created_by INTEGER NOT NULL,
    expires_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_used_at TEXT,
    FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE CASCADE
);

-- Copy data from old table (excluding site_id)
INSERT INTO mcp_tokens_new (id, token_hash, name, scopes, created_by, expires_at, created_at, last_used_at)
SELECT id, token_hash, name, scopes, created_by, expires_at, created_at, last_used_at
FROM mcp_tokens;

-- Drop old table
DROP TABLE mcp_tokens;

-- Rename new table
ALTER TABLE mcp_tokens_new RENAME TO mcp_tokens;

-- Recreate indexes
CREATE INDEX idx_mcp_tokens_hash ON mcp_tokens(token_hash);
CREATE INDEX idx_mcp_tokens_expires ON mcp_tokens(expires_at);