-- Add sort_mode field to pages table
ALTER TABLE pages ADD COLUMN sort_mode TEXT NOT NULL DEFAULT 'created_at_asc';

-- Create index for better query performance
CREATE INDEX idx_pages_sort_mode ON pages(sort_mode);