# Doxyde Development Guide

## Language Policy

**ALL code, comments, documentation, commit messages, and variable names MUST be in English.**

This includes:
- Code comments and documentation
- Variable, function, and type names
- Error messages and log output
- Git commit messages
- README and documentation files
- User-facing text in templates
- CLI output and help text

## Project Overview

Doxyde is a modern, AI-native content management system built with Rust. It's designed to be simple, fast, and extensible with AI capabilities.

**Current Status**: MVP COMPLETE! The system is now functional with web UI, authentication, and content management.

### 🎯 Project Status Summary

**✅ Completed Phases:**
- Phase 1: Project Setup - Complete
- Phase 2: Core Domain Models - Complete  
- Phase 3: Database Layer - Complete
- Phase 4: Authentication System - Complete
- Phase 5: MVP Web Interface - Complete
- Phase 6: UI/UX Improvements - Complete
- Phase 7: Image Upload System - Complete
- Phase 8: Modern CSS Design - Complete

**📊 Test Coverage:** 300+ tests passing across all crates

**🚀 Ready to Use:** See README.md for quick start guide

### Latest Updates (July 15, 2025)

**MCP Integration:**
- **MCP Token System**: Users can generate secure tokens to connect Doxyde with Claude Code
- **Token Management UI**: Available at `/.settings/mcp` for creating and revoking tokens
- **HTTP MCP Server**: Integrated directly into doxyde-web at `/.mcp/:token_id`
- **Per-Site Access**: Each token is limited to a specific site for security
- **Usage Tracking**: Last usage time is tracked for each token
- **Simple Integration**: Users just copy the generated URL into Claude Code as a custom connector

**MCP Error Handling (Important):**
- **Always return JSON-RPC format**: MCP endpoints must ALWAYS return JSON-RPC responses, even for errors
- **Never throw HTTP errors**: Don't use `AppError` or return HTTP status codes for MCP endpoints
- **Proper error structure**: Errors must have `jsonrpc`, `id`, and `error` fields with `code` and `message`
- **Tool errors propagate**: Tool handlers should propagate errors (use `?`) instead of catching them
- **Error codes**: Use `-32602` for invalid params, `-32603` for internal/execution errors
- **Example**: When a tool fails, return `{"jsonrpc": "2.0", "id": 1, "error": {"code": -32603, "message": "Error details"}}`

### Previous Updates (July 13, 2025)

**Image Upload System:**
- **Full image upload functionality**: Support for uploading images to components
- **Image serving**: Slug-based URLs for images (e.g., /my-image.jpg)
- **Multiple formats**: Support for JPEG, PNG, GIF, WebP, and SVG
- **Organized storage**: Date-based directory structure (year/month/day)
- **Metadata extraction**: Automatic extraction of dimensions and format
- **Component integration**: Images stored as JSON with full metadata

**Modern UI/CSS Design:**
- **Professional typography**: Inter font for body, JetBrains Mono for code
- **Dark header/footer**: Consistent dark theme with white text
- **CSS variables**: Complete design system with color, spacing, and shadow tokens
- **Responsive design**: Mobile-friendly with proper breakpoints
- **Improved forms**: Modern input styling with focus states
- **Button variants**: Primary, secondary, success, and danger styles

**Branding Update:**
- **Renamed to Doxyde**: All references updated from Doxyde to Doxyde
- **Header shows page title**: Logo displays root page title instead of site title
- **Better error handling**: Enhanced template error logging for debugging

### Previous Updates (July 11, 2025)

**Migration System:**
- **Simple version tracking**: Single `_schema_version` table tracks current database version
- **Automatic migrations**: Server checks and applies migrations on startup if needed
- **Clean migration naming**: Simple numbered format (001_initial.sql, 002_auth.sql, etc.)
- **Uses sqlx::migrate!**: Leverages SQLx's built-in migration system for reliability
- **Clear logging**: Shows when migrations run or when schema is already up to date

### Previous Updates (July 10, 2025)

**UI/UX Improvements:**
- **Full-width layout**: Removed max-width constraint, now uses entire screen width
- **Two-column layout**: Sidebar (250px) on left for navigation, main content on right
- **Improved action bar**: Yellow sticky bar with text links (not buttons), current action shown in bold
- **Better navigation**: Relative links throughout, fixed double-slash issues
- **Enhanced templates**: Robust variable handling to prevent template errors
- **Password management**: Added CLI command to change user passwords

