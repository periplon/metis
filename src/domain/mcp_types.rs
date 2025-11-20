use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct InitializeRequest {
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
    pub client_info: Implementation,
}

#[derive(Debug, Serialize)]
pub struct InitializeResult {
    pub protocol_version: String,
    pub capabilities: ServerCapabilities,
    pub server_info: Implementation,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Implementation {
    pub name: String,
    pub version: String,
}

#[derive(Debug, Deserialize)]
pub struct ClientCapabilities {
    pub roots: Option<Value>,
    pub sampling: Option<Value>,
    pub experimental: Option<HashMap<String, Value>>,
}

#[derive(Debug, Serialize)]
pub struct ServerCapabilities {
    pub resources: Option<Value>,
    pub tools: Option<Value>,
    pub prompts: Option<Value>,
    pub logging: Option<Value>,
    pub experimental: Option<HashMap<String, Value>>,
}
