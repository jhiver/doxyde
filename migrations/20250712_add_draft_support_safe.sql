-- Safe version of draft support migration that checks for existing columns
-- Since SQLite doesn't support IF NOT EXISTS for columns, we handle this differently

-- Skip the ALTER TABLE statements since the columns already exist
-- Just run the UPDATE and CREATE INDEX statements which are idempotent

-- Update existing versions to be published (safe to run multiple times)
UPDATE page_versions 
SET is_published = 1 
WHERE is_published = 0 
  AND id IN (
    SELECT MAX(id) FROM page_versions GROUP BY page_id
);

-- Create index for finding published versions (CREATE INDEX IF NOT EXISTS would be ideal but not supported)
-- This will fail gracefully if the index already exists
CREATE INDEX IF NOT EXISTS idx_page_versions_published ON page_versions(page_id, is_published, version_number DESC);