### Progress Summary

#### ✅ Phase 1: Project Setup (Complete)
- Initialized Rust workspace with 4 crates (doxyde-core, doxyde-db, doxyde-web, doxyde-mcp)
- Set up dependencies and project structure
- Created initial SQLite migration schema
- Added .env and .gitignore files

#### ✅ Phase 2: Core Domain Models (Complete)
- **Site Model** (doxyde-core/src/models/site.rs):
  - `new(domain, title)` - Constructor with auto-timestamps
  - `validate_domain()` - Ensures valid domain format
  - `validate_title()` - Ensures non-empty, max 255 chars
  - `is_valid()` - Combines all validations
  - 21 comprehensive tests

- **Page Model** (doxyde-core/src/models/page.rs):
  - `new(site_id, slug, title)` - Constructor with site reference
  - `new_with_parent(site_id, parent_page_id, slug, title)` - Constructor for child pages
  - `validate_slug()` - Ensures URL-friendly format
  - `validate_title()` - Same as Site validation
  - `is_valid()` - Combines all validations
  - Added hierarchical fields: `parent_page_id`, `position`
  - 24 comprehensive tests

#### ✅ Phase 3: Database Layer (Complete)
- **All domain model repositories implemented**:
  - SiteRepository with 45 tests
  - PageRepository with hierarchical support and 63 tests
  - ComponentRepository with 17 tests
  - PageVersionRepository with 13 tests
  - UserRepository with full authentication support
  - SessionRepository for session management
  - SiteUserRepository for role-based permissions
- **Total: 227 tests passing** (73 core + 154 db)

#### ✅ Phase 4: Authentication System (Complete)
- **User Management**:
  - User model with secure password hashing (Argon2)
  - Session-based authentication with cookies
  - Role-based permissions (Admin, Site Owner/Editor/Viewer)
- **Security Features**:
  - Secure session tokens
  - HTTP-only cookies
  - Permission checks on all protected routes
- **Authentication Flow**:
  - Login/logout handlers
  - Session middleware
  - CurrentUser extractor for protected routes

#### ✅ Phase 5: MVP Web Interface (Complete)
- **Axum Web Server** (doxyde-web):
  - Full routing with dot-prefixed actions (/.login, /.edit, /.new)
  - Dynamic content resolution with hierarchical page navigation
  - Fallback handler for content vs. action routing
- **Content Management Features**:
  - View pages with hierarchical navigation
  - Edit page titles
  - Add text components to pages
  - Create new child pages
  - Breadcrumb navigation
- **Templates** (Tera):
  - Base layout with navigation
  - Login form
  - Page view with component rendering
  - Page edit interface
  - New page creation form
- **CLI Tool** (doxyde-cli):
  - Initialize database: `doxyde init`
  - Create sites: `doxyde site create domain "title"`
  - Create users: `doxyde user create email username --password pass [--admin]`
  - Grant permissions: `doxyde user grant username domain role`

## 🚨 ULTRA IMPORTANT: MANDATORY CODING GUIDELINES 🚨

**THIS IS THE MOST CRITICAL SECTION OF THIS DOCUMENT**

Since Rust is hard, you MUST follow this EXACT development process:

### STRICT ONE-FUNCTION-AT-A-TIME RULE

1. **IMPLEMENT EXACTLY ONE FUNCTION**
   - Write ONE and ONLY ONE function
   - Do NOT write multiple functions at once
   - Do NOT plan ahead for other functions
   - STOP after implementing the function

2. **IMMEDIATELY WRITE TESTS FOR THAT FUNCTION**
   - Tests MUST be written before moving to any other code
   - Write comprehensive tests covering:
     - Happy path
     - Edge cases
     - Error cases
   - Tests go in the same file in a `#[cfg(test)]` module

3. **VERIFY ALL TESTS PASS**
   - Run `cargo test`
   - If compilation fails: STOP AND ULTRATHINK
   - If tests fail: STOP AND ULTRATHINK
   - Do NOT proceed until ALL tests pass
   - Do NOT just fix the tests - understand WHY they failed

4. **ONLY THEN MOVE TO THE NEXT FUNCTION**
   - After and ONLY after all tests pass
   - Repeat this cycle for every single function

