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
    #[serde(default)]
    pub schemas_count: usize,
    #[serde(default)]
    pub data_lakes_count: usize,
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

/// Database persistence configuration
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DatabaseConfig {
    /// Database connection URL (sqlite://, postgres://, mysql://)
    pub url: String,
    /// Maximum number of connections in the pool
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    /// Whether to run migrations automatically on startup
    #[serde(default = "default_auto_migrate")]
    pub auto_migrate: bool,
    /// Whether to seed the database from config files on startup
    #[serde(default)]
    pub seed_on_startup: bool,
    /// Create a full snapshot every N commits
    #[serde(default = "default_snapshot_interval")]
    pub snapshot_interval: u32,
}

fn default_max_connections() -> u32 { 5 }
fn default_auto_migrate() -> bool { true }
fn default_snapshot_interval() -> u32 { 10 }

/// Server settings that can be edited via the UI
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerSettings {
    pub auth: AuthConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<RateLimitConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s3: Option<S3Config>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<DatabaseConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_storage: Option<FileStorageConfig>,
}

/// Static resource with fixed URI (no input variables, only output schema)
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Resource {
    pub uri: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Tags for categorization and filtering
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
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
    /// Tags for categorization and filtering
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
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
    /// Tags for categorization and filtering
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
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
    /// Tags for categorization and filtering
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
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
    /// Tags for categorization and filtering
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
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
    pub database: Option<MockDatabaseConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_lake_crud: Option<DataLakeCrudConfig>,
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
    #[serde(rename = "data_lake_crud")]
    DataLakeCrud,
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
    /// Base URL for API endpoint (required for Ollama and AzureOpenAI)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LLMProvider {
    #[default]
    OpenAI,
    Anthropic,
    /// Google Gemini
    Gemini,
    /// Ollama (local models)
    Ollama,
    /// Azure OpenAI
    #[serde(rename = "azureopenai")]
    AzureOpenAI,
}

/// Database type for mock strategy
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseType {
    #[default]
    Sqlite,
    Postgres,
    Mysql,
    DataFusion,
}

impl std::fmt::Display for DatabaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DatabaseType::Sqlite => write!(f, "sqlite"),
            DatabaseType::Postgres => write!(f, "postgres"),
            DatabaseType::Mysql => write!(f, "mysql"),
            DatabaseType::DataFusion => write!(f, "datafusion"),
        }
    }
}

/// DataFusion-specific configuration for querying datalakes
/// Storage settings are inherited from the data lake configuration
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct DataFusionConfig {
    /// Data lake name to query (storage settings are inherited from data lake)
    pub data_lake: String,
    /// Schema name within the data lake
    pub schema_name: String,
}

/// Database mock configuration for mock strategy
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct MockDatabaseConfig {
    /// Database URL (for sqlite, postgres, mysql) or empty for datafusion
    pub url: String,
    /// SQL query to execute
    pub query: String,
    /// Parameter names to bind from input arguments
    #[serde(default)]
    pub params: Vec<String>,
    /// Database type (determines how to connect)
    #[serde(default)]
    pub db_type: DatabaseType,
    /// DataFusion-specific configuration (when db_type is DataFusion)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datafusion: Option<DataFusionConfig>,
}

/// CRUD operation type for DataLakeCrud mock strategy
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DataLakeCrudOperation {
    /// Create a new record - input: schema fields, output: created record
    #[default]
    Create,
    /// Read a single record by ID - input: {id: string}, output: record or null
    ReadById,
    /// List all records - input: optional {limit, offset}, output: array of records
    ReadAll,
    /// Filter records - input: filter fields from template, output: array of records
    ReadFilter,
    /// Update an existing record - input: {id: string, ...fields}, output: updated record
    Update,
    /// Delete a record - input: {id: string}, output: {success: bool}
    Delete,
}

fn default_id_field() -> String {
    "id".to_string()
}

fn default_read_limit() -> usize {
    100
}

/// Configuration for DataLakeCrud mock strategy
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DataLakeCrudConfig {
    /// Name of the data lake to operate on (required)
    pub data_lake: String,
    /// Schema name within the data lake (required)
    pub schema_name: String,
    /// CRUD operation to perform (required)
    pub operation: DataLakeCrudOperation,
    /// For ReadFilter: Tera template to build filter conditions from input
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter_template: Option<String>,
    /// For ReadById/Update/Delete: which input field contains the record ID
    #[serde(default = "default_id_field")]
    pub id_field: String,
    /// For ReadAll/ReadFilter: maximum number of records to return
    #[serde(default = "default_read_limit")]
    pub read_limit: usize,
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
    /// Tags for categorization and filtering
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
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
    /// Tags for categorization and filtering
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
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
    /// Tags for categorization and filtering
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub schema: Value,
}

