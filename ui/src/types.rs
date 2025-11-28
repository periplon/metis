//! Shared types for the Metis Web UI
//!
//! These types mirror the backend API response structures.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Generic API response wrapper
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

/// Configuration overview
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConfigOverview {
    pub server: ServerInfo,
    pub resources_count: usize,
    #[serde(default)]
    pub resource_templates_count: usize,
    pub tools_count: usize,
    pub prompts_count: usize,
    pub workflows_count: usize,
    #[serde(default)]
    pub agents_count: usize,
    pub auth_enabled: bool,
    pub rate_limit_enabled: bool,
    pub s3_enabled: bool,
    pub config_file_loaded: bool,
    #[serde(default)]
    pub mcp_servers_count: usize,
    /// Version number for optimistic locking
    #[serde(default)]
    pub config_version: u64,
}

/// Request body for save operations with optimistic locking
#[derive(Debug, Clone, Serialize)]
pub struct SaveConfigRequest {
    /// Expected version for optimistic locking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_version: Option<u64>,
}

/// Response from save operations
#[derive(Debug, Clone, Deserialize)]
pub struct SaveConfigResponse {
    /// New version after save
    pub new_version: u64,
}

/// Error response for version conflicts
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct VersionConflictResponse {
    pub expected_version: u64,
    pub current_version: u64,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerInfo {
    pub host: String,
    pub port: u16,
    pub version: String,
}

/// Configuration for an external MCP server connection
#[allow(dead_code)]
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct McpServerConfig {
    pub name: String,
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_env: Option<String>,
    #[serde(default = "default_mcp_enabled")]
    pub enabled: bool,
    #[serde(default = "default_mcp_timeout")]
    pub timeout_seconds: u64,
}

#[allow(dead_code)]
fn default_mcp_enabled() -> bool {
    true
}

#[allow(dead_code)]
fn default_mcp_timeout() -> u64 {
    30
}

/// MCP tool info for display
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpToolInfo {
    pub server: String,
    pub name: String,
    pub description: Option<String>,
}

/// Editable authentication configuration
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AuthConfig {
    pub enabled: bool,
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_keys: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jwt_secret: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jwt_algorithm: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub basic_users: Option<std::collections::HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jwks_url: Option<String>,
}

/// Editable rate limit configuration
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RateLimitConfig {
    pub enabled: bool,
    pub requests_per_second: u32,
    pub burst_size: u32,
}

/// S3 configuration
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct S3Config {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bucket: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    pub poll_interval_secs: u64,
}

/// Server settings that can be edited via the UI
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerSettings {
    pub auth: AuthConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<RateLimitConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s3: Option<S3Config>,
}

/// Static resource with fixed URI (no input variables, only output schema)
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// JSON Schema for the expected output structure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mock: Option<MockConfig>,
}

/// Resource template with URI pattern containing {placeholder} variables
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ResourceTemplate {
    /// URI template pattern (e.g., "postgres://db/users/{id}")
    pub uri_template: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// JSON Schema for template input parameters (the {variables} in uri_template)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<Value>,
    /// JSON Schema for the expected output structure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mock: Option<MockConfig>,
}

/// Tool configuration
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub input_schema: Value,
    /// Optional JSON Schema defining the expected output structure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub static_response: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mock: Option<MockConfig>,
}

/// Prompt configuration
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Prompt {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<PromptArgument>>,
    /// JSON Schema for prompt input parameters (more detailed than arguments)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages: Option<Vec<PromptMessage>>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PromptArgument {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub required: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PromptMessage {
    pub role: String,
    pub content: String,
}

