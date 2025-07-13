# Doxyde

A modern, AI-native content management system built with Rust. Simple, fast, and extensible.

## Features

- **Multi-site Support**: Host multiple sites from a single instance
- **Hierarchical Pages**: Organize content with parent-child page relationships
- **Component-Based Content**: Build pages with reusable components (text, images)
- **Version Control**: Every edit creates a new version
- **Draft/Publish Workflow**: Edit safely without affecting the live site
- **Role-Based Access**: Admin, Owner, Editor, and Viewer roles
- **Image Management**: Upload and serve images with metadata
- **Markdown Support**: Write content in Markdown with automatic HTML conversion
- **Modern UI**: Responsive design with dark header/footer theme
- **Logo Support**: Custom logo per site with dimension control

## Quick Start

### Prerequisites

- Rust 1.70 or higher
- SQLite 3

### Installation

1. Clone the repository:
```bash
git clone https://github.com/yourusername/doxyde.git
cd doxyde
```

2. Build the project:
```bash
cargo build --release
```

3. Initialize the database:
```bash
./target/release/doxyde init
```

4. Create a site:
```bash
./target/release/doxyde site create localhost:3000 "My Site"
```

5. Create an admin user:
```bash
./target/release/doxyde user create admin@example.com admin --admin --password mypassword
```

6. Grant site access:
```bash
./target/release/doxyde user grant admin localhost:3000 owner
```

7. Start the web server:
```bash
./target/release/doxyde-web
```

8. Visit http://localhost:3000 and login at http://localhost:3000/.login

## Project Structure

```
doxyde/
├── doxyde-core/     # Domain models and business logic
├── doxyde-db/       # Database layer with SQLite
├── doxyde-web/      # Web server (Axum framework)
├── doxyde-ai/       # AI integration (planned)
├── doxyde-cli/      # Command-line interface
├── migrations/     # Database migrations
└── templates/      # HTML templates
```

## Configuration

Create a `.env` file in the project root:

```env
DATABASE_URL=sqlite:doxyde.db
RUST_LOG=info
PORT=3000
TEMPLATES_DIR=templates
```

## CLI Commands

### Site Management

```bash
# Create a new site
doxyde site create domain "Site Title"

# List all sites
doxyde site list

# Delete a site
doxyde site delete domain
```

### User Management

```bash
# Create a user
doxyde user create email username --password pass [--admin]

# Grant permissions
doxyde user grant username domain role
# Roles: owner, editor, viewer

# Change password
doxyde user password username newpassword

# List users
doxyde user list
```

### Database

```bash
# Initialize database
doxyde init

# Run migrations
doxyde migrate
```

## Usage Guide

### Content Management

1. **Login**: Navigate to `/.login` on any site
2. **Create Pages**: Click "New Page" in the action bar
3. **Edit Content**: Click "Edit" to modify page content
4. **Add Components**: Use the component editor to add text or images
5. **Save Drafts**: Changes are saved as drafts until published
6. **Publish**: Click "Save & Publish" to make changes live

### Page Features

- **Properties**: Edit page metadata, SEO settings, and templates
- **Markdown**: Write content in Markdown, automatically converted to HTML
- **Images**: Upload images with automatic format detection
- **Templates**: Choose from multiple component display templates

### URL Structure

Doxyde uses a hierarchical URL structure with dot-prefixed system actions:

- **Content URLs**: `/`, `/about`, `/products/widget`
- **System Actions**: 
  - `/.login` - User login
  - `/.logout` - User logout
  - `/about/.edit` - Edit page content
  - `/about/.properties` - Edit page properties
  - `/about/.new` - Create child page
  - `/about/.move` - Move page
  - `/about/.delete` - Delete page

## Development

### Running Tests

```bash
# Run all tests
cargo test --all

# Run with coverage
cargo tarpaulin --out Html
```

### Code Quality

```bash
# Format code
cargo fmt --all

# Run linter
cargo clippy --all-targets --all-features

# Check compilation
cargo check --all
```

### Development Mode

For template hot-reloading during development:
```bash
TEMPLATES_DIR=templates cargo run --bin doxyde-web
```

## Architecture

### Component System

Pages are built from components:
- **Text**: Markdown content with multiple display templates (default, card, highlight, quote)
- **Image**: Upload and display images with dimension control and templates

### Security

- Session-based authentication with secure cookies
- Password hashing with Argon2
- Role-based permissions at site level
- HTML sanitization for user content
- CSRF protection on forms

### Database

SQLite with migrations:
- Automatic migration on startup
- Version tracking in `_schema_version` table
- Migrations in `/migrations/` directory

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Development Guidelines

- Follow Rust naming conventions
- Write tests for new functionality
- Keep functions small and focused
- Add documentation for public APIs
- Run `cargo fmt` before committing
- Follow the one-function-at-a-time development rule

## Troubleshooting

### Common Issues

1. **Server won't start**
   - Check port 3000 is free
   - Ensure database exists (`doxyde init`)
   - Verify permissions on `doxyde.db`

2. **Login issues**
   - Verify user exists and has site permissions
   - Domain must match access URL (including port)

3. **Page not found**
   - Slugs must be lowercase with hyphens
   - Check page hierarchy
   - Ensure site exists for domain

## License

This project is dual-licensed under MIT OR Apache-2.0.

## Acknowledgments

Built with:
- [Axum](https://github.com/tokio-rs/axum) - Web framework
- [SQLx](https://github.com/launchbadge/sqlx) - Database toolkit
- [Tera](https://tera.netlify.app/) - Template engine
- [Argon2](https://github.com/RustCrypto/password-hashes) - Password hashing