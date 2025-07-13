#!/bin/bash
set -e

echo "Setting up test environment for image upload..."

# Initialize database
./target/release/doxyde init

# Create a site
./target/release/doxyde site create localhost:3000 "Test Site"

# Create admin user
./target/release/doxyde user create admin@test.com admin --admin --password testpass

# Grant site access
./target/release/doxyde user grant admin localhost:3000 owner

echo "Setup complete!"
echo ""
echo "To test image upload:"
echo "1. Start the server: ./target/release/doxyde-web"
echo "2. Visit http://localhost:3000/.login"
echo "3. Login with: admin / testpass"
echo "4. Create or edit a page and add an image component"
echo "5. Click 'Upload Image' button to test the upload functionality"