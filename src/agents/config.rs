//! Configuration types for AI Agents

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::domain::AgentType;

/// Configuration for an AI agent
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentConfig {
    /// Unique agent name
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Type of agent
    #[serde(default)]
    pub agent_type: AgentType,
    /// JSON Schema defining expected input
    #[serde(default = "default_input_schema")]
    pub input_schema: Value,
    /// Optional JSON Schema defining output structure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<Value>,
    /// LLM provider configuration
    pub llm: LlmProviderConfig,
    /// System prompt for the agent
    pub system_prompt: String,
    /// Prompt template for generating user messages from input schema values
    /// Uses Tera templating syntax (e.g., "Analyze {{topic}} for {{audience}}")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_template: Option<String>,
    /// Available tools (tool names from the tool registry)
    #[serde(default)]
    pub available_tools: Vec<String>,
    /// MCP tools from external MCP servers (format: "server_name:tool_name" or "server_name:*" for all)
    #[serde(default)]
    pub mcp_tools: Vec<String>,
    /// Other agents that can be called as tools (agent names, without "agent_" prefix)
    #[serde(default)]
    pub agent_tools: Vec<String>,
    /// Memory/persistence configuration
    #[serde(default)]
    pub memory: MemoryConfig,
    /// Maximum iterations for ReAct agents
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,
    /// Timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    /// Temperature override (if not set, uses LLM config default)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Max tokens override (if not set, uses LLM config default)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

fn default_input_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "prompt": {
                "type": "string",
                "description": "The input prompt for the agent"
            }
        },
        "required": ["prompt"]
    })
}

fn default_max_iterations() -> u32 {
    10
}

fn default_timeout() -> u64 {
    120
}

/// LLM provider configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LlmProviderConfig {
    /// Provider type
    pub provider: LlmProviderType,
    /// Model name/identifier
    pub model: String,
    /// Environment variable containing the API key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_env: Option<String>,
    /// Custom base URL (for self-hosted or proxied endpoints)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// Default temperature for completions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Default max tokens for completions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Whether to use streaming
    #[serde(default = "default_stream")]
    pub stream: bool,
}

fn default_stream() -> bool {
    true
}

/// Supported LLM providers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LlmProviderType {
    /// OpenAI (GPT-4, GPT-3.5, etc.)
    #[default]
    OpenAI,
    /// Anthropic (Sonnet, etc.)
    Anthropic,
    /// Google Gemini
    #[serde(alias = "google")]
    Gemini,
    /// Ollama (local models)
    Ollama,
    /// Azure OpenAI
    #[serde(alias = "azure")]
    AzureOpenAI,
}

impl std::fmt::Display for LlmProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LlmProviderType::OpenAI => write!(f, "openai"),
            LlmProviderType::Anthropic => write!(f, "anthropic"),
            LlmProviderType::Gemini => write!(f, "gemini"),
            LlmProviderType::Ollama => write!(f, "ollama"),
            LlmProviderType::AzureOpenAI => write!(f, "azure"),
        }
    }
}

/// Memory/persistence configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MemoryConfig {
    /// Storage backend type
    #[serde(default)]
    pub backend: MemoryBackend,
    /// Memory management strategy
    #[serde(default)]
    pub strategy: MemoryStrategy,
    /// Maximum number of messages to retain
    #[serde(default = "default_max_messages")]
    pub max_messages: u32,
    /// File path for file-based storage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    /// Database URL for database storage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database_url: Option<String>,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            backend: MemoryBackend::InMemory,
            strategy: MemoryStrategy::Full,
            max_messages: default_max_messages(),
            file_path: None,
            database_url: None,
        }
    }
}

fn default_max_messages() -> u32 {
    100
}

/// Memory storage backends
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MemoryBackend {
    /// Store in memory only (lost on restart)
    #[default]
    InMemory,
    /// Store in files
    File,
    /// Store in database
    Database,
}

/// Memory management strategies
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MemoryStrategy {
    /// Keep all messages (up to max_messages limit)
    Full,
    /// Sliding window of most recent messages
    SlidingWindow {
        /// Number of messages to keep
        size: usize,
    },
    /// Keep first N messages + last M messages
    FirstLast {
        /// Number of initial messages to keep
        first: usize,
        /// Number of recent messages to keep
        last: usize,
    },
}

impl Default for MemoryStrategy {
    fn default() -> Self {
        Self::Full
    }
}

/// Configuration for multi-agent orchestration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OrchestrationConfig {
    /// Unique orchestration name
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Orchestration pattern
    pub pattern: OrchestrationPattern,
    /// JSON Schema defining expected input
    #[serde(default = "default_input_schema")]
    pub input_schema: Value,
    /// Optional JSON Schema defining output structure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<Value>,
    /// Agents participating in the orchestration
    pub agents: Vec<AgentReference>,
    /// Manager agent (for hierarchical pattern)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manager_agent: Option<String>,
    /// Strategy for merging results (for collaborative pattern)
    #[serde(default)]
    pub merge_strategy: MergeStrategy,
    /// Timeout for the entire orchestration in seconds
    #[serde(default = "default_orchestration_timeout")]
    pub timeout_seconds: u64,
}

fn default_orchestration_timeout() -> u64 {
    300
}

/// Orchestration patterns
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OrchestrationPattern {
    /// Agents execute in sequence, passing results
    #[default]
    Sequential,
    /// Manager agent delegates to worker agents
    Hierarchical,
    /// Agents work in parallel, results merged
    Collaborative,
}

/// Reference to an agent in an orchestration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentReference {
    /// Agent name
    pub agent: String,
    /// Dependencies (agent names that must complete first)
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Condition for execution (Rhai expression)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// Input transformation (Rhai expression)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_transform: Option<String>,
}

/// Strategies for merging results in collaborative orchestration
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MergeStrategy {
    /// Concatenate all outputs
    #[default]
    Concat,
    /// Use majority voting (for classification tasks)
    Vote,
    /// Custom merge using Rhai script
    Custom {
        /// Rhai script for merging
        script: String,
    },
}
