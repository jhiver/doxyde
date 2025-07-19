-- Update all root pages (pages with no parent) to have empty slug
UPDATE pages 
SET slug = '' 
WHERE parent_page_id IS NULL;