-- Migration 101: Toolkits and Instances System
-- Implements hierarchical parameter binding: server defaults → toolkit bindings → instance bindings
-- Enables progressive parameter binding and tool cloning for specialized instances

-- ============================================================================
-- SUBDOMAIN DEFAULTS (Global Auto-Bind Rules)
-- ============================================================================
-- Global parameter bindings that automatically apply to ALL new tool instances
-- Example: Set api_key once, all new tools get it automatically
-- These are "baked in" at instance creation time (not dynamic)

CREATE TABLE IF NOT EXISTS subdomain_defaults (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    subdomain_id INTEGER NOT NULL,
    parameter_name TEXT NOT NULL,           -- "api_key", "timeout", "format"
    parameter_value TEXT NOT NULL,          -- JSON value: "\"sk-xxx\"", "30", "\"json\""
    description TEXT,                       -- User note: "Production API key"

    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (subdomain_id) REFERENCES subdomains(id) ON DELETE CASCADE,
    UNIQUE(subdomain_id, parameter_name)
);

CREATE INDEX IF NOT EXISTS idx_subdomain_defaults_subdomain ON subdomain_defaults(subdomain_id);

-- ============================================================================
-- TOOLKITS (Collections of Tool Templates)
-- ============================================================================
-- A toolkit is a curated collection of related tool templates
-- Example: "GitHub API Toolkit" contains 10 GitHub-related tools
-- Can be shared in marketplace or kept private

