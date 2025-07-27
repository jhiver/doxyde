// Doxyde - A modern, AI-native CMS built with Rust
// Copyright (C) 2025 Doxyde Project Contributors
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use rmcp::{
    handler::server::{router::tool::ToolRouter, tool::Parameters, ServerHandler},
    model::{ServerCapabilities, ServerInfo},
    schemars::{self, JsonSchema},
    tool, tool_router,
};
use serde::Deserialize;
use tracing::{debug, info};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TimeRequest {
    #[schemars(description = "Timezone name (e.g., 'America/New_York', 'UTC')")]
    pub timezone: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DoxydeRmcpService {
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
}

impl DoxydeRmcpService {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl DoxydeRmcpService {
    #[tool(description = "Get the current time in a specified timezone")]
    pub fn time(&self, Parameters(req): Parameters<TimeRequest>) -> String {
        let timezone = req.timezone.unwrap_or_else(|| "UTC".to_string());
        debug!("Getting time for timezone: {}", timezone);
        
        let now = chrono::Utc::now();
        let formatted = match timezone.as_str() {
            "UTC" => now.to_rfc3339(),
            "America/New_York" => {
                use chrono_tz::US::Eastern;
                now.with_timezone(&Eastern).to_rfc3339()
            }
            "Europe/London" => {
                use chrono_tz::Europe::London;
                now.with_timezone(&London).to_rfc3339()
            }
            "Asia/Tokyo" => {
                use chrono_tz::Asia::Tokyo;
                now.with_timezone(&Tokyo).to_rfc3339()
            }
            _ => {
                return format!("Error: Unknown timezone: {}", timezone);
            }
        };
        
        format!("{{\"time\": \"{}\", \"timezone\": \"{}\"}}", formatted, timezone)
    }
}

impl ServerHandler for DoxydeRmcpService {
    fn get_info(&self) -> ServerInfo {
        info!("Getting server info");
        ServerInfo {
            protocol_version: rmcp::model::ProtocolVersion::default(),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .build(),
            server_info: rmcp::model::Implementation {
                name: "Doxyde MCP Service".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
            instructions: Some("Doxyde CMS MCP integration for AI-native content management".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_service() {
        let _service = DoxydeRmcpService::new();
        // Just verify it can be created
        assert!(true);
    }

    #[test]
    fn test_time_utc() {
        let service = DoxydeRmcpService::new();
        let result = service.time(Parameters(TimeRequest { timezone: None }));
        assert!(result.contains("UTC"));
        assert!(result.contains("time"));
        assert!(result.contains("timezone"));
    }

    #[test]
    fn test_time_new_york() {
        let service = DoxydeRmcpService::new();
        let result = service.time(Parameters(TimeRequest {
            timezone: Some("America/New_York".to_string()),
        }));
        assert!(result.contains("America/New_York"));
        assert!(result.contains("time"));
    }

    #[test]
    fn test_time_london() {
        let service = DoxydeRmcpService::new();
        let result = service.time(Parameters(TimeRequest {
            timezone: Some("Europe/London".to_string()),
        }));
        assert!(result.contains("Europe/London"));
        assert!(result.contains("time"));
    }

    #[test]
    fn test_time_tokyo() {
        let service = DoxydeRmcpService::new();
        let result = service.time(Parameters(TimeRequest {
            timezone: Some("Asia/Tokyo".to_string()),
        }));
        assert!(result.contains("Asia/Tokyo"));
        assert!(result.contains("time"));
    }

    #[test]
    fn test_time_unknown_timezone() {
        let service = DoxydeRmcpService::new();
        let result = service.time(Parameters(TimeRequest {
            timezone: Some("Unknown/Timezone".to_string()),
        }));
        assert!(result.contains("Error: Unknown timezone"));
    }

    #[test]
    fn test_get_info() {
        let service = DoxydeRmcpService::new();
        let info = service.get_info();
        assert_eq!(info.server_info.name, "Doxyde MCP Service");
        assert!(info.instructions.is_some());
        assert!(info.instructions.unwrap().contains("Doxyde CMS"));
    }
}