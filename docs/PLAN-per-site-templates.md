# Plan: Per-Site Template & CSS Customization with MCP Management

## Objective

Enable each site to have its own customizable templates, CSS, and static assets. Templates and styles should be manageable via MCP so AI tools can customize the look and feel of each site.

## Current State

- **Single shared template directory**: `templates/` used by all sites
- **Templates stored on filesystem**: Tera loads from `templates/**/*`
- **Per-site data**: Only `sites/<hash>/uploads/` exists per-site
- **No MCP tools** for template/CSS management

## Target Architecture

```
sites/<domain-hash>/
├── site.db              # SQLite database (existing)
├── uploads/             # Image uploads (existing)
└── assets/              # NEW: Per-site customizable assets
    ├── templates/       # Tera templates (copied from defaults)
    │   ├── base.html
    │   ├── page_templates/
    │   └── components/
    ├── css/             # Custom stylesheets
    │   └── site.css     # Main site CSS (editable via MCP)
    └── js/              # Custom JavaScript (optional)
```

## Database Schema

New tables in per-site SQLite database:

```sql
-- Track template customizations
CREATE TABLE site_templates (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,        -- e.g., 'base.html', 'components/text/default.html'
    content TEXT NOT NULL,            -- Template source code
    is_customized BOOLEAN DEFAULT 0,  -- True if modified from default
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Track CSS customizations
CREATE TABLE site_styles (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,        -- e.g., 'site.css', 'components.css'
    content TEXT NOT NULL,            -- CSS source code
    is_customized BOOLEAN DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Track static assets (JS, fonts, etc.)
CREATE TABLE site_assets (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL UNIQUE,        -- e.g., 'js/custom.js'
    content BLOB NOT NULL,            -- Binary content
    mime_type TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
```

## Template Loading Hierarchy

When rendering a template:

1. **Check database** (`site_templates` table) for site-specific version
2. **Fall back to filesystem** (`templates/` directory) for defaults
3. Cache templates with invalidation on update

```rust
// Pseudocode for template resolution
fn get_template(site_db: &SqlitePool, path: &str) -> Result<String> {
    // Try site-specific first
    if let Some(template) = site_templates_repo.find_by_path(path).await? {
        return Ok(template.content);
    }
    // Fall back to default filesystem template
    read_default_template(path)
}
```

## CSS Serving

New route: `/.site-assets/{path}` serves per-site CSS/JS from database:

```rust
// Route: GET /.site-assets/css/site.css
async fn serve_site_asset(
    db: SqlitePool,
    Path(path): Path<String>,
) -> Response {
    // Try database first
    if let Some(style) = site_styles_repo.find_by_name(&path).await? {
        return (
            [(header::CONTENT_TYPE, "text/css")],
            style.content
        ).into_response();
    }
    // Fall back to static file
    serve_static_file(&path).await
}
```

## MCP Tools

### Template Management

```
list_templates
  - Returns all template paths with customization status
  - Input: none
  - Output: [{ path, is_customized, updated_at }]

get_template
  - Get template content (from DB if customized, else default)
  - Input: { path: string }
  - Output: { path, content, is_customized, is_default }

update_template
  - Create or update a site-specific template
  - Input: { path: string, content: string }
  - Output: { success, path }

reset_template
  - Remove customization, revert to default
  - Input: { path: string }
  - Output: { success }

preview_template
  - Render template with sample data (for AI validation)
  - Input: { path: string, content: string, sample_data?: object }
  - Output: { html: string, errors?: string[] }
```

### CSS Management

```
get_site_css
  - Get current site CSS
  - Input: { name?: string }  (default: 'site.css')
  - Output: { name, content, is_customized }

update_site_css
  - Update site CSS
  - Input: { name?: string, content: string }
  - Output: { success }

reset_site_css
  - Revert to default CSS
  - Input: { name?: string }
  - Output: { success }

list_css_variables
  - List CSS custom properties for easy theming
  - Input: none
  - Output: [{ name, value, description }]

update_css_variables
  - Update specific CSS variables without replacing entire file
  - Input: { variables: { name: value } }
  - Output: { success }
```

### Asset Management

```
list_assets
  - List all custom assets (JS, fonts, etc.)
  - Input: none
  - Output: [{ path, mime_type, size, updated_at }]

upload_asset
  - Upload a new asset (base64 or URL)
  - Input: { path: string, content: base64 | url, mime_type: string }
  - Output: { success, path }

delete_asset
  - Remove a custom asset
  - Input: { path: string }
  - Output: { success }
```

## Implementation Phases

### Phase 1: Database Schema & Repositories (2-3 hours)
1. Create migration `020_site_templates_and_styles.sql`
2. Create models: `SiteTemplate`, `SiteStyle`, `SiteAsset`
3. Create repositories with CRUD operations
4. Add tests for repositories

### Phase 2: Template Loading Refactor (3-4 hours)
1. Create `SiteTemplateEngine` that wraps Tera + DB lookup
2. Modify `AppState` to use new engine
3. Update all handlers to use site-aware template loading
4. Add caching with invalidation
5. Add tests for template resolution hierarchy

