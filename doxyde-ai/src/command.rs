use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    pub action: String,
    pub parameters: serde_json::Value,
}
