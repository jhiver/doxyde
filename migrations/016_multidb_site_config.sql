-- Multi-database architecture refactor
-- Each database now represents exactly ONE site
-- The sites table becomes site_config with no domain column

-- First, create site_config table if it doesn't exist
CREATE TABLE IF NOT EXISTS site_config (
    id INTEGER PRIMARY KEY CHECK (id = 1), -- Ensures only one row
    title TEXT NOT NULL,
    description TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Use a simple approach: just insert a default row if site_config is empty
-- The migration tool will handle the actual data migration from sites table
INSERT INTO site_config (id, title, created_at, updated_at)
SELECT 1, 'New Site', datetime('now'), datetime('now')
WHERE NOT EXISTS (SELECT 1 FROM site_config);

-- Drop the sites table if it exists (suppress error if doesn't exist)
-- Note: In SQLite, IF EXISTS works but older versions might not have the table
-- We'll check first
-- This is a no-op if sites table doesn't exist
-- DROP TABLE IF EXISTS sites;

-- For pages table: The new schema doesn't have site_id
-- Migration 017 will handle removing site_id from pages and creating root page

-- site_users table stays as is (it already doesn't have site_id in the schema)
-- Just ensure it exists
CREATE TABLE IF NOT EXISTS site_users (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL,
    role TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(user_id)
);