### Phase 3: CSS Serving Route (1-2 hours)
1. Add `/.site-assets/{path}` route
2. Implement database lookup with filesystem fallback
3. Update `base.html` to reference `/.site-assets/css/site.css`
4. Add proper caching headers
5. Add tests

### Phase 4: MCP Template Tools (3-4 hours)
1. Add `list_templates`, `get_template`, `update_template`, `reset_template`
2. Add `preview_template` with error handling
3. Add tests for each tool
4. Update MCP documentation

### Phase 5: MCP CSS Tools (2-3 hours)
1. Add `get_site_css`, `update_site_css`, `reset_site_css`
2. Add `list_css_variables`, `update_css_variables`
3. Add tests
4. Update MCP documentation

### Phase 6: MCP Asset Tools (2 hours)
1. Add `list_assets`, `upload_asset`, `delete_asset`
2. Add tests
3. Update MCP documentation

### Phase 7: Site Initialization (1-2 hours)
1. On site creation, optionally copy default templates to DB
2. Add CLI command: `doxyde site init-templates <domain>`
3. Add tests

## CSS Variables for Theming

Define a standard set of CSS custom properties that AI can easily modify:

```css
:root {
    /* Colors */
    --color-primary: #2563eb;
    --color-primary-hover: #1d4ed8;
    --color-secondary: #64748b;
    --color-accent: #f59e0b;
    --color-background: #ffffff;
    --color-surface: #f8fafc;
    --color-text: #1e293b;
    --color-text-muted: #64748b;
    --color-border: #e2e8f0;
    --color-error: #dc2626;
    --color-success: #16a34a;

    /* Typography */
    --font-family-base: system-ui, -apple-system, sans-serif;
    --font-family-heading: var(--font-family-base);
    --font-family-mono: ui-monospace, monospace;
    --font-size-base: 1rem;
    --font-size-sm: 0.875rem;
    --font-size-lg: 1.125rem;
    --font-size-xl: 1.25rem;
    --line-height-base: 1.6;

    /* Spacing */
    --spacing-xs: 0.25rem;
    --spacing-sm: 0.5rem;
    --spacing-md: 1rem;
    --spacing-lg: 1.5rem;
    --spacing-xl: 2rem;

    /* Layout */
    --max-content-width: 1200px;
    --sidebar-width: 280px;
    --border-radius: 0.375rem;
    --shadow-sm: 0 1px 2px rgba(0,0,0,0.05);
    --shadow-md: 0 4px 6px rgba(0,0,0,0.1);
}
```

## Security Considerations

1. **Template Injection**: Validate Tera syntax before saving; reject templates with dangerous constructs
2. **CSS Injection**: Sanitize CSS to prevent `expression()`, `javascript:`, `@import` from external URLs
3. **Size Limits**:
   - Templates: max 500KB each
   - CSS: max 1MB total
   - Assets: max 10MB per file
4. **Rate Limiting**: Limit template/CSS updates to prevent abuse
5. **Backup**: Keep previous versions for rollback (future enhancement)

## Testing Strategy

1. **Unit tests**: Repository CRUD operations
2. **Integration tests**: Template resolution hierarchy
3. **MCP tests**: Each tool with valid/invalid inputs
4. **E2E tests**: Full workflow - customize template via MCP, verify rendering

## Migration Path

1. Existing sites continue working (DB tables empty = use filesystem defaults)
2. No breaking changes to current behavior
3. Opt-in customization via MCP tools
4. Future: Admin UI for template editing (post-MVP)

## Files to Create/Modify

### New Files
- `migrations/020_site_templates_and_styles.sql`
- `doxyde-core/src/models/site_template.rs`
- `doxyde-core/src/models/site_style.rs`
- `doxyde-core/src/models/site_asset.rs`
- `doxyde-db/src/repositories/site_template_repository.rs`
- `doxyde-db/src/repositories/site_style_repository.rs`
- `doxyde-db/src/repositories/site_asset_repository.rs`
- `doxyde-web/src/site_template_engine.rs`
- `doxyde-web/src/handlers/site_assets.rs`

### Modified Files
- `doxyde-core/src/models/mod.rs` - Export new models
- `doxyde-db/src/repositories/mod.rs` - Export new repos
- `doxyde-web/src/lib.rs` - Add site assets route
- `doxyde-web/src/routes.rs` - Add `/.site-assets/{path}`
- `doxyde-mcp/src/mcp/service.rs` - Add new MCP tools
- `templates/base.html` - Reference `/.site-assets/css/site.css`

## Success Criteria

1. Each site can have custom templates stored in its database
2. AI tools can list, read, update, and reset templates via MCP
3. AI tools can customize CSS variables for theming
4. Default templates work seamlessly when no customization exists
5. Template changes are immediately reflected (no restart needed)
6. 100% backward compatibility with existing sites
