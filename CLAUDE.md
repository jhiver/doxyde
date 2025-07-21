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
- **Frontend**: HTMX, vanilla CSS
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

**Code Quality:**
- Refactored large functions into small, focused ones
- Consistent error handling patterns
- Improved test coverage

## Future Work

### High Priority
- Component reordering
- Page deletion with cascade
- Version history viewing
- Search functionality

### Medium Priority
- HTMX dynamic editing
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