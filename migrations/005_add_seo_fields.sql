-- Add new SEO fields to pages table
ALTER TABLE pages ADD COLUMN meta_robots TEXT NOT NULL DEFAULT 'index,follow';
ALTER TABLE pages ADD COLUMN canonical_url TEXT;
ALTER TABLE pages ADD COLUMN og_image_url TEXT;
ALTER TABLE pages ADD COLUMN structured_data_type TEXT NOT NULL DEFAULT 'WebPage';

-- Drop the meta_json column
ALTER TABLE pages DROP COLUMN meta_json;