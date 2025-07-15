-- Create table for MCP access tokens
CREATE TABLE IF NOT EXISTS mcp_tokens (
    id TEXT PRIMARY KEY,  -- UUID
    user_id INTEGER NOT NULL,
    site_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_used_at TEXT,
    revoked_at TEXT,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (site_id) REFERENCES sites(id) ON DELETE CASCADE
);

-- Index for faster lookups
CREATE INDEX IF NOT EXISTS idx_mcp_tokens_user_id ON mcp_tokens(user_id);
CREATE INDEX IF NOT EXISTS idx_mcp_tokens_site_id ON mcp_tokens(site_id);
CREATE INDEX IF NOT EXISTS idx_mcp_tokens_revoked ON mcp_tokens(revoked_at);