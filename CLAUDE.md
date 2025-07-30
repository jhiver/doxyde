# Doxyde Development Guide

## 🚨 CRITICAL: READ THIS FIRST 🚨

### Language Policy
**ALL code, comments, documentation, commit messages, and variable names MUST be in English.**

### Development Process Rules

**MANDATORY: One Function at a Time**
1. Write ONE function
2. Write tests for that function
3. Run tests and ensure they pass
4. ONLY then move to the next function

**FORBIDDEN:**
- ❌ Writing multiple functions before testing
- ❌ Using `unwrap()` or `expect()` anywhere
- ❌ Functions longer than 30 lines
- ❌ Nesting more than 3 levels deep
- ❌ More than 4 parameters per function
- ❌ Skipping tests "for now"

## 📋 Table of Contents

1. [Project Overview](#project-overview)
2. [Critical Development Guidelines](#critical-development-guidelines)
3. [Database and SQLx](#database-and-sqlx)
4. [Architecture](#architecture)
5. [Testing Strategy](#testing-strategy)
6. [Common Tasks](#common-tasks)
7. [Recent Updates](#recent-updates)
8. [Future Work](#future-work)

## Project Overview

Doxyde is a modern, AI-native content management system built with Rust.

### Current Status
- **MVP Complete** ✅
- **420+ tests passing**
- **Production-ready features**: Authentication, hierarchical pages, draft/publish workflow, image uploads, MCP integration

### Tech Stack
- **Backend**: Rust, Axum web framework
- **Database**: SQLite with SQLx
- **Templates**: Tera
- **AI Integration**: MCP (Model Context Protocol)

## Critical Development Guidelines

### 🔴 MANDATORY: SQLx Offline Mode Updates

When adding or modifying SQL queries:

```bash
# 1. Temporarily disable offline mode
export SQLX_OFFLINE=false

# 2. Ensure database exists and is up to date
export DATABASE_URL="sqlite:doxyde.db"
./target/debug/doxyde init  # or create fresh: sqlx database create && sqlx migrate run --source migrations

# 3. Regenerate the offline query cache
cargo sqlx prepare --workspace

# 4. Verify the .sqlx files were updated
git status .sqlx/

# 5. Commit the updated .sqlx files
git add .sqlx/
git commit -m "Update sqlx offline query cache"

# 6. Re-enable offline mode (automatic via .cargo/config.toml)
unset SQLX_OFFLINE
```

**IMPORTANT**: The project uses offline mode by default. Failing to update .sqlx files will break builds for other developers!

### Function Development Rules

```rust
// ✅ GOOD: Short, focused function
pub fn validate_domain(domain: &str) -> Result<()> {
    if domain.is_empty() {
        return Err(anyhow!("Domain cannot be empty"));
    }
    if domain.len() > 255 {
        return Err(anyhow!("Domain too long"));
    }
    if !domain.contains('.') && !domain.contains(':') {
        return Err(anyhow!("Domain must contain a dot or colon"));
    }
    Ok(())
}

// ❌ BAD: Too long, does too many things
pub fn process_request(request: Request) -> Result<Response> {
    // 50+ lines of validation, processing, formatting...
}
```

### Error Handling

```rust
// ✅ REQUIRED: Explicit error handling
let value = some_option
    .ok_or_else(|| anyhow!("Value was None"))?;

// ✅ BETTER: With context
let result = some_operation()
    .with_context(|| format!("Failed to process item {}", id))?;

// ❌ FORBIDDEN: These will panic
let value = some_option.unwrap();
let result = some_result.expect("this should work");
```

### Testing Requirements

Every public function must have tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_domain() {
        // Happy path
        assert!(validate_domain("example.com").is_ok());

        // Edge cases
        assert!(validate_domain("").is_err());
        assert!(validate_domain("no-dot").is_err());

        // Error cases
        let long_domain = "a".repeat(256);
        assert!(validate_domain(&long_domain).is_err());
    }
}
```

## Database and SQLx

### Migration System

Migrations use sequential numbering (001, 002, etc.) in `/migrations/`:
- `001_initial_schema.sql` - Base tables
- `002_add_auth_tables.sql` - Authentication
- `003_update_pages_hierarchy.sql` - Page hierarchy
- ... etc

**Adding new migrations:**
```bash
# Create next migration (e.g., 012_your_feature.sql)
echo "-- Your SQL here" > migrations/012_your_feature.sql

# Apply and test
export DATABASE_URL="sqlite:doxyde.db"
sqlx migrate run --source migrations
```

### SQLite Type Mappings

```rust
// DateTime handling
sqlx::query_as!(
    Site,
    r#"
    SELECT
        id as "id: i64",
        created_at as "created_at: chrono::DateTime<chrono::Utc>",
        updated_at as "updated_at: chrono::DateTime<chrono::Utc>"
    FROM sites
    "#
)

// Nullable columns
let row = sqlx::query!(
    r#"SELECT MAX(version) as "max_version: Option<i32>" FROM versions"#
)
```

### Repository Pattern

```rust
impl SiteRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, site: &Site) -> Result<i64> {
        let result = sqlx::query!(
            "INSERT INTO sites (domain, title) VALUES (?, ?)",
            site.domain,
            site.title
        )
        .execute(&self.pool)
        .await
        .context("Failed to create site")?;

        Ok(result.last_insert_rowid())
    }
}
```

### Database Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    #[sqlx::test]
    async fn test_create_site(pool: SqlitePool) -> Result<()> {
        let repo = SiteRepository::new(pool);
        let site = Site::new("example.com", "Example");

        let id = repo.create(&site).await?;
        assert!(id > 0);

        Ok(())
    }
}
```

## Architecture

### URL Routing Strategy

System actions use dot-prefix to avoid conflicts with content:
- **Content**: `/`, `/about`, `/products/widget`
- **Actions**: `/.login`, `/.admin`, `/about/.edit`, `/about/.new`

### Component System

Pages contain versioned components:
- **Types**: text, image, code, html
- **Templates**: default, card, highlight, quote
- **Draft/Publish**: All edits go through draft workflow

### MCP Integration

**Important MCP Rules:**
1. Always return JSON-RPC format
2. Never throw HTTP errors
3. Use error codes: `-32602` (invalid params), `-32603` (internal error)
4. Draft-first editing required

**Available MCP Tools:**
- `list_pages` - List all pages in a site
- `get_page` - Get page details and components
- `create_page` - Create a new page
- `update_page` - Update page properties
- `publish_page` - Publish page changes
- `create_text_component` - Add text/markdown content
- `update_text_component` - Modify text content
- `create_image_component` - Add images (URL or base64)
- `update_image_component` - Update image properties
- `create_code_component` - Add syntax-highlighted code
- `update_code_component` - Modify code content
- `create_html_component` - Add raw HTML content
- `update_html_component` - Modify HTML content
- `delete_component` - Remove any component
- `reorder_components` - Change component order

## Testing Strategy

### Running Tests

```bash
# Run all tests
cargo test --all

# Run specific test
cargo test test_validate_domain

# Run with logging
RUST_LOG=debug cargo test
```

### Test Organization

- Unit tests: In same file as code (`#[cfg(test)] mod tests`)
- Integration tests: In `tests/` directory
- Database tests: Use `#[sqlx::test]` for automatic SQLite setup

## Common Tasks

### Before ANY Commit

```bash
# 1. Format code
cargo fmt --all

# 2. Check for issues
cargo clippy --all-targets --all-features

# 3. Ensure compilation
cargo check --all

# 4. Run tests
cargo test --all

# 5. Update SQLx cache if needed (see above)
```

### Creating a Pull Request

```bash
# 1. Check git status
git status

# 2. Create meaningful commit
git add -A
git commit -m "feat: Add user avatar support

- Add avatar_url field to users table
- Update user edit form with avatar upload
- Add image validation for avatars
- Update tests for new functionality

🤖 Generated with [Claude Code](https://claude.ai/code)

Co-Authored-By: Claude <noreply@anthropic.com>"

# 3. Push to feature branch
git push origin feature/user-avatars
```

## Recent Updates

### July 2025

**Migration System Refactor:**
- Switched from timestamp to sequential numbering (001-011)
- Added robust error handling for partial migrations
- Enabled SQLx offline mode by default

**MCP Improvements:**
- Draft-first workflow enforcement
- Better error messages for AI agents
- Comprehensive test coverage
- Added create/update tools for all component types (text, image, code, html)
- Image upload support via URL download and Base64 data URIs
- SVG support added to image uploads

**Component System Enhancements:**
- Added `code` component type with syntax highlighting
- Added `html` component type for raw HTML content
- Enhanced image component templates and layouts
- Improved image upload and preview functionality

**Code Quality:**
- Refactored large functions into small, focused ones
- Consistent error handling patterns
- Improved test coverage
- Renamed `doxyde-shared` crate to `doxyde-mcp`

## 0.1 Release Plan (2-4 weeks remaining)

### ✅ Completed Tasks

#### Week 1-2: Security Fundamentals ✅
- ✅ Basic security audit (critical vulnerabilities only)
- ✅ Fix session handling issues
- ✅ Add security headers (CSP, HSTS, X-Frame-Options)
- ✅ Implement basic rate limiting
- ✅ Ensure path traversal protection
- ✅ Add CSRF tokens where missing

#### Week 2-3: Route Naming & Mobile Design ✅
- ✅ Move `/health` → `/.health`
- ✅ Move `/static` → `/.static`
- ✅ Update all template references
- ✅ Implement hamburger menu for mobile
- ✅ Create responsive layouts
- ✅ Add viewport meta tag
- ✅ Make touch-friendly (44px targets)

### 🚀 Remaining Tasks for 0.1 Release

#### Week 3-4: Basic File Upload Improvements
- Add drag-and-drop for images
- Implement upload progress bars
- Add file type validation
- Improve error messages
- Basic preview functionality
- Fix any upload security issues

#### Week 4-6: Polish & Release
- Testing and bug fixes
- Update documentation
- Create announcement materials
- Final security review

### 0.1 Release Messaging
**Target**: Developers and early adopters
**Positioning**: "A modern, AI-native CMS built with Rust - secure, fast, and designed for the future of content management."

**Key Features to Highlight**:
- First CMS with native MCP integration
- Built with Rust for performance and safety
- AI agents can manage content naturally
- 420+ tests ensuring reliability
- Mobile-responsive design
- Secure by default

### Post-0.1 Roadmap
- Version history and restoration
- Custom database templates
- Auto-hyperlinking with doxyde-tagger
- Generic file/attachment component
- Advanced image editing
- Comprehensive security testing
- Component drag-and-drop reordering UI

## Future Work (Full List)

### Urgent Priority (Post-0.1)
- Comprehensive security audit and hardening
- Complete mobile responsive design

#### Security Audit and Hardening
**Goal**: Comprehensive security review and testing to prevent vulnerabilities.

**Audit Areas**:
1. **Authentication & Sessions**
   - Session fixation attacks
   - Session hijacking
   - Brute force protection
   - Password policy enforcement
   - Secure cookie flags (HttpOnly, Secure, SameSite)

2. **Authorization**
   - Privilege escalation
   - Broken access control
   - Direct object references
   - Missing authorization checks
   - Cross-site access

3. **Input Validation**
   - SQL injection (despite SQLx protections)
   - XSS in user content
   - Path traversal in uploads
   - Command injection
   - LDAP/XML injection

4. **CSRF Protection**
   - Token validation on state-changing operations
   - Double submit cookies
   - Same-site cookie attributes

5. **File Security**
   - Upload restrictions (type, size, location)
   - Path traversal prevention
   - Executable file blocking
   - Virus scanning integration

6. **MCP/API Security**
   - Token entropy and storage
   - Rate limiting per token
   - API abuse prevention
   - Token revocation propagation

**Security Tests to Add**:
- Authentication bypass attempts (missing cookies, expired sessions)
- Authorization escalation (user→admin, cross-site access)
- SQL injection fuzzing (even with prepared statements)
- XSS payload testing (stored, reflected, DOM-based)
- Path traversal (../../../etc/passwd variations)
- CSRF token manipulation
- File upload exploits (.php, .exe, double extensions)
- Rate limiting effectiveness
- Input boundary testing (nulls, unicode, oversized)
- Concurrent session handling
- Token prediction/brute force

**Security Headers**:
```
Content-Security-Policy: default-src 'self'; script-src 'self' 'unsafe-inline'
X-Frame-Options: DENY
X-Content-Type-Options: nosniff
Strict-Transport-Security: max-age=31536000; includeSubDomains
Referrer-Policy: strict-origin-when-cross-origin
Permissions-Policy: geolocation=(), camera=(), microphone=()
```

**Additional Hardening**:
- Implement rate limiting (per IP, per user, per endpoint)
- Add request size limits
- Timeout long-running operations
- Log security events for monitoring
- Implement account lockout after failed attempts
- Add CAPTCHA for sensitive operations
- Regular dependency vulnerability scanning
- Security.txt file for responsible disclosure

#### Route Naming Conflicts Fix
**Goal**: Move all system routes to dot-prefix to avoid conflicts with content paths.

**Routes to Update**:
- `/health` → `/.health`
- `/static` → `/.static`

**Impact**:
- Update route definitions in `routes.rs`
- Update all template references from `/static/` to `/.static/`
- Update JavaScript references (e.g., `/static/js/clipboard.js`)
- Check for hardcoded paths in documentation
- Ensure backward compatibility or migration path

**Rationale**: 
- Consistent with existing dot-prefix pattern (/.login, /.admin, etc.)
- Prevents conflicts with user content at /health or /static paths
- Maintains clear separation between system and content routes

#### Mobile Responsive Design
**Goal**: Make Doxyde fully responsive for mobile and tablet devices.

**Navigation Changes**:
- **Hamburger Menu**: Replace sidebar with collapsible menu
- **Fixed Top Bar**: Sticky header with menu trigger
- **Action Bar**: Collapse into dropdown in mobile menu
- **Breadcrumbs**: Hide on mobile to save space
- **Root Page**: Add to hamburger menu for easy access

**Breakpoints**:
- Mobile: < 768px
- Tablet: 768px - 1024px  
- Desktop: > 1024px

**Mobile UI Elements**:
- Hamburger icon (three lines) in top-left
- Site title/logo centered in top bar
- User menu/login in top-right
- Slide-out navigation drawer from left
- Overlay backdrop when menu open
- Close button or swipe-to-close

**Touch Optimizations**:
- Minimum touch target size: 44x44px
- Larger tap areas for links
- Swipe gestures for navigation
- Smooth scrolling with momentum
- No hover-dependent UI elements

**Content Adjustments**:
- Single column layout on mobile
- Responsive images with max-width: 100%
- Horizontal scroll for wide tables
- Collapsible sections for long content
- Larger base font size (16px minimum)
- Increased line height for readability

**Performance**:
- CSS media queries (no JS dependency)
- Hardware-accelerated transitions
- Lazy loading for off-screen images
- Reduced motion for accessibility

**Accessibility**:
- ARIA labels for menu buttons
- Focus trap in open menu
- Keyboard navigation support
- High contrast mode support
- Respect prefers-reduced-motion

### High Priority
- Improved file upload system (images and generic files)
- Component reordering
- Page deletion with cascade
- Version history viewing and restoration
- Search functionality
- Auto-hyperlinking adjacent pages (using doxyde-tagger)
- Hyperlink component for external links
- Database-stored custom templates with MCP management

#### Improved File Upload System
**Goal**: Create a modern, user-friendly file upload experience for images and all file types.

**Image Upload Improvements**:
- **Drag-and-Drop**: Drop zone with visual feedback
- **Preview**: Show thumbnails before upload
- **Progress Bar**: Real-time upload progress
- **Bulk Upload**: Select/drop multiple images
- **Gallery Browser**: Browse/select from existing uploads
- **Image Editing**: Basic crop/resize capabilities
- **Paste Support**: Paste images from clipboard

**Generic File Upload Component**:
- **New Component Type**: `file` or `attachment`
- **Supported Files**: PDFs, docs, spreadsheets, zip, etc.
- **File Browser**: Organized file management UI
- **Metadata Storage**: Track size, type, upload date, uploader

**Database Schema**:
```sql
CREATE TABLE uploads (
    id INTEGER PRIMARY KEY,
    site_id INTEGER NOT NULL,
    filename TEXT NOT NULL,
    original_name TEXT NOT NULL,
    file_type TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    mime_type TEXT,
    uploaded_by TEXT,
    uploaded_at TEXT NOT NULL,
    category TEXT,
    is_public BOOLEAN DEFAULT false,
    metadata JSON,
    FOREIGN KEY (site_id) REFERENCES sites(id)
);
```

**UI/UX Features**:
- Modern upload widget with drop zone
- File type icons and previews
- Search and filter capabilities
- Folder/category organization
- Quick copy URL/embed code
- Thumbnail generation for images
- File size and type validation
- Chunked uploads for large files

**Component Schema**:
```json
{
  "file_id": 123,
  "display_name": "Download our brochure",
  "show_preview": true,
  "show_metadata": true,
  "download_button": true
}
```

**Security**:
- Virus scanning integration
- File type whitelist/blacklist
- Size limits per file type
- Secure file storage location
- Access control per file
- Sanitize filenames
- Prevent path traversal

**MCP Integration**:
- `upload_file` - Upload via base64 or URL
- `list_uploads` - Browse available files
- `delete_upload` - Remove files
- `update_file_metadata` - Edit file details

#### Auto-Hyperlinking Feature
**Goal**: Automatically create hyperlinks to adjacent pages (parent, children, siblings) when their titles appear in content.

**Implementation Notes**:
- Use the `doxyde-tagger` crate for HTML parsing and manipulation
- Scan markdown content after rendering for page titles
- Only link to pages that are visible/accessible to the current user
- Consider performance impact for large sites
- Add option to disable auto-linking on specific components

#### Hyperlink Component
**Goal**: Create a dedicated component type for managing external links.

**Component Schema**:
```json
{
  "url": "https://example.com",
  "text": "Link text",
  "title": "Optional hover text",
  "target": "_blank|_self",
  "rel": "nofollow|noopener|etc"
}
```

**Templates**:
- `default`: Simple inline link
- `button`: Styled as a button
- `card`: Card-style link preview
- `icon`: Link with icon

#### Version History & Restoration
**Goal**: View page version history and restore previous versions.

**Routes**:
- `/page/.history` - List all versions with metadata
- `/page/.history/<version>` - View specific version content
- `/page/.history/<version>/.restore` - Restore a version (POST)

**Implementation Notes**:
- Show version number, created_by, created_at, is_published status
- Display component count and types for each version
- Compare/diff view between versions
- Restore creates a new version (not overwrites)
- Only editors/admins can view history and restore
- Add "restored from version X" in version metadata

**UI Elements**:
- Timeline view of versions
- Quick preview on hover
- One-click restore with confirmation
- Highlight differences between versions

#### Database-Stored Custom Templates
**Goal**: Allow per-site customization of all templates through database storage, manageable via AI/MCP.

**Template Types**:
- **Page Templates**: `default`, `landing`, `blog`, custom...
- **Component Templates**: Per component type (text, image, code, etc.)
- **Layout Templates**: `base.html`, `header.html`, `footer.html`
- **Style Templates**: Custom CSS stored in DB

**Database Schema**:
```sql
CREATE TABLE site_templates (
    id INTEGER PRIMARY KEY,
    site_id INTEGER NOT NULL,
    template_type TEXT NOT NULL, -- 'page', 'component', 'layout', 'style'
    template_name TEXT NOT NULL,
    template_content TEXT NOT NULL,
    is_active BOOLEAN DEFAULT true,
    version INTEGER DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    created_by TEXT,
    UNIQUE(site_id, template_type, template_name, version)
);
```

**Override Hierarchy**:
1. Check DB for site-specific template
2. Fall back to file-based template
3. Fall back to default template

**MCP Tools**:
- `list_templates` - Get all templates for a site
- `get_template` - Retrieve specific template
- `create_template` - Add new custom template
- `update_template` - Modify existing template
- `delete_template` - Remove custom template
- `preview_template` - Test template with sample data
- `restore_template_version` - Rollback to previous version

**AI-Friendly Features**:
- Template syntax validation before save
- Live preview with test data
- Template variable documentation
- CSS/HTML linting
- Automatic versioning for rollback
- Template inheritance support

**Security Considerations**:
- Sanitize Tera template syntax
- Prevent template injection attacks
- Limit template complexity (no arbitrary code execution)
- Admin-only access by default

### Medium Priority
-  dynamic editing
- Media library
- Multi-language support
- Import/export

### Low Priority
- Plugin system
- Custom themes
- Advanced permissions
- Analytics

## Quick Reference

### Environment Variables
```bash
DATABASE_URL=sqlite:doxyde.db
RUST_LOG=info
PORT=3000
```

### Common Commands
```bash
# Development
cargo run --bin doxyde-web        # Start web server
cargo run --bin doxyde init       # Initialize database
cargo run --bin doxyde user create email@example.com username --admin --password pass

# Production
cargo build --release
./target/release/doxyde-web
```

### Troubleshooting

**"Domain must contain dot" error**: Use `localhost:3000` or `example.local`

**SQLx compilation errors**: Update .sqlx cache (see SQLx section above)

**Migration errors**: Check `_sqlx_migrations` table, migrations are idempotent

**Can't see edit links**: Verify user permissions with `doxyde user list`

**Static files not found (404)**: 
- Static files go in the `static/` directory (NOT `.static/`)
- The route `/.static` serves files from `static/` directory
- Example: `/.static/js/script.js` serves `static/js/script.js`
- Always create static assets in `static/` not `.static/`