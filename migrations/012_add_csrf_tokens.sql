-- Add CSRF token column to sessions table
ALTER TABLE sessions ADD COLUMN csrf_token TEXT;

-- Add last_activity column for session timeout tracking
ALTER TABLE sessions ADD COLUMN last_activity TEXT;

-- Index for faster CSRF token lookups (if needed)
CREATE INDEX idx_sessions_csrf_token ON sessions(csrf_token);