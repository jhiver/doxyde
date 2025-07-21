-- Add draft/publish support to page versions
ALTER TABLE page_versions ADD COLUMN is_published BOOLEAN NOT NULL DEFAULT 0;

-- Add title and template support to components
ALTER TABLE components ADD COLUMN title TEXT;
ALTER TABLE components ADD COLUMN template TEXT NOT NULL DEFAULT 'default';

-- Update existing versions to be published
UPDATE page_versions SET is_published = 1 WHERE id IN (
    SELECT MAX(id) FROM page_versions GROUP BY page_id
);

-- Create index for finding published versions
CREATE INDEX idx_page_versions_published ON page_versions(page_id, is_published, version_number DESC);