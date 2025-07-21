# DOXYDE_RULES

## CRITICAL_CONSTRAINTS
- LANG=EN_ONLY
- FUNC_MAX_LINES=30
- FUNC_MAX_PARAMS=4
- NEST_MAX_DEPTH=3
- NO_UNWRAP_EXPECT
- ONE_FUNC_THEN_TEST

## SQL_QUERY_CHANGES
```bash
export SQLX_OFFLINE=false DATABASE_URL="sqlite:doxyde.db"
./target/debug/doxyde init || (sqlx database create && sqlx migrate run --source migrations)
cargo sqlx prepare --workspace
git add .sqlx/ && git commit -m "Update sqlx offline query cache"
unset SQLX_OFFLINE
```

## ERROR_HANDLING
```rust
// YES: .context()/.with_context()/.ok_or_else()
// NO: .unwrap()/.expect()
```

## TEST_PATTERN
```rust
#[sqlx::test] // for db tests
async fn test_name(pool: SqlitePool) -> Result<()> {
    // test happy/edge/error cases
}
```

## PRE_COMMIT
```bash
cargo fmt --all && cargo clippy --all-targets --all-features && cargo test --all
```

## ARCHITECTURE
- URLs: content=`/path`, actions=`/.action` or `/path/.action`
- Draft workflow: get_or_create_draft→modify→publish_draft
- MCP: JSON-RPC only, errors={-32602:invalid_params,-32603:internal}
- Migrations: sequential 001-011 in /migrations/

## TYPE_HINTS
- DateTime: `as "field: chrono::DateTime<chrono::Utc>"`
- Optional: `as "field: Option<T>"`
- i64 for IDs, i32 for counts/positions

## CURRENT_STATE
- MVP_COMPLETE=true
- TESTS=420+
- OFFLINE_MODE=default_enabled(.cargo/config.toml)