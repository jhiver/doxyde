# Doxyde Multi-Site Database Architecture

## Overview

Doxyde now supports a multi-database architecture where each domain gets its own isolated SQLite database. This provides complete data isolation between sites, improves security, and enables better scalability.

## Key Features

- **Per-Domain Isolation**: Each domain gets its own SQLite database
- **Subdomain Sharing**: Subdomains (www, api, etc.) share the parent domain's database
- **Backward Compatibility**: Existing single-database installations continue to work
- **Zero-Downtime Migration**: Migrate from single to multi-database mode without downtime
- **Automatic Database Creation**: New sites automatically get their own database

## Architecture

```
sites/
├── example-com-a1b2c3d4/
│   └── site.db              # Database for example.com (and www.example.com)
├── other-site-com-e5f6g7h8/
│   └── site.db              # Database for other-site.com
└── localhost-i9j0k1l2/
    └── site.db              # Database for localhost development
```

## Configuration

### Enable Multi-Database Mode

Update your configuration file to enable multi-database mode:

```toml
# /etc/doxyde/doxyde.toml or ~/.doxyde/config.toml
[database]
sites_directory = "/var/lib/doxyde/sites"  # Directory for site databases
multi_site_mode = true                      # Enable multi-database mode

# The main database_url is still used for OAuth tokens and cross-site data
database_url = "sqlite:/var/lib/doxyde/doxyde.db"
```

Or use environment variables:

```bash
export DOXYDE_DATABASE_SITES_DIRECTORY="/var/lib/doxyde/sites"
export DOXYDE_DATABASE_MULTI_SITE_MODE=true
```

### Directory Permissions

Ensure the sites directory has proper permissions:

```bash
sudo mkdir -p /var/lib/doxyde/sites
sudo chown doxyde:doxyde /var/lib/doxyde/sites
sudo chmod 755 /var/lib/doxyde/sites
```

## Migration from Single Database

### 1. Check Current Database

First, inspect your existing database:

```bash
# Build the migration tool
cd /path/to/doxyde
rustc --edition 2021 -o doxyde-migrate migrate-to-multidb.rs \
  $(pkg-config --cflags --libs sqlite3) \
  -L target/release/deps \
  --extern sqlx=target/release/deps/libsqlx-*.rlib \
  --extern anyhow=target/release/deps/libanyhow-*.rlib \
  --extern clap=target/release/deps/libclap-*.rlib \
  --extern sha2=target/release/deps/libsha2-*.rlib \
  --extern hex=target/release/deps/libhex-*.rlib \
  --extern tokio=target/release/deps/libtokio-*.rlib

# Or compile with cargo if set up as a proper binary
cargo build --release --bin doxyde-migrate

# Check database info
./doxyde-migrate info --database /path/to/doxyde.db
```

### 2. Dry Run Migration

Always do a dry run first:

```bash
./doxyde-migrate to-multi-db \
  --source /path/to/doxyde.db \
  --target-dir /var/lib/doxyde/sites \
  --dry-run
```

### 3. Perform Migration

```bash
# Stop Doxyde
sudo systemctl stop doxyde

# Run migration
./doxyde-migrate to-multi-db \
  --source /path/to/doxyde.db \
  --target-dir /var/lib/doxyde/sites

# Update configuration (as shown above)

# Start Doxyde with new configuration
sudo systemctl start doxyde
```

### 4. Verify Migration

```bash
# Check that site databases were created
ls -la /var/lib/doxyde/sites/

# Test each site
curl -I https://example.com
curl -I https://other-site.com
```

## How It Works

### Domain Resolution

1. When a request comes in, the Host header is extracted
2. The domain is normalized (lowercase, port removed)
3. Base domain is extracted (e.g., `api.example.com` → `example.com`)
4. A SHA256 hash of the base domain creates a unique 8-character site key
5. Database path: `sites/{base-domain}-{site-key}/site.db`

### Database Routing

The `DatabaseRouter` component:
- Maintains a pool of database connections (one per site)
- Automatically creates databases for new sites
- Runs migrations on first access
- Handles connection pooling with configurable limits

### Request Flow

1. `site_resolver_middleware` - Extracts domain and creates `SiteContext`
2. `database_injection_middleware` - Gets appropriate database pool
3. Handlers receive site-specific database via `SiteDatabase` extractor

## API and MCP Compatibility

The multi-database architecture is fully compatible with:
- REST API endpoints
- MCP (Model Context Protocol) tools
- OAuth authentication (tokens stored centrally)

## Security Benefits

1. **Complete Isolation**: Sites cannot access each other's data
2. **Reduced Attack Surface**: Compromise of one site doesn't affect others
3. **Easier Compliance**: Data can be physically separated per customer
4. **Simplified Backups**: Per-site backup and restore

## Performance Considerations

- Each site has its own SQLite write lock (better concurrency)
- Connection pools are managed per-site (5 max connections default)
- Databases are created on-demand (no overhead for unused sites)
- File-based isolation enables easy horizontal scaling

## Troubleshooting

### Site Not Found Error

If you get "Site not found" errors:
1. Check the sites directory exists and has proper permissions
2. Verify the domain exists in the database
3. Check logs for database creation errors

### Migration Issues

If migration fails:
1. Check disk space in target directory
2. Verify source database is not corrupted
3. Run with `RUST_LOG=debug` for detailed output
4. Keep the original database - it's not modified

### Performance Issues

If experiencing slow performance:
1. Check disk I/O on the sites directory
2. Consider using SSD storage
3. Monitor number of open file descriptors
4. Adjust connection pool settings if needed

## Best Practices

1. **Regular Backups**: Back up the entire sites directory
2. **Monitoring**: Monitor disk usage in sites directory
3. **Capacity Planning**: Plan for ~10MB per site initially
4. **Migration Testing**: Always test migration on a copy first
5. **Gradual Rollout**: Test with a few sites before full migration

## Rollback Procedure

If you need to rollback to single-database mode:

1. Stop Doxyde
2. Update configuration:
   ```toml
   [database]
   multi_site_mode = false
   # Remove or comment out sites_directory
   ```
3. Start Doxyde (will use original database)
4. Sites directory remains intact for future use

## Future Enhancements

- Automatic site archival for inactive domains
- Per-site backup/restore tools
- Database size monitoring and alerts
- Cross-site data migration tools
- Read-only replica support