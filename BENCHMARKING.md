# Doxyde Performance Benchmarking Guide

## Overview

This guide provides comprehensive instructions for benchmarking Doxyde's performance across different layers of the application stack.

## Quick Start

### 1. Install Required Tools

```bash
# macOS
brew install wrk vegeta k6

# Linux
apt-get install wrk
go install github.com/tsenart/vegeta@latest
# Download k6 from https://k6.io/docs/getting-started/installation/

# Rust benchmarking
cargo install cargo-criterion
```

### 2. Run Basic Load Test

```bash
# Start Doxyde in release mode for accurate results
cargo build --release
./target/release/doxyde-web

# In another terminal, run basic throughput test
wrk -t12 -c400 -d30s http://localhost:3000/
```

## Benchmark Types

### 1. Micro-benchmarks (Rust Level)

Located in `benches/` directory. These test individual components:

```bash
# Run all benchmarks
cargo bench

# Run specific benchmark
cargo bench page_create

# Generate HTML report
cargo criterion --output-format=html
# Open target/criterion/report/index.html
```

**Current benchmarks:**
- `repository_bench.rs` - Database operations
  - Page creation
  - Page retrieval
  - Page listing
  - Tree generation
  - Search operations

### 2. HTTP Load Testing

#### Basic Throughput Test
```bash
# Test homepage with 12 threads, 400 connections for 30 seconds
wrk -t12 -c400 -d30s http://localhost:3000/

# Test with custom script for multiple endpoints
wrk -t4 -c100 -d30s -s scripts/mixed-endpoints.lua http://localhost:3000/
```

#### Constant Rate Testing (Vegeta)
```bash
# Test at 1000 requests/second for 30 seconds
echo "GET http://localhost:3000/" | vegeta attack -rate=1000/s -duration=30s | vegeta report

# Test multiple endpoints
cat targets.txt | vegeta attack -rate=500/s -duration=1m | vegeta report

# Generate latency plot
cat targets.txt | vegeta attack -rate=500/s -duration=1m | vegeta plot > latency.html
```

#### Complex Scenarios (k6)
```bash
# Run basic load test with ramping users
k6 run load-tests/basic-load-test.js

# Run MCP benchmark
k6 run load-tests/mcp-benchmark.js

# Output results to JSON
k6 run --out json=results.json load-tests/basic-load-test.js
```

### 3. Database Performance

#### SQLite Specific Benchmarks
```bash
# Test raw SQLite performance
sqlite3 doxyde.db << EOF
.timer on
SELECT COUNT(*) FROM pages;
SELECT * FROM pages WHERE site_id = 1 LIMIT 100;
EXPLAIN QUERY PLAN SELECT * FROM pages WHERE slug = 'test';
EOF
```

#### Connection Pool Testing
Monitor connection pool usage during load tests:
```rust
// Add to your benchmark code
println!("Pool size: {}", pool.size());
println!("Idle connections: {}", pool.num_idle());
```

### 4. Memory Profiling

```bash
# Install valgrind (Linux) or Instruments (macOS)
# Linux
valgrind --tool=massif --massif-out-file=massif.out ./target/release/doxyde-web

# Parse results
ms_print massif.out > memory-usage.txt

# macOS - Use Instruments
instruments -t "Memory Leak" ./target/release/doxyde-web
```

## Key Metrics to Monitor

### 1. Response Time
- **p50** (median): Should be < 50ms for most endpoints
- **p95**: Should be < 200ms
- **p99**: Should be < 500ms

### 2. Throughput
- **Requests/second**: Target > 1000 req/s for static pages
- **Concurrent connections**: Should handle > 1000 concurrent

### 3. Resource Usage
- **Memory**: Should stay stable under load
- **CPU**: Should scale linearly with load
- **Database connections**: Monitor pool exhaustion

### 4. Error Rates
- **HTTP errors**: < 0.1%
- **Timeouts**: < 0.01%
- **Database errors**: 0%

## Performance Optimization Tips

### 1. Database
- Use indexes on frequently queried columns
- Enable SQLite WAL mode for better concurrency
- Consider connection pool sizing

### 2. Web Server
- Enable compression for responses
- Use CDN for static assets
- Implement proper caching headers

### 3. Application
- Use `Arc` for shared immutable data
- Minimize allocations in hot paths
- Profile with `perf` or `flamegraph`

## Automated Performance Testing

### GitHub Actions Integration
```yaml
name: Performance Tests
on: [push, pull_request]

jobs:
  benchmark:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
      - name: Run benchmarks
        run: cargo bench
      - name: Run load test
        run: |
          cargo build --release
          ./target/release/doxyde-web &
          sleep 5
          wrk -t4 -c100 -d30s http://localhost:3000/
```

## Reporting Performance Issues

When reporting performance issues, include:

1. **Benchmark results** showing the regression
2. **System specifications** (CPU, RAM, OS)
3. **Doxyde version** and configuration
4. **Reproducible test case**

## Future Enhancements

- [ ] Add Criterion benchmarks for template rendering
- [ ] Implement distributed load testing
- [ ] Add real-time performance monitoring
- [ ] Create performance regression detection
- [ ] Add flame graph generation