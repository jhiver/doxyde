#!/bin/bash

# Remove all axum_extra::extract::Host imports
find doxyde-web/src -name "*.rs" -exec sed -i '' '/use axum_extra::extract::Host;/d' {} \;

# Revert auth.rs
sed -i '' 's/use async_trait::async_trait;/use axum::{/' doxyde-web/src/auth.rs
sed -i '' '/^use axum:{$/a\
    async_trait,
' doxyde-web/src/auth.rs

# Add Host back to extract imports where needed
files=(
    "doxyde-web/src/content.rs"
    "doxyde-web/src/error_middleware.rs"
    "doxyde-web/src/handlers/action.rs"
    "doxyde-web/src/handlers/auth.rs"
    "doxyde-web/src/rmcp/discovery.rs"
    "doxyde-web/src/rmcp/oauth.rs"
    "doxyde-web/src/routes.rs"
    "doxyde-web/src/site_resolver.rs"
    "doxyde-web/src/www_redirect.rs"
)

for file in "${files[@]}"; do
    echo "Fixing $file"
    # Add Host to extract import if not present
    sed -i '' 's/extract::{FromRequest,/extract:{Host, FromRequest,/' "$file"
    sed -i '' 's/extract::{/extract:{Host, /' "$file"
    sed -i '' 's/extract:{Host, Host,/extract:{Host,/' "$file"
done

echo "Reverted Host imports!"