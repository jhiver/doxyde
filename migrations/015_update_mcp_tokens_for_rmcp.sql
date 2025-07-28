-- Drop the old mcp_tokens table and recreate with new schema for RMCP
DROP TABLE IF EXISTS mcp_tokens;

CREATE TABLE mcp_tokens (
    id INTEGER PRIMARY KEY,
    site_id INTEGER NOT NULL,
    token_hash TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    scopes TEXT,
    created_by INTEGER NOT NULL,
    expires_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_used_at TEXT,
    FOREIGN KEY (site_id) REFERENCES sites(id) ON DELETE CASCADE,
    FOREIGN KEY (created_by) REFERENCES users(id) ON DELETE CASCADE
);

-- Index for fast token lookup
CREATE INDEX idx_mcp_tokens_hash ON mcp_tokens(token_hash);

-- Index for cleanup of expired tokens
CREATE INDEX idx_mcp_tokens_expires ON mcp_tokens(expires_at);