### ⛔ FORBIDDEN PRACTICES
- ❌ Writing multiple functions before testing
- ❌ Writing placeholder or stub functions
- ❌ Skipping tests "for now"
- ❌ Moving forward with failing tests
- ❌ Implementing a whole module at once
- ❌ Using `unwrap()` or `expect()` anywhere in the code
- ❌ Ignoring error handling
- ❌ Writing functions longer than 30 lines
- ❌ Nesting more than 3 levels deep
- ❌ Having more than 4 parameters per function

### ✅ REQUIRED PRACTICES
- ✅ One function → its tests → verify → next function
- ✅ Stop and think deeply when tests fail
- ✅ Understand root causes, not symptoms
- ✅ Clean, thoughtful fixes over quick patches
- ✅ Keep functions short and focused (max 30 lines)
- ✅ Extract complex logic into helper functions
- ✅ Write at least one test per function
- ✅ Test edge cases and error conditions
- ✅ Use descriptive function names that explain what they do

### Example Development Flow
```rust
// Step 1: Write ONE function
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

// Step 2: Immediately write tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 3), 5);
        assert_eq!(add(-1, 1), 0);
        assert_eq!(add(0, 0), 0);
    }
}

// Step 3: Run tests and ensure they pass
// $ cargo test
// Only after all tests pass, move to next function
```

### Code Readability Guidelines

**Function Length**
- Maximum 30 lines per function (excluding tests)
- If a function is getting long, extract helper functions
- Each function should do ONE thing well

**Example of refactoring a long function:**
```rust
// ❌ BAD: Too long and does too many things
pub fn process_request(request: Request) -> Result<Response> {
    // 50+ lines of validation, processing, formatting...
}

// ✅ GOOD: Split into focused functions
pub fn process_request(request: Request) -> Result<Response> {
    let validated = validate_request(&request)?;
    let result = execute_operation(validated)?;
    format_response(result)
}

fn validate_request(request: &Request) -> Result<ValidatedRequest> {
    // 10-15 lines focused on validation
}

fn execute_operation(request: ValidatedRequest) -> Result<OperationResult> {
    // 10-15 lines focused on business logic
}

fn format_response(result: OperationResult) -> Result<Response> {
    // 10-15 lines focused on formatting
}
```

**Nesting Depth**
- Maximum 3 levels of nesting
- Use early returns to reduce nesting
- Extract complex conditions into well-named functions

**Testing Requirements**
- Every public function must have tests
- Test the happy path, edge cases, and error cases
- Use descriptive test names that explain what is being tested

## Development Workflow

### Before marking ANY task as complete

1. **Format the code**:
   ```bash
   cargo fmt --all
   ```

2. **Check for common issues**:
   ```bash
   cargo clippy --all-targets --all-features
   ```

3. **Ensure compilation**:
   ```bash
   cargo check --all
   ```

4. **Run all tests**:
   ```bash
   cargo test --all
   ```

### Error Handling Guidelines

**NEVER use `expect()` or `unwrap()`** - Handle all errors explicitly:

```rust
// ❌ FORBIDDEN - These will panic
let value = some_option.unwrap();
let result = some_result.expect("this should work");

// ✅ REQUIRED - Handle errors explicitly
let value = match some_option {
    Some(v) => v,
    None => return Err(anyhow!("Value was None")),
};

// ✅ Or use the ? operator with proper error types
let value = some_option
    .ok_or_else(|| anyhow!("Value was None"))?;
```

**AVOID** using naked `?` operator:
```rust
// ❌ BAD - No context when error occurs
let result = some_operation()?;

// ✅ GOOD - Add context
let result = some_operation()
    .context("Failed to perform some_operation")?;

// ✅ BETTER - Add specific context
let result = some_operation()
    .with_context(|| format!("Failed to perform operation on id: {}", id))?;
```

## Testing Strategy

### Unit Tests
Every public function should have at least one test:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_functionality() {
        // Arrange
        let input = ...;
        
        // Act  
        let result = function(input);
        
        // Assert
        assert_eq!(result, expected);
    }
}
```

### Integration Tests with SQLite
Use sqlx::test for database tests:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;
    
    #[sqlx::test]
    async fn test_repository_operation(pool: SqlitePool) -> Result<()> {
        let repo = SiteRepository::new(pool);
        
        // Test operations
        let site = Site::new("example.com".to_string(), "Example".to_string());
        let id = repo.create(&site).await?;
        
        let found = repo.find_by_id(id).await?;
        assert!(found.is_some());
        
        Ok(())
    }
}
```

