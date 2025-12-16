use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::adapters::execution_context::ExecutionContext;

pub mod auth;
pub mod sampling;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

/// Resource template with URI pattern containing {placeholder} variables
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResourceTemplate {
    /// URI template pattern (e.g., "postgres://db/users/{id}")
    pub uri_template: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    /// Optional JSON Schema defining the expected output structure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<Value>,
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
    async fn list_resource_templates(&self) -> anyhow::Result<Vec<ResourceTemplate>>;
    /// Read a resource template by resolving the URI template with provided arguments
    async fn read_resource_template(
        &self,
        uri_template: &str,
        args: Option<&serde_json::Value>,
    ) -> anyhow::Result<ResourceReadResult>;
}

/// Port for tool execution operations.
///
/// Implementations handle tool invocation, including mock strategies,
/// agent calls, workflows, and MCP tools.
#[async_trait]
pub trait ToolPort: Send + Sync {
    /// Execute a tool by name with the given arguments.
    async fn execute_tool(&self, name: &str, args: Value) -> anyhow::Result<Value>;

    /// Execute a tool with an execution context for cross-invocation scenarios.
    ///
    /// The context provides:
    /// - **Recursion limits**: `call_depth` tracking to prevent infinite loops
    /// - **Access control**: Policy-based restrictions on which tools can be called
    /// - **Timeout management**: Per-call timeout budgets
    /// - **Session continuity**: Agent session ID propagation
    ///
    /// Default implementation delegates to `execute_tool`, ignoring the context.
    /// Override this method in handlers that need context-aware execution
    /// (e.g., for Python scripts calling other tools).
    async fn execute_tool_with_context(
        &self,
        name: &str,
        args: Value,
        _ctx: ExecutionContext,
    ) -> anyhow::Result<Value> {
        // Default: ignore context, delegate to standard execution
        self.execute_tool(name, args).await
    }

    /// List all available tools.
    async fn list_tools(&self) -> anyhow::Result<Vec<Tool>>;
}

#[async_trait]
pub trait PromptPort: Send + Sync {
    async fn get_prompt(&self, name: &str, arguments: Option<Value>) -> anyhow::Result<GetPromptResult>;
    async fn list_prompts(&self) -> anyhow::Result<Vec<Prompt>>;
}