CREATE TABLE IF NOT EXISTS toolkits (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,                     -- "GitHub API Toolkit"
    description TEXT,                       -- "Complete GitHub API integration"
    creator_user_id INTEGER NOT NULL,       -- Who created this toolkit
    is_public BOOLEAN DEFAULT false,        -- Available in marketplace?
    category TEXT,                          -- "API", "Data Processing", "Utility"

    -- Marketplace stats
    download_count INTEGER DEFAULT 0,       -- How many times installed
    rating_avg REAL,                        -- Average user rating (1-5 stars)

    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (creator_user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_toolkits_creator ON toolkits(creator_user_id);
CREATE INDEX IF NOT EXISTS idx_toolkits_public ON toolkits(is_public);

-- ============================================================================
-- TOOLKIT TEMPLATES (Junction: Toolkits ↔ Tool Templates)
-- ============================================================================
-- Defines which tool templates belong to which toolkits
-- A template can be in multiple toolkits
-- A toolkit can contain multiple templates

CREATE TABLE IF NOT EXISTS toolkit_templates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    toolkit_id INTEGER NOT NULL,
    template_id INTEGER NOT NULL,
    display_order INTEGER DEFAULT 0,        -- For UI ordering
    notes TEXT,                             -- Creator notes about this tool in context

    created_at TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (toolkit_id) REFERENCES toolkits(id) ON DELETE CASCADE,
    FOREIGN KEY (template_id) REFERENCES tool_templates(id) ON DELETE CASCADE,
    UNIQUE(toolkit_id, template_id)
);

CREATE INDEX IF NOT EXISTS idx_toolkit_templates_toolkit ON toolkit_templates(toolkit_id);
CREATE INDEX IF NOT EXISTS idx_toolkit_templates_template ON toolkit_templates(template_id);

-- ============================================================================
-- SUBDOMAIN TOOLKITS (Toolkit Installation Records)
-- ============================================================================
-- Tracks which toolkits are installed on which subdomains
-- Also stores toolkit-level parameter bindings that apply to all tools in the toolkit
-- Example: Install GitHub toolkit with binding {github_api_version: "2022-11-28"}

CREATE TABLE IF NOT EXISTS subdomain_toolkits (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    subdomain_id INTEGER NOT NULL,
    toolkit_id INTEGER NOT NULL,

    -- Toolkit-level parameter bindings (JSON)
    -- Applied to all tool instances created from this toolkit installation
    -- Example: {"format": "json", "api_version": "v2"}
    parameter_bindings TEXT,

    enabled BOOLEAN DEFAULT true,           -- Can disable entire toolkit
    installed_at TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (subdomain_id) REFERENCES subdomains(id) ON DELETE CASCADE,
    FOREIGN KEY (toolkit_id) REFERENCES toolkits(id) ON DELETE CASCADE,
    UNIQUE(subdomain_id, toolkit_id)
);

CREATE INDEX IF NOT EXISTS idx_subdomain_toolkits_subdomain ON subdomain_toolkits(subdomain_id);
CREATE INDEX IF NOT EXISTS idx_subdomain_toolkits_toolkit ON subdomain_toolkits(toolkit_id);

-- ============================================================================
-- TOOL INSTANCES (Final Instantiated Tools with Merged Bindings)
-- ============================================================================
-- The actual tools that are exposed via MCP protocol
-- Created from templates with parameter bindings merged from:
--   1. Server defaults (subdomain_defaults)
--   2. Toolkit bindings (subdomain_toolkits.parameter_bindings)
--   3. Instance-specific bindings (this table's parameter_bindings)
--
-- Example progression:
--   Template: get_color(color, format, api_key)
--   + Server default: api_key="sk-xxx"
--   + Toolkit binding: format="json"
--   = Instance: get_color(color) with bindings {api_key: "sk-xxx", format: "json"}
--
--   Then clone it with: color="blue"
--   = New instance: get_blue() with bindings {color: "blue", api_key: "sk-xxx", format: "json"}
--   → MCP signature: get_blue() - NO PARAMETERS!

CREATE TABLE IF NOT EXISTS tool_instances (
    id INTEGER PRIMARY KEY AUTOINCREMENT,

    subdomain_id INTEGER NOT NULL,          -- Which subdomain hosts this instance
    template_id INTEGER NOT NULL,           -- Which template is this based on

    -- Customization
    instance_name TEXT,                     -- NULL = use template.name, else custom name
    parameter_bindings TEXT,                -- JSON: Instance-specific parameter bindings

    -- Provenance (optional tracking)
    source_toolkit_id INTEGER,              -- Which toolkit was this installed from?
    cloned_from_instance_id INTEGER,        -- Was this cloned from another instance?

    -- Configuration
    custom_config TEXT,                     -- JSON: Custom headers, timeout overrides, etc.
    enabled BOOLEAN DEFAULT true,           -- Can disable without deleting

    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    FOREIGN KEY (subdomain_id) REFERENCES subdomains(id) ON DELETE CASCADE,
    FOREIGN KEY (template_id) REFERENCES tool_templates(id) ON DELETE CASCADE,
    FOREIGN KEY (source_toolkit_id) REFERENCES toolkits(id) ON DELETE SET NULL,
    FOREIGN KEY (cloned_from_instance_id) REFERENCES tool_instances(id) ON DELETE SET NULL,

    -- Instance name must be unique per subdomain (NULL is allowed multiple times)
    UNIQUE(subdomain_id, instance_name)
);

CREATE INDEX IF NOT EXISTS idx_tool_instances_subdomain ON tool_instances(subdomain_id);
CREATE INDEX IF NOT EXISTS idx_tool_instances_template ON tool_instances(template_id);
CREATE INDEX IF NOT EXISTS idx_tool_instances_toolkit ON tool_instances(source_toolkit_id);

-- ============================================================================
-- DEPRECATION NOTICE: subdomain_tools
-- ============================================================================
-- The old subdomain_tools junction table is now replaced by tool_instances
-- tool_instances provides the same many-to-many relationship but with richer
-- parameter binding capabilities and instance-level customization
--
-- Migration path (future):
--   1. For each row in subdomain_tools:
--      - Create tool_instance with same subdomain_id and template_id
--      - Copy custom_config to tool_instances.custom_config
--      - Set instance_name = NULL (use template name)
--      - Set parameter_bindings = NULL (no bindings)
--   2. Drop subdomain_tools table
--
-- For now, both tables coexist to avoid breaking existing code
-- TODO: Remove subdomain_tools in migration 102 after full migration
