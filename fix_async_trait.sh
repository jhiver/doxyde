#!/bin/bash

# List of files that need async_trait import fixed
files=(
    "doxyde-web/src/handlers/delete_page.rs"
    "doxyde-web/src/handlers/edit.rs"
    "doxyde-web/src/handlers/image_serve.rs"
    "doxyde-web/src/handlers/image_upload.rs"
    "doxyde-web/src/handlers/pages.rs"
    "doxyde-web/src/handlers/properties.rs"
    "doxyde-web/src/handlers/reorder.rs"
    "doxyde-web/src/rate_limit.rs"
    "doxyde-web/src/request_logging.rs"
    "doxyde-web/src/security_headers.rs"
    "doxyde-web/src/session_activity.rs"
    "doxyde-web/src/rmcp/handlers.rs"
)

for file in "${files[@]}"; do
    # Check if file has async_trait import in axum block
    if grep -q "use axum::{" "$file" && grep -q "async_trait," "$file"; then
        echo "Fixing $file"
        # Remove async_trait from axum imports
        sed -i '' '/^    async_trait,$/d' "$file"
        # Add async_trait import before axum
        sed -i '' '/^use axum::{/ i\
use async_trait::async_trait;
' "$file"
    fi
done

echo "Fixed async_trait imports!"