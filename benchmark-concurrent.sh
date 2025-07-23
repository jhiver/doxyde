#!/bin/bash

# Simple concurrent load test
URL="${1:-http://127.0.0.1:3000/.login}"
CONCURRENT="${2:-50}"
REQUESTS="${3:-1000}"

echo "Testing $URL with $CONCURRENT concurrent requests ($REQUESTS total)"
echo "Starting at $(date)"

# Function to make a request and measure time
make_request() {
    local start=$(date +%s.%N)
    curl -s -o /dev/null -w "%{http_code}" "$URL"
    local end=$(date +%s.%N)
    local duration=$(echo "$end - $start" | bc)
    echo "$duration"
}

export -f make_request
export URL

# Run requests in parallel
seq 1 $REQUESTS | xargs -P $CONCURRENT -I {} bash -c 'make_request'