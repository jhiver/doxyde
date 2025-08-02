# Migrating from Legacy to Multi-Site Database

Doxyde now supports multiple sites with separate databases for each domain. If you have an existing Doxyde installation with a single database, you'll need to migrate it to the new structure.

## Migration Tool

Use the `doxyde-migrate` tool to migrate your legacy database:

```bash
# Build the migration tool
cargo build --release --bin doxyde-migrate

# Basic migration
./target/release/doxyde-migrate --domain example.com

# Specify custom paths
./target/release/doxyde-migrate \
  --source /path/to/old/doxyde.db \
  --domain example.com \
  --sites-directory /var/lib/doxyde/sites

# Update domain in database if it doesn't match
./target/release/doxyde-migrate \
  --domain example.com \
  --update-domain
```

## Migration Options

- `--source` (default: `doxyde.db`) - Path to your existing database
- `--domain` (required) - Domain name for your site
- `--sites-directory` (default: `./sites`) - Where to store site databases
- `--force` - Overwrite if destination already exists
- `--update-domain` - Update the domain in the database to match specified domain
- `--dry-run` - Show what would be done without actually doing it

## Migration Process

1. The tool creates the appropriate directory structure:
   ```
   sites/
   └── example.com-a1b2c3/
       └── site.db
   ```

2. Copies your existing database to the new location

3. Verifies the migrated database is valid

4. Optionally updates the domain in the database if needed

## After Migration

1. Update your Doxyde configuration to use the new sites directory:
   
   **Option A - Environment Variable:**
   ```bash
   export SITES_DIRECTORY=/path/to/sites
   ```
   
   **Option B - Configuration File:**
   Create `/etc/doxyde.conf` or `./.doxyde.conf`:
   ```toml
   sites_directory = "/path/to/sites"
   ```

2. Start Doxyde normally:
   ```bash
   ./target/release/doxyde-web
   ```

## Subdomain Support

The new structure automatically shares databases between subdomains:
- `example.com` → `sites/example.com-a1b2c3/site.db`
- `www.example.com` → `sites/example.com-a1b2c3/site.db` (same database)
- `blog.example.com` → `sites/example.com-a1b2c3/site.db` (same database)

## Troubleshooting

### Domain Mismatch Warning
If you see a warning about domain mismatch, you have two options:
1. Use `--update-domain` flag to automatically update it
2. Manually update using SQLite:
   ```bash
   sqlite3 sites/example.com-*/site.db
   UPDATE sites SET domain = 'example.com' WHERE id = 1;
   ```

### Permission Issues
Ensure the user running Doxyde has read/write access to the sites directory:
```bash
sudo chown -R doxyde:doxyde /var/lib/doxyde/sites
```