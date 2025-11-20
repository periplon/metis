use async_trait::async_trait;
use serde_json::Value;

pub mod mcp_types;
pub mod sampling;

#[async_trait]
pub trait ResourcePort: Send + Sync {
    async fn get_resource(&self, uri: &str) -> anyhow::Result<Value>;
    async fn list_resources(&self) -> anyhow::Result<Vec<Value>>;
}

#[async_trait]
pub trait ToolPort: Send + Sync {
    async fn execute_tool(&self, name: &str, args: Value) -> anyhow::Result<Value>;
    async fn list_tools(&self) -> anyhow::Result<Vec<Value>>;
}

#[async_trait]
pub trait PromptPort: Send + Sync {
    async fn get_prompt(&self, name: &str, arguments: Option<Value>) -> anyhow::Result<Value>;
    async fn list_prompts(&self) -> anyhow::Result<Vec<Value>>;
}
