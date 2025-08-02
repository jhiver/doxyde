#!/bin/bash

# Fix Host imports
find doxyde-web/src -name "*.rs" -exec sed -i '' 's/use axum::{$/use axum:{\n    async_trait,/g' {} \;
find doxyde-web/src -name "*.rs" -exec sed -i '' 's/extract::{Host,/extract::{/g' {} \;
find doxyde-web/src -name "*.rs" -exec sed -i '' 's/extract::{\([^}]*\)Host,/extract::{\1/g' {} \;
find doxyde-web/src -name "*.rs" -exec sed -i '' 's/extract::{Host}/extract::{}/g' {} \;

# Add axum_extra Host import where needed
find doxyde-web/src -name "*.rs" -exec grep -l "Host(" {} \; | while read file; do
    if ! grep -q "use axum_extra::extract::Host" "$file"; then
        # Check if axum_extra is already imported
        if grep -q "use axum_extra::" "$file"; then
            # Add Host to existing import
            sed -i '' 's/use axum_extra::{/use axum_extra::{\n    extract::Host,/g' "$file"
        else
            # Add new import after axum imports
            sed -i '' '/^use axum::/a\
use axum_extra::extract::Host;
' "$file"
        fi
    fi
done

# Fix async_trait
sed -i '' 's/    async_trait,//' doxyde-web/src/auth.rs
sed -i '' '/^use axum::/i\
use async_trait::async_trait;
' doxyde-web/src/auth.rs

echo "Import fixes applied!"