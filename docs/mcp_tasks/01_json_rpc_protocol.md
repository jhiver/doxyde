# Task 01: JSON-RPC Protocol Implementation

## Overview
The rmcp library handles most of the JSON-RPC protocol automatically through the ServerHandler trait. However, we need to understand how it maps the original protocol methods.

## Understanding rmcp's Protocol Handling

### What rmcp Handles Automatically

1. **initialize** - Handled by `ServerHandler::get_info()`
2. **tools/list** - Automatically generated from `#[tool]` methods
3. **tools/call** - Automatically routes to appropriate tool methods
4. **notifications/initialized** - Handled internally by rmcp

### What We Need to Add

1. **resources/list** - Custom implementation needed ?
2. **resources/read** - Custom implementation needed ?
3. **prompts/list** - Custom implementation needed (empty for now) ?

## Implementation Notes

### ServerHandler Implementation

The `get_info()` method we implemented in Task 00 already handles the initialize request:

```rust
impl ServerHandler for DoxydeRmcpService {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: rmcp::model::ProtocolVersion::default(),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()  // Enables resources/list and resources/read
                .enable_prompts()    // Enables prompts/list
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

### Protocol Version

The rmcp library uses its own protocol version (e.g., "2024-11-05"). The original code used "2024-06-18", but rmcp handles this correctly.

### Capability Differences

Original capabilities:
```json
{
    "tools": {},
    "resources": {
        "list": true,
        "read": true,
        "templates": {
            "list": false
        }
    },
    "prompts": {
        "list": true
    }
}
```

With rmcp:
- Tools capability is enabled with `.enable_tools()`
- Resources capability is enabled with `.enable_resources()`
- Prompts capability is enabled with `.enable_prompts()`
- Templates are not supported by rmcp (and were disabled in original anyway)

## Key Differences from Original Implementation

1. **No Manual JSON-RPC Parsing**: rmcp handles all JSON-RPC message parsing and response formatting
2. **No Method Routing**: rmcp automatically routes tools/list and tools/call
3. **Type Safety**: rmcp uses Rust types instead of raw JSON values
4. **Error Handling**: rmcp converts Result<T> and errors into proper JSON-RPC error responses

## Testing Protocol Compliance

To test that our implementation correctly handles protocol messages:

```rust
#[cfg(test)]
mod protocol_tests {
    use super::*;
    use rmcp::handler::server::ServerHandler;

    #[test]
    fn test_server_info_protocol_compliance() {
        let pool = // ... create test pool
        let service = DoxydeRmcpService::new(pool, 1);
        let info = service.get_info();

        // Verify protocol version is set
        assert!(!info.protocol_version.to_string().is_empty());

        // Verify capabilities
        assert!(info.capabilities.tools.is_some());
        assert!(info.capabilities.resources.is_some());
        assert!(info.capabilities.prompts.is_some());

        // Verify server info
        assert_eq!(info.server_info.name, "doxyde-mcp");
    }
}
```

## Next Steps

WRITE UNIT TESTS - MAKE SURE EVERYTHING IS STILL PASSING.

With the basic protocol handling in place via ServerHandler, we need to:
1. Implement resources support (Task 02)
2. Implement prompts support (Task 03)
3. Start adding the actual tools using `#[tool]` attribute