/// Workflow configuration
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Workflow {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub input_schema: Value,
    /// JSON Schema for the expected workflow output structure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<Value>,
    pub steps: Vec<WorkflowStep>,
    #[serde(default)]
    pub on_error: ErrorStrategy,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct WorkflowStep {
    pub id: String,
    pub tool: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub args: Option<Value>,
    /// Step IDs that must complete before this step can execute (DAG dependencies)
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loop_over: Option<String>,
    #[serde(default = "default_loop_var")]
    pub loop_var: String,
    #[serde(default = "default_concurrency")]
    pub loop_concurrency: u32,
    #[serde(default)]
    pub on_error: ErrorStrategy,
}

fn default_loop_var() -> String {
    "item".to_string()
}

fn default_concurrency() -> u32 {
    1
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorStrategy {
    #[default]
    Fail,
    Continue,
    Retry { max_attempts: u32, delay_ms: u64 },
    Fallback { value: Value },
}

/// Mock configuration
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct MockConfig {
    pub strategy: MockStrategyType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub faker_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stateful: Option<StatefulConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_lang: Option<ScriptLang>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file: Option<FileConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub llm: Option<LLMConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<DatabaseConfig>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MockStrategyType {
    #[default]
    Static,
    Template,
    Random,
    Stateful,
    Script,
    File,
    Pattern,
    #[serde(rename = "llm")]
    LLM,
    #[serde(rename = "database")]
    Database,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct StatefulConfig {
    pub state_key: String,
    pub operation: StateOperation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StateOperation {
    #[default]
    Get,
    Set,
    Increment,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ScriptLang {
    #[default]
    Rhai,
    Lua,
    Js,
    Python,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct FileConfig {
    pub path: String,
    #[serde(default = "default_selection")]
    pub selection: String,
}

fn default_selection() -> String {
    "random".to_string()
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct LLMConfig {
    pub provider: LLMProvider,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_env: Option<String>,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub stream: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LLMProvider {
    #[default]
    OpenAI,
    Anthropic,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub query: String,
    #[serde(default)]
    pub params: Vec<String>,
}

/// Request for testing tools, resources, prompts, workflows
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct TestRequest {
    #[serde(default)]
    pub args: Value,
    /// Optional session ID for multi-turn conversations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

/// Response from test endpoints
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TestResult {
    pub output: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

// ============================================================================
// AI Agent Types
// ============================================================================

/// AI Agent configuration
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Agent {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub agent_type: AgentType,
    #[serde(default)]
    pub input_schema: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<Value>,
    pub llm: AgentLlmConfig,
    pub system_prompt: String,
    /// Prompt template for generating user messages from input schema values
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_template: Option<String>,
    #[serde(default)]
    pub available_tools: Vec<String>,
    /// MCP tools from external servers (format: "server_name:tool_name" or "server_name:*")
    #[serde(default)]
    pub mcp_tools: Vec<String>,
    /// Other agents that can be called as tools (agent names, without "agent_" prefix)
    #[serde(default)]
    pub agent_tools: Vec<String>,
    /// Available resources (resource URIs the agent can access)
    #[serde(default)]
    pub available_resources: Vec<String>,
    /// Available resource templates (template URIs the agent can access)
    #[serde(default)]
    pub available_resource_templates: Vec<String>,
    #[serde(default)]
    pub memory: MemoryConfig,
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

fn default_max_iterations() -> u32 {
    10
}

fn default_timeout() -> u64 {
    120
}

/// Agent type
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    #[default]
    SingleTurn,
    MultiTurn,
    #[serde(rename = "react")]
    ReAct,
}

/// LLM provider configuration for agents
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AgentLlmConfig {
    pub provider: AgentLlmProvider,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_env: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(default = "default_stream")]
    pub stream: bool,
}

fn default_stream() -> bool {
    true
}

/// Supported LLM providers for agents
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AgentLlmProvider {
    #[default]
    OpenAI,
    Anthropic,
    Gemini,
    Ollama,
    #[serde(rename = "azureopenai")]
    AzureOpenAI,
}

/// Memory configuration for agents
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct MemoryConfig {
    #[serde(default)]
    pub backend: MemoryBackend,
    #[serde(default)]
    pub strategy: MemoryStrategy,
    #[serde(default = "default_max_messages")]
    pub max_messages: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database_url: Option<String>,
}

fn default_max_messages() -> u32 {
    100
}

/// Memory storage backend
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryBackend {
    #[default]
    InMemory,
    File,
    Database,
}

/// Memory management strategy
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MemoryStrategy {
    #[default]
    Full,
    SlidingWindow { size: usize },
    FirstLast { first: usize, last: usize },
}

// ============================================================================
// Orchestration Types
// ============================================================================

/// Multi-agent orchestration configuration
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Orchestration {
    pub name: String,
    pub description: String,
    pub pattern: OrchestrationPattern,
    #[serde(default)]
    pub input_schema: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<Value>,
    pub agents: Vec<AgentReference>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manager_agent: Option<String>,
    #[serde(default)]
    pub merge_strategy: MergeStrategy,
    #[serde(default = "default_orchestration_timeout")]
    pub timeout_seconds: u64,
}

fn default_orchestration_timeout() -> u64 {
    300
}

/// Orchestration patterns
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OrchestrationPattern {
    #[default]
    Sequential,
    Hierarchical,
    Collaborative,
}

/// Reference to an agent in an orchestration
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct AgentReference {
    pub agent: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_transform: Option<String>,
}

/// Strategies for merging results in collaborative orchestration
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MergeStrategy {
    #[default]
    Concat,
    Vote,
    Custom { script: String },
}

/// Reusable JSON Schema definition
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Schema {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub schema: Value,
}
