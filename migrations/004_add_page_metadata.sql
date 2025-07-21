-- Add metadata fields to pages table
ALTER TABLE pages ADD COLUMN description TEXT;
ALTER TABLE pages ADD COLUMN keywords TEXT;
ALTER TABLE pages ADD COLUMN template TEXT DEFAULT 'default';
ALTER TABLE pages ADD COLUMN meta_json TEXT DEFAULT '{}';

-- Create an index on template for performance
CREATE INDEX idx_pages_template ON pages(template);