## Database Migration System

The project uses a simple but effective migration system:

### How it works
1. On startup, the server checks for a `_schema_version` table
2. If the current version doesn't match the expected version, it runs all migrations
3. Migrations are stored in `/migrations/` with simple numbered names (001_initial.sql, etc.)
4. Uses SQLx's built-in `migrate!` macro for reliable execution

### Adding new migrations
1. Create a new SQL file in `/migrations/` with the next number (e.g., `004_new_feature.sql`)
2. Update the `expected_version` in `doxyde-web/src/db.rs`
3. The migration will run automatically on next server start

### Migration files
- `001_initial.sql` - Base schema (sites, pages, components, versions)
- `002_auth.sql` - Authentication tables (users, sessions, permissions)
- `003_hierarchy.sql` - Hierarchical page structure updates

## SQLite Type Mappings

### DateTime Handling
SQLite doesn't have native DateTime type. Use these patterns:

```rust
// For queries, use type overrides
sqlx::query_as!(
    Site,
    r#"
    SELECT 
        id as "id: i64",
        domain as "domain: String", 
        title as "title: String",
        created_at as "created_at: chrono::DateTime<chrono::Utc>",
        updated_at as "updated_at: chrono::DateTime<chrono::Utc>"
    FROM sites
    WHERE id = ?
    "#,
    id
)
```

### Nullable Columns
For potentially NULL values (like MAX()):
```rust
// Use Option and handle NULL case
let row = sqlx::query!(
    r#"SELECT MAX(version_number) as "max_version: Option<i32>" FROM page_versions WHERE page_id = ?"#,
    page_id
)
.fetch_one(&self.pool)
.await?;

let next_version = row.max_version.unwrap_or(0) + 1;
```

### Integer Types
- SQLite INTEGER PRIMARY KEY returns `i64`
- If your model uses `i32`, cast explicitly:
  ```rust
  position as "position: i32"
  ```

## Common Issues and Solutions

### Issue: sqlx compile-time verification fails
**Solution**: Use explicit type annotations:
```rust
// Instead of
SELECT id, name FROM table

// Use
SELECT 
    id as "id: i64",
    name as "name: String"
FROM table
```

### Issue: DateTime conversion errors
**Solution**: Ensure migrations use TEXT for timestamps:
```sql
created_at TEXT NOT NULL DEFAULT (datetime('now')),
updated_at TEXT NOT NULL DEFAULT (datetime('now'))
```

### Issue: Tests need database
**Solution**: Use in-memory SQLite for tests:
```rust
#[sqlx::test]
async fn test_something(pool: SqlitePool) -> Result<()> {
    // pool is automatically created as in-memory database
}
```

## Migration System

Doxyde uses SQLx's built-in migration system with custom version tracking:

- Migrations are stored in the `/migrations/` directory
- Migration files use timestamp prefixes (e.g., `20250711_add_page_metadata.sql`)
- The system tracks the current schema version in the `_schema_version` table
- Migrations run automatically on server startup if needed

Current migration version: `20250712_add_draft_support`

## Page Metadata System

Pages now support comprehensive metadata for SEO and customization:

### Metadata Fields

1. **Title** (required): The page title displayed in navigation and browser tabs
2. **Description** (optional): SEO description, max 500 characters
3. **Keywords** (optional): Comma-separated keywords for SEO, max 255 characters
4. **Template** (required): Layout template selection
   - `default`: Standard page layout
   - `full_width`: Full-width content without sidebars
   - `landing`: Landing page layout
   - `blog`: Blog post layout
5. **SEO Fields**:
   - **meta_robots**: Search engine instructions (index,follow | noindex,follow | etc.)
   - **canonical_url**: Override canonical URL for duplicate content
   - **og_image_url**: Social media preview image URL
   - **structured_data_type**: Schema.org type (WebPage, Article, BlogPosting, etc.)

### Editing Interface

The page editing interface provides:

1. **Properties Mode** (`.properties`): Edit page metadata
   - Title, description, keywords
   - Template selection
   - SEO settings (robots, canonical, structured data)
   - Social media preview image
   
2. **Edit Mode** (`.edit`): Manage page content with draft/publish workflow
   - Single form for all components
   - Inline editing with Markdown support
   - Component templates (default, card, highlight, etc.)
   - Three actions: Cancel Changes, Save Draft, Save & Publish

