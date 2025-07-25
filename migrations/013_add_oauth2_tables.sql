-- Add OAuth2 support for MCP integration with Claude Desktop

-- OAuth2 clients table for dynamic client registration
CREATE TABLE IF NOT EXISTS oauth_clients (
    client_id TEXT PRIMARY KEY,
    client_secret_hash TEXT, -- NULL for public clients
    client_name TEXT NOT NULL,
    redirect_uris TEXT NOT NULL, -- JSON array of allowed redirect URIs
    grant_types TEXT NOT NULL DEFAULT '["authorization_code"]', -- JSON array
    response_types TEXT NOT NULL DEFAULT '["code"]', -- JSON array
    scope TEXT DEFAULT 'mcp:read mcp:write',
    token_endpoint_auth_method TEXT DEFAULT 'client_secret_basic',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    created_by_token_id TEXT,
    FOREIGN KEY (created_by_token_id) REFERENCES mcp_tokens(id) ON DELETE CASCADE
);

-- Authorization codes (short-lived, for OAuth2 flow)
CREATE TABLE IF NOT EXISTS oauth_authorization_codes (
    code TEXT PRIMARY KEY,
    client_id TEXT NOT NULL,
    user_id INTEGER NOT NULL,
    mcp_token_id TEXT NOT NULL,
    redirect_uri TEXT NOT NULL,
    scope TEXT,
    code_challenge TEXT, -- For PKCE
    code_challenge_method TEXT, -- S256
    expires_at TEXT NOT NULL,
    used_at TEXT, -- Track if code was already used
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (client_id) REFERENCES oauth_clients(client_id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (mcp_token_id) REFERENCES mcp_tokens(id) ON DELETE CASCADE
);

-- Access tokens
CREATE TABLE IF NOT EXISTS oauth_access_tokens (
    token_hash TEXT PRIMARY KEY, -- Store SHA256 hash of token
    client_id TEXT NOT NULL,
    user_id INTEGER NOT NULL,
    mcp_token_id TEXT NOT NULL,
    scope TEXT,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (client_id) REFERENCES oauth_clients(client_id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (mcp_token_id) REFERENCES mcp_tokens(id) ON DELETE CASCADE
);

-- Refresh tokens
CREATE TABLE IF NOT EXISTS oauth_refresh_tokens (
    token_hash TEXT PRIMARY KEY, -- Store SHA256 hash of token
    client_id TEXT NOT NULL,
    user_id INTEGER NOT NULL,
    mcp_token_id TEXT NOT NULL,
    scope TEXT,
    expires_at TEXT NOT NULL,
    used_at TEXT, -- Track when refresh token was used
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (client_id) REFERENCES oauth_clients(client_id) ON DELETE CASCADE,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (mcp_token_id) REFERENCES mcp_tokens(id) ON DELETE CASCADE
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_oauth_clients_created_by ON oauth_clients(created_by_token_id);
CREATE INDEX IF NOT EXISTS idx_oauth_auth_codes_client ON oauth_authorization_codes(client_id);
CREATE INDEX IF NOT EXISTS idx_oauth_auth_codes_expires ON oauth_authorization_codes(expires_at);
CREATE INDEX IF NOT EXISTS idx_oauth_access_tokens_client ON oauth_access_tokens(client_id);
CREATE INDEX IF NOT EXISTS idx_oauth_access_tokens_mcp ON oauth_access_tokens(mcp_token_id);
CREATE INDEX IF NOT EXISTS idx_oauth_access_tokens_expires ON oauth_access_tokens(expires_at);
CREATE INDEX IF NOT EXISTS idx_oauth_refresh_tokens_client ON oauth_refresh_tokens(client_id);
CREATE INDEX IF NOT EXISTS idx_oauth_refresh_tokens_mcp ON oauth_refresh_tokens(mcp_token_id);