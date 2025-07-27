# Task 00: RMCP Infrastructure Setup

## Overview
Set up the basic rmcp service structure with database pool and site_id support. This is the foundation for all other MCP tools.

## Implementation Steps

### 1. Update `doxyde-shared/src/mcp/service.rs`

Replace the current simple implementation with:

```rust
use rmcp::{
    handler::server::{router::tool::ToolRouter, tool::Parameters, ServerHandler},
    model::{ServerCapabilities, ServerInfo},
    schemars::{self, JsonSchema},
    tool, tool_router,
};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use tracing::{debug, info};
use anyhow::Result;
use serde_json::json;

#[derive(Debug, Clone)]
pub struct DoxydeRmcpService {
    pool: SqlitePool,
    site_id: i64,
    tool_router: ToolRouter<Self>,
}

impl DoxydeRmcpService {
    pub fn new(pool: SqlitePool, site_id: i64) -> Self {
        Self {
            pool,
            site_id,
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl DoxydeRmcpService {
    // Tools will be added here in subsequent tasks
}

impl ServerHandler for DoxydeRmcpService {
    fn get_info(&self) -> ServerInfo {
        info!("Getting server info");
        ServerInfo {
            protocol_version: rmcp::model::ProtocolVersion::default(),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_prompts()
                .build(),
            server_info: rmcp::model::Implementation {
                name: "doxyde-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some("Doxyde CMS MCP integration for AI-native content management".to_string()),
        }
    }
}
```

### 2. Add Helper Structs

Add these data structures from the original implementation:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageInfo {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub path: String,
    pub parent_id: Option<i64>,
    pub position: i32,
    pub has_children: bool,
    pub template: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PageHierarchy {
    pub page: PageInfo,
    pub children: Vec<PageHierarchy>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ComponentInfo {
    pub id: i64,
    pub component_type: String,
    pub position: i32,
    pub template: String,
    pub title: Option<String>,
    pub content: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DraftInfo {
    pub page_id: i64,
    pub version_id: i64,
    pub version_number: i32,
    pub created_by: Option<String>,
    pub is_published: bool,
    pub component_count: i32,
}
```

### 3. Update Both SSE and HTTP Servers

Since both doxyde-sse and doxyde-web need the same MCP functionality, we'll use the shared service in both places.

#### Update SSE Server (`doxyde-sse/src/main.rs`)

```rust
// In the SSE handler function
let service = DoxydeRmcpService::new(pool.clone(), site_id);
let server = SseServer::new(service, config);
```

#### Update HTTP Server (`doxyde-web/src/rmcp/handlers.rs`)

```rust
use doxyde_shared::mcp::DoxydeRmcpService;
use rmcp::transport::http::HttpServer;

pub async fn handle_http(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: String,
) -> Result<impl IntoResponse, AppError> {
    // Extract site_id from token validation (existing code)
    let site_id = // ... existing token validation logic

    // Create the same service instance
    let service = DoxydeRmcpService::new(state.db.clone(), site_id);
    let server = HttpServer::new(service);

    // Handle the request
    let response = server.handle(body).await?;

    Ok(Json(response))
}
```

This ensures that any tool we add to `DoxydeRmcpService` is automatically available through both SSE and HTTP transports without any additional code.

### 4. Update Dependencies

Ensure `doxyde-shared/Cargo.toml` has:

```toml
[dependencies]
rmcp = { version = "0.1", features = ["server"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "chrono"] }
anyhow = "1.0"
tracing = "0.1"
doxyde-core = { path = "../doxyde-core" }
doxyde-db = { path = "../doxyde-db" }
chrono = { version = "0.4", features = ["serde"] }
```

## Notes

- The `tool_router` attribute macro automatically generates the router from methods marked with `#[tool]`
- The service struct holds the database pool and site_id for all tools to use
- ServerCapabilities enables tools, resources, and prompts support
- The protocol version uses rmcp's default (typically "2024-11-05" or similar)
- **Key Architecture Benefit**: By implementing everything in `doxyde-shared`, both HTTP and SSE transports automatically get all MCP functionality without duplication

## Testing

Create a test to verify the service can be instantiated:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::SqlitePool;

    #[sqlx::test]
    async fn test_service_creation(pool: SqlitePool) -> Result<()> {
        let service = DoxydeRmcpService::new(pool, 1);
        let info = service.get_info();
        assert_eq!(info.server_info.name, "doxyde-mcp");
        Ok(())
    }
}
```

## Next Steps

WRITE UNIT TESTS - MAKE SURE EVERYTHING IS STILL PASSING.

After this infrastructure is in place, we can start adding the actual MCP tools using the `#[tool]` attribute.