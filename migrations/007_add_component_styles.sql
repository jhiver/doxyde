-- Add style options support to components
ALTER TABLE components ADD COLUMN style_options TEXT;

-- Update existing components to have empty style options
UPDATE components SET style_options = '{}' WHERE style_options IS NULL;