// ============================================================================
// Database & Version History Types
// ============================================================================

/// Database status information
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DatabaseStatus {
    /// Whether database persistence is enabled
    pub enabled: bool,
    /// Database backend type (SQLite, PostgreSQL, MySQL)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend: Option<String>,
    /// Whether the database connection is healthy
    pub healthy: bool,
    /// Current HEAD commit (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head: Option<Commit>,
    /// Total number of commits
    pub total_commits: usize,
    /// Total number of tags
    pub total_tags: usize,
}

/// Version history commit
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Commit {
    pub id: String,
    pub commit_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_hash: Option<String>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    pub committed_at: String,
    pub is_snapshot: bool,
    pub changes_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
}

/// Changeset (individual change within a commit)
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Changeset {
    pub operation: String,
    pub archetype_type: String,
    pub archetype_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_definition: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_definition: Option<Value>,
}

/// Tag pointing to a commit
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Tag {
    pub name: String,
    pub commit_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    pub created_at: String,
}

/// Request for creating a tag
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateTagRequest {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Request for rollback
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RollbackRequest {
    pub commit_hash: String,
}

// ============================================================================
// Data Lake Types
// ============================================================================

/// Storage mode for data lakes
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DataLakeStorageMode {
    #[default]
    Database,
    File,
    Both,
}

/// File format for data lake storage
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DataLakeFileFormat {
    #[default]
    Parquet,
    Jsonl,
}

/// Data Lake configuration (data model + records)
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct DataLake {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Tags for categorization and filtering
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default)]
    pub schemas: Vec<DataLakeSchemaRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
    /// Storage mode: database, file, or both
    #[serde(default)]
    pub storage_mode: DataLakeStorageMode,
    /// File format: parquet or jsonl
    #[serde(default)]
    pub file_format: DataLakeFileFormat,
    /// Enable SQL queries via DataFusion
    #[serde(default)]
    pub enable_sql_queries: bool,
}

/// Reference to a schema within a data lake
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct DataLakeSchemaRef {
    pub schema_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
}

/// Data record stored in a data lake
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DataRecord {
    pub id: String,
    pub data_lake: String,
    pub schema_name: String,
    pub data: Value,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

/// Request to create a new record
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateRecordRequest {
    pub schema_name: String,
    pub data: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

/// Request to update a record
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UpdateRecordRequest {
    pub data: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
}

/// Request to generate records using a mock strategy
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GenerateRecordsRequest {
    pub schema_name: String,
    pub count: usize,
    pub strategy: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy_config: Option<Value>,
}

/// Request to execute SQL query via DataFusion
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SqlQueryRequest {
    /// SQL query to execute (use $table as placeholder for table name)
    pub sql: String,
    /// Schema to query (required)
    pub schema_name: String,
}

/// Response from a SQL query
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SqlQueryResponse {
    pub columns: Vec<String>,
    pub rows: Vec<Value>,
    pub total_rows: usize,
}

/// Request to sync records to file storage
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SyncRequest {
    /// Optional: sync only this schema (if omitted, sync all schemas)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_name: Option<String>,
    /// Format to use for sync (overrides data lake default)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<DataLakeFileFormat>,
}

/// Response from sync operation
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SyncResponse {
    pub files_written: usize,
    pub records_synced: usize,
    pub paths: Vec<String>,
}

/// File info for data lake files
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct FileInfo {
    pub path: String,
    pub size_bytes: usize,
    pub last_modified: String,
    pub format: String,
}

// ============================================================================
// File Storage Configuration Types
// ============================================================================

/// Global file storage configuration for data lakes
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct FileStorageConfig {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s3: Option<S3DataConfig>,
    #[serde(default)]
    pub default_format: DataLakeFileFormat,
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    #[serde(default = "default_max_file_size")]
    pub max_file_size_bytes: usize,
}

fn default_batch_size() -> usize {
    1000
}

fn default_max_file_size() -> usize {
    128 * 1024 * 1024 // 128MB
}

/// S3 configuration for file storage (separate from S3Config for config sync)
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct S3DataConfig {
    pub bucket: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_key_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_access_key: Option<String>,
    #[serde(default)]
    pub force_path_style: bool,
    #[serde(default)]
    pub allow_http: bool,
}
