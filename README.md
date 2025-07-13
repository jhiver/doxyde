# Doxyde

## Introduction

Doxyde aims to be a modern, Rust-powered content management system built from the ground up to create sites that don’t suck — sites that are clean, fast, accessible, and genuinely pleasant to use, both for visitors and for the people managing them.

This project started as a personal challenge. In a previous life — back in the early 2000s — I had written a similar CMS. Doxyde is my way of reconnecting with that era, but with fresh eyes and modern tools. Inspired by the so-called "Vibe Coding" trend, this is also a learning journey: an opportunity to get back into serious development using today’s best practices — Rust, Axum, modern workflows, and AI-driven tooling.

The goal is simple: build something solid and future-proof. Content-first, with clear navigation, a logical structure, and the ability to extend things without everything falling apart. Over time, Doxyde will evolve to include AI-assisted workflows for content generation, SEO, and editorial planning — all while integrating cleanly with the tools people actually use today.

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

Doxyde is licensed under the GNU Affero General Public License v3 (AGPLv3). See the [LICENSE](LICENSE) file for the full license text.

This ensures that the source code remains open and that any modifications — including those used in SaaS platforms — must be shared under the same terms. The AGPLv3 specifically addresses network use, requiring that users who interact with the software over a network must have access to the source code.

Copyright (C) 2025 Doxyde Project Contributors

If you're interested in a different licensing model — for example, for commercial use without the obligations of the AGPL — let's talk: jhiver@gmail.com.

## Acknowledgments

Built with:
- [Axum](https://github.com/tokio-rs/axum) - Web framework
- [SQLx](https://github.com/launchbadge/sqlx) - Database toolkit
- [Tera](https://tera.netlify.app/) - Template engine
- [Argon2](https://github.com/RustCrypto/password-hashes) - Password hashing