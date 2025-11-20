use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod mcp_types;
pub mod sampling;
pub mod auth;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Prompt {
    pub name: String,
    pub description: String,
    pub arguments: Option<Vec<PromptArgument>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PromptArgument {
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetPromptResult {
    pub description: Option<String>,
    pub messages: Vec<PromptMessage>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PromptMessage {
    pub role: String,
    pub content: PromptContent,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PromptContent {
    #[serde(rename = "type")]
    pub type_: String,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResourceReadResult {
    pub uri: String,
    pub mime_type: Option<String>,
    pub content: String,
}

#[async_trait]
pub trait ResourcePort: Send + Sync {
    async fn get_resource(&self, uri: &str) -> anyhow::Result<ResourceReadResult>;
    async fn list_resources(&self) -> anyhow::Result<Vec<Resource>>;
}

#[async_trait]
pub trait ToolPort: Send + Sync {
    async fn execute_tool(&self, name: &str, args: Value) -> anyhow::Result<Value>;
    async fn list_tools(&self) -> anyhow::Result<Vec<Tool>>;
}

#[async_trait]
pub trait PromptPort: Send + Sync {
    async fn get_prompt(&self, name: &str, arguments: Option<Value>) -> anyhow::Result<GetPromptResult>;
    async fn list_prompts(&self) -> anyhow::Result<Vec<Prompt>>;
}
