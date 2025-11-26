//! Tool call types for agent interactions

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A tool call made by an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique identifier for this tool call
    pub id: String,
    /// Name of the tool being called
    pub name: String,
    /// Arguments passed to the tool (as JSON)
    pub arguments: Value,
}

impl ToolCall {
    /// Create a new tool call
    pub fn new(id: impl Into<String>, name: impl Into<String>, arguments: Value) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            arguments,
        }
    }

    /// Generate a unique ID for a tool call
    pub fn generate_id() -> String {
        format!("call_{}", uuid::Uuid::new_v4().to_string().replace("-", "")[..24].to_string())
    }
}

/// Result of executing a tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    /// ID of the tool call this is responding to
    pub tool_call_id: String,
    /// Name of the tool that was called
    pub tool_name: String,
    /// Input arguments that were passed
    pub input: Value,
    /// Output returned by the tool
    pub output: Value,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Whether the tool execution succeeded
    pub success: bool,
    /// Error message if execution failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ToolCallResult {
    /// Create a successful tool call result
    pub fn success(
        tool_call_id: String,
        tool_name: String,
        input: Value,
        output: Value,
        execution_time_ms: u64,
    ) -> Self {
        Self {
            tool_call_id,
            tool_name,
            input,
            output,
            execution_time_ms,
            success: true,
            error: None,
        }
    }

    /// Create a failed tool call result
    pub fn failure(
        tool_call_id: String,
        tool_name: String,
        input: Value,
        error: String,
        execution_time_ms: u64,
    ) -> Self {
        Self {
            tool_call_id,
            tool_name,
            input,
            output: Value::Null,
            execution_time_ms,
            success: false,
            error: Some(error),
        }
    }
}

/// Definition of a tool available to an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Tool name
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// JSON Schema defining the tool's parameters
    pub parameters: Value,
}

impl ToolDefinition {
    /// Create a new tool definition
    pub fn new(name: impl Into<String>, description: impl Into<String>, parameters: Value) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters,
        }
    }
}