This approach provides a safe editing environment where changes don't go live until explicitly published.

### URL Actions

- `/page/.properties` - Edit page metadata
- `/page/.content` - Edit page content/components
- `/page/.edit` - Redirects to `.content` for backward compatibility

## Environment Setup

Create `.env` file:
```
DATABASE_URL=sqlite:doxyde.db
```

## Architecture Decisions

1. **SQLite over PostgreSQL**: Simpler deployment, sufficient for CMS needs
2. **HTMX over heavy JS**: Server-side rendering with selective interactivity
3. **Component-based pages**: Flexible content structure
4. **Version control built-in**: Every edit creates a version
5. **AI-first design**: Command API for AI agents
6. **Dot-prefixed action URLs**: System actions use `.` prefix to avoid conflicts with content

### URL Routing Strategy

Doxyde uses a hierarchical URL structure where content paths are kept separate from system actions:

**Content URLs** (resolved dynamically):
- `/` - Site homepage
- `/about` - Top-level page
- `/about/team` - Nested page
- `/products/widget-x` - Product page

**System Action URLs** (prefixed with `.`):
- `/.login` - User login
- `/.logout` - User logout
- `/.admin` - Site administration
- `/about/.edit` - Edit the "about" page
- `/about/.new` - Create new child page under "about"
- `/about/.move` - Move/rename the "about" page
- `/about/.history` - View version history
- `/about/.add-component` - Add component to page

This design provides several benefits:
- **No URL conflicts**: Users can create pages with any name (including "login", "admin", etc.)
- **Clear action semantics**: The `.` prefix clearly indicates a system action
- **Granular permissions**: Each action can have its own permission checks
- **RESTful-like**: Actions are discoverable and consistent
- **Familiar pattern**: Similar to Unix hidden files or Zope/Plone's `@@view` pattern

The main content handler (`fallback` route) parses the URL to:
1. Extract the content path and optional action
2. Resolve the site from the Host header
3. Navigate the page hierarchy to find the target page
4. Route to the appropriate handler based on the action (or display the page)

## Code Style

- Use `cargo fmt` before every commit
- Follow Rust naming conventions
- Add doc comments for public APIs
- Keep functions small and focused
- Write tests alongside implementation

## Planned Architecture (To Be Built)

- **doxyde-core**: Domain models, business logic, and services
- **doxyde-db**: Database layer with SQLite and sqlx
- **doxyde-web**: Web server using Axum framework  
- **doxyde-mcp**: MCP server for AI integration

## Quick Commands

```bash
# Format, check and test everything
cargo fmt --all && cargo clippy --all && cargo test --all
```

## Getting Started

### Quick Start

```bash
# Build the project
cargo build --release

# Initialize database
./target/release/doxyde init

# Create a site
./target/release/doxyde site create localhost:3000 "My Site"

# Create admin user
./target/release/doxyde user create admin@example.com admin --admin --password mypassword

# Grant site access
./target/release/doxyde user grant admin localhost:3000 owner

# Start the web server
./target/release/doxyde-web
```

Visit http://localhost:3000 and login at http://localhost:3000/.login

### Key Features

1. **Hierarchical Page Structure**: Pages can have parent-child relationships
2. **Component-Based Content**: Pages contain versioned components (text, images, etc.)
3. **Version Control**: Every edit creates a new version
4. **Role-Based Access**: Admin, Owner, Editor, and Viewer roles
5. **Dot-Prefixed Actions**: System URLs use `.` prefix to avoid conflicts
6. **Modern UI**: Full-width layout with sidebar navigation and sticky action bar
7. **Domain Flexibility**: Supports localhost and localhost:port for development
8. **Draft/Publish Workflow**: Edit content safely without affecting live site
9. **Markdown Support**: Write content in Markdown, renders to safe HTML
10. **Component Templates**: Multiple display options for components
11. **Image Management**: Upload and serve images with automatic metadata extraction
12. **Modern UI**: Professional design with Inter font and dark header/footer theme

### 🔧 Current MCP Development Tasks

**Completed:**
- **delete_page MCP tool**: Page deletion with comprehensive safety checks ✅
- **move_page MCP tool**: Page hierarchy management with circular reference prevention ✅
- **Component management**: Markdown-specific tools (create, update, delete, list, get) ✅
- **Draft/publish tools**: publish_draft and discard_draft for managing page versions ✅

