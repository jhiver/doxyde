#!/bin/bash

# List of files that need Host import
files=(
    "doxyde-web/src/error_middleware.rs"
    "doxyde-web/src/handlers/action.rs"
    "doxyde-web/src/handlers/auth.rs"
    "doxyde-web/src/rmcp/discovery.rs"
    "doxyde-web/src/rmcp/oauth.rs"
    "doxyde-web/src/routes.rs"
    "doxyde-web/src/site_resolver.rs"
)

for file in "${files[@]}"; do
    echo "Fixing $file"
    # Add axum_extra::extract::Host after axum imports
    if ! grep -q "use axum_extra::extract::Host" "$file"; then
        # Find the last axum import line and add Host import after it
        awk '/^use axum::/ {p=NR} {a[NR]=$0} END {for(i=1;i<=NR;i++){print a[i]; if(i==p) print "use axum_extra::extract::Host;"}}' "$file" > "${file}.tmp" && mv "${file}.tmp" "$file"
    fi
done

echo "Host imports fixed!"