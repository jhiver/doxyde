#!/bin/bash

# Test adding a component to the page

TOKEN=$(sqlite3 doxyde.db "SELECT id FROM mcp_tokens WHERE revoked_at IS NULL LIMIT 1")

echo "Using MCP token: $TOKEN"

# For now, we'll add a component directly via SQL since the MCP tool isn't implemented yet
echo "Adding a text component to the test page..."

# First create a page version
sqlite3 doxyde.db << EOF
-- Get the page ID
SELECT id FROM pages WHERE slug = 'test-fidgets';

-- Create a published version for the page
INSERT INTO page_versions (page_id, version_number, created_by, is_published, created_at, updated_at)
VALUES (8, 1, NULL, 1, datetime('now'), datetime('now'));

-- Get the version ID
SELECT last_insert_rowid();

-- Add a text component
INSERT INTO components (page_version_id, component_type, position, template, title, content, created_at, updated_at)
VALUES (
    last_insert_rowid(),
    'text',
    0,
    'default',
    'About Fidgets',
    '{"text": "Fidgets are small toys or tools designed to help people focus, relieve stress, or simply keep their hands busy. The fidget spinner craze of 2017 brought these devices into mainstream consciousness, but fidget toys have been used for years to help people with ADHD, anxiety, or those who simply benefit from tactile stimulation."}',
    datetime('now'),
    datetime('now')
);

SELECT 'Component added successfully';
EOF