**Pending:**
- Comprehensive tests for all new MCP tools
- Support for other component types (image, code, html, etc.)
- Component reordering functionality

## Next Implementation Tasks

### 🎉 MVP COMPLETE! 🎉

The minimum viable product is now functional with:
- ✅ User authentication and sessions
- ✅ Site and page management
- ✅ Content editing with text components
- ✅ Hierarchical page navigation
- ✅ CLI tool for administration

### 🚀 Future Enhancements

#### High Priority
1. **Component Management**:
   - Edit existing components
   - Delete components
   - Reorder components
   - Support for more component types (image, code, markdown)

2. **Page Management**:
   - Delete pages
   - Move pages in hierarchy
   - Bulk operations

3. **Version History**:
   - View version history
   - Restore previous versions
   - Diff viewer

#### Medium Priority
1. **HTMX Integration**:
   - Dynamic component editing
   - Live preview
   - Drag-and-drop reordering

2. **Media Management**:
   - File upload support
   - Image component type
   - Media library

3. **Search**:
   - Full-text search
   - Search within site
   - Search filters

#### Low Priority
1. **AI Integration** (doxyde-mcp):
   - MCP server implementation
   - AI-assisted content creation
   - Content suggestions

2. **Advanced Features**:
   - Import/export
   - Multi-language support
   - Custom themes
   - Plugin system

### Important Implementation Notes

1. **Use sqlx query macros** for compile-time SQL verification
2. **Handle SQLite datetime** - Use TEXT columns with explicit type annotations
3. **Use sqlx::test** for all database tests (provides in-memory SQLite)
4. **Error handling** - Add context to all database operations
5. **No unwrap/expect** - All errors must be properly handled

## Draft/Publish System

Doxyde uses a draft/publish workflow for content editing:

### How it Works

1. **Draft Creation**: When editing a page, a draft version is automatically created
2. **Inline Editing**: All components can be edited in a single form
3. **Auto-save**: Changes are saved to the draft without affecting the published version
4. **Publishing**: "Save & Publish" makes the draft the new live version
5. **Canceling**: "Cancel Changes" deletes the draft and reverts to published version

### Key Components

- **PageVersion.is_published**: Boolean flag indicating if version is live
- **Draft helpers** (doxyde-web/src/draft.rs):
  - `get_or_create_draft()`: Get existing draft or create new one
  - `publish_draft()`: Mark draft as published
  - `delete_draft_if_exists()`: Remove unpublished draft

### Component Templates

Components now support different display templates:
- **default**: Just the content
- **with_title**: Title + content
- **card**: Content in a bordered card
- **highlight**: Content with colored background
- **quote**: Styled as a quotation
- **hidden**: Present but not displayed

### Markdown Support

Text components support Markdown with:
- Tables, strikethrough, footnotes, task lists
- Automatic HTML sanitization for security
- Tera filter: `{{ content | markdown }}`

### Example Repository Pattern

```rust
impl SiteRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, site: &Site) -> Result<i64> {
        let result = sqlx::query!(
            r#"
            INSERT INTO sites (domain, title, created_at, updated_at)
            VALUES (?, ?, ?, ?)
            "#,
            site.domain,
            site.title,
            site.created_at,
            site.updated_at
        )
        .execute(&self.pool)
        .await
        .context("Failed to create site")?;

        Ok(result.last_insert_rowid())
    }
}
```

### Testing Pattern

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;
    
    #[sqlx::test]
    async fn test_create_site(pool: SqlitePool) -> Result<()> {
        let repo = SiteRepository::new(pool);
        let site = Site::new("example.com".to_string(), "Example".to_string());
        
        let id = repo.create(&site).await?;
        assert!(id > 0);
        
        Ok(())
    }
}
```

## Troubleshooting

### Common Issues

1. **"Domain must contain at least one dot" error**
   - Use fully qualified domains like `localhost.local` or `localhost:3000`
   - Not just `localhost`

2. **"index already exists" on web server start**
   - This is a known issue with migrations
   - The server will still work correctly

3. **Can't see edit links**
   - Make sure you're logged in
   - Verify you have Editor or Owner permissions on the site

4. **Password not working with CLI**
   - Use the `--password` flag: `doxyde user create email username --password mypass`
   - Don't pipe passwords through stdin