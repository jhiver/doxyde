-- Remove site_id from all remaining tables in multi-database architecture
-- Each database represents exactly one site, so site_id is redundant

-- The pages table was already updated in migration 016

-- Update any remaining references to site_id in other tables
-- Currently, no other tables have site_id references after migration 016

-- This migration serves as a marker that the multi-database architecture is complete