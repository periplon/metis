//! Agent domain types

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Agent type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    /// Single request → single response, no history
    #[default]
    SingleTurn,
    /// Maintains conversation history across interactions
    MultiTurn,
    /// Reasoning + Action loop with tool calling
    #[serde(rename = "react")]
    ReAct,
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentType::SingleTurn => write!(f, "single_turn"),
            AgentType::MultiTurn => write!(f, "multi_turn"),
            AgentType::ReAct => write!(f, "react"),
        }
    }
}

/// Agent information returned from list/get operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    /// Unique agent name
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Type of agent
    pub agent_type: AgentType,
    /// JSON Schema defining expected input
    pub input_schema: Value,
    /// Optional JSON Schema defining output structure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<Value>,
    /// Available tools (for ReAct agents)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub available_tools: Vec<String>,
    /// MCP tools from external servers (format: "server:tool")
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mcp_tools: Vec<String>,
    /// LLM provider being used
    pub llm_provider: String,
    /// LLM model being used
    pub llm_model: String,
}

/// Summary of a conversation session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    /// Unique session identifier
    pub session_id: String,
    /// Agent name this session belongs to
    pub agent_name: String,
    /// Number of messages in the session
    pub message_count: usize,
    /// Session creation timestamp (Unix epoch milliseconds)
    pub created_at: u64,
    /// Last update timestamp (Unix epoch milliseconds)
    pub updated_at: u64,
    /// Optional preview of last message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_message_preview: Option<String>,
}
