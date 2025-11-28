use config::{Config, File};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod s3;
pub mod s3_watcher;
pub mod validator;
pub mod watcher;

pub use s3::S3Config;
pub use s3_watcher::S3Watcher;

use crate::agents::config::{AgentConfig, OrchestrationConfig};
use crate::cli::Cli;

#[derive(Debug, Deserialize, Serialize)]
pub struct Settings {
    pub server: ServerSettings,
    #[serde(default)]
    pub auth: crate::domain::auth::AuthConfig,
    #[serde(default)]
    pub resources: Vec<ResourceConfig>,
    #[serde(default)]
    pub resource_templates: Vec<ResourceTemplateConfig>,
    #[serde(default)]
    pub tools: Vec<ToolConfig>,
    #[serde(default)]
    pub prompts: Vec<PromptConfig>,
    #[serde(default)]
    pub workflows: Vec<WorkflowConfig>,
    #[serde(default)]
    pub agents: Vec<AgentConfig>,
    #[serde(default)]
    pub orchestrations: Vec<OrchestrationConfig>,
    #[serde(default)]
    pub rate_limit: Option<RateLimitConfig>,
    #[serde(default)]
    pub s3: Option<S3Config>,
    /// External MCP servers that can be connected to for tools
    #[serde(default)]
    pub mcp_servers: Vec<McpServerConfig>,
    /// Encrypted secrets that can be stored in the config file
    /// Values can be plain text or AGE-encrypted (prefixed with "age:")
    /// Encrypted values require METIS_SECRET_PASSPHRASE env var or --secret-passphrase flag
    #[serde(default)]
    pub secrets: SecretsConfig,
}

/// Configuration for embedded secrets (can be encrypted with AGE)
#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct SecretsConfig {
    /// OpenAI API key (plain or AGE-encrypted)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub openai_api_key: Option<String>,
    /// Anthropic API key (plain or AGE-encrypted)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anthropic_api_key: Option<String>,
    /// Gemini API key (plain or AGE-encrypted)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gemini_api_key: Option<String>,
    /// AWS Access Key ID (plain or AGE-encrypted)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aws_access_key_id: Option<String>,
    /// AWS Secret Access Key (plain or AGE-encrypted)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aws_secret_access_key: Option<String>,
    /// AWS Region
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aws_region: Option<String>,
}

impl SecretsConfig {
    /// Merge another SecretsConfig into this one.
    /// Other's values override self's values if present (Some).
    pub fn merge(&mut self, other: &SecretsConfig) {
        if other.openai_api_key.is_some() {
            self.openai_api_key = other.openai_api_key.clone();
        }
        if other.anthropic_api_key.is_some() {
            self.anthropic_api_key = other.anthropic_api_key.clone();
        }
        if other.gemini_api_key.is_some() {
            self.gemini_api_key = other.gemini_api_key.clone();
        }
        if other.aws_access_key_id.is_some() {
            self.aws_access_key_id = other.aws_access_key_id.clone();
        }
        if other.aws_secret_access_key.is_some() {
            self.aws_secret_access_key = other.aws_secret_access_key.clone();
        }
        if other.aws_region.is_some() {
            self.aws_region = other.aws_region.clone();
        }
    }
}

/// Configuration for connecting to an external MCP server
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct McpServerConfig {
    /// Unique name for this MCP server connection
    pub name: String,
    /// URL of the MCP server (e.g., "http://localhost:3001/mcp")
    pub url: String,
    /// Optional API key for authentication
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Environment variable containing the API key
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_env: Option<String>,
    /// Whether this server is enabled
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Connection timeout in seconds
    #[serde(default = "default_mcp_timeout")]
    pub timeout_seconds: u64,
}

fn default_enabled() -> bool {
    true
}

fn default_mcp_timeout() -> u64 {
    30
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RateLimitConfig {
    pub enabled: bool,
    pub requests_per_second: u32,
    pub burst_size: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ServerSettings {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ResourceConfig {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
    /// JSON Schema for the expected output structure
    #[serde(default)]
    pub output_schema: Option<Value>,
    pub content: Option<String>, // Simple static content for now
    pub mock: Option<MockConfig>,
}

/// Configuration for a resource template with URI pattern variables
/// Resource templates use URI patterns with {placeholder} syntax
/// e.g., "postgres://db/users/{id}" or "file:///home/{username}/{filename}"
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ResourceTemplateConfig {
    /// URI template pattern with {variable} placeholders
    pub uri_template: String,
    pub name: String,
    pub description: Option<String>,
    pub mime_type: Option<String>,
    /// JSON Schema for template input parameters (the {variables} in uri_template)
    #[serde(default)]
    pub input_schema: Option<Value>,
    /// JSON Schema for the expected output structure
    #[serde(default)]
    pub output_schema: Option<Value>,
    pub content: Option<String>,
    pub mock: Option<MockConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ToolConfig {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
    /// Optional JSON Schema defining the expected output structure
    #[serde(default)]
    pub output_schema: Option<Value>,
    pub static_response: Option<Value>, // Simple static response for now
    pub mock: Option<MockConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MockStrategyType {
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MockConfig {
    pub strategy: MockStrategyType,
    pub template: Option<String>,
    pub faker_type: Option<String>,
    pub stateful: Option<StatefulConfig>,
    pub script: Option<String>,
    #[serde(default)]
    pub script_lang: Option<ScriptLang>,
    pub file: Option<FileConfig>,
    pub pattern: Option<String>,
    pub llm: Option<LLMConfig>,
    pub database: Option<DatabaseConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ScriptLang {
    Rhai,
    Lua,
    Js,
    Python,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub query: String,
    pub params: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LLMConfig {
    pub provider: LLMProvider,
    #[serde(default)]
    pub api_key_env: Option<String>,
    pub model: String,
    pub system_prompt: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub stream: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LLMProvider {
    OpenAI,
    Anthropic,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileConfig {
    pub path: String,
    #[serde(default = "default_selection")]
    pub selection: String, // "random", "sequential", "weighted"
}

fn default_selection() -> String {
    "random".to_string()
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StatefulConfig {
    pub state_key: String,
    pub operation: StateOperation,
    pub template: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StateOperation {
    Get,
    Set,
    Increment,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PromptConfig {
    pub name: String,
    pub description: String,
    pub arguments: Option<Vec<PromptArgument>>,
    /// JSON Schema for prompt input parameters (more detailed than arguments)
    #[serde(default)]
    pub input_schema: Option<Value>,
    pub messages: Option<Vec<PromptMessage>>, // Static messages for now
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PromptArgument {
    pub name: String,
    pub description: Option<String>,
    pub required: bool,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PromptMessage {
    pub role: String,
    pub content: String,
}

// ============================================================================
// Workflow Configuration Types
// ============================================================================

/// Configuration for a workflow that can be executed as a tool
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct WorkflowConfig {
    /// Unique name for the workflow (becomes the tool name)
    pub name: String,
    /// Description of what the workflow does
    pub description: String,
    /// JSON schema for workflow inputs
    #[serde(default = "default_workflow_schema")]
    pub input_schema: Value,
    /// JSON Schema for the expected workflow output structure
    #[serde(default)]
    pub output_schema: Option<Value>,
    /// Ordered list of steps to execute
    pub steps: Vec<WorkflowStep>,
    /// Default error handling strategy for the workflow
    #[serde(default)]
    pub on_error: ErrorStrategy,
}

fn default_workflow_schema() -> Value {
    serde_json::json!({
        "type": "object",
        "properties": {}
    })
}

/// A single step in a workflow
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct WorkflowStep {
    /// Unique identifier for this step (used for referencing results)
    pub id: String,
    /// Name of the tool to call
    pub tool: String,
    /// Arguments to pass to the tool (supports Tera templates)
    #[serde(default)]
    pub args: Option<Value>,
    /// Step IDs that must complete before this step can execute (DAG dependencies)
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Rhai expression that must evaluate to true for step to execute
    #[serde(default)]
    pub condition: Option<String>,
    /// Rhai expression returning an array to iterate over
    #[serde(default)]
    pub loop_over: Option<String>,
    /// Variable name for current loop item (default: "item")
    #[serde(default = "default_loop_var")]
    pub loop_var: String,
    /// Maximum concurrent loop iterations (default: 1 = sequential)
    #[serde(default = "default_concurrency")]
    pub loop_concurrency: u32,
    /// Error handling strategy for this step
    #[serde(default)]
    pub on_error: ErrorStrategy,
}

fn default_loop_var() -> String {
    "item".to_string()
}

fn default_concurrency() -> u32 {
    1
}

/// Strategy for handling errors in workflow execution
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
#[serde(rename_all = "snake_case")]
pub enum ErrorStrategy {
    /// Stop workflow execution and return error
    #[default]
    Fail,
    /// Log error and continue to next step
    Continue,
    /// Retry the step with exponential backoff
    Retry {
        /// Maximum number of retry attempts
        max_attempts: u32,
        /// Initial delay in milliseconds
        delay_ms: u64,
    },
    /// Use a fallback value on error
    Fallback {
        /// Value to use when step fails
        value: Value,
    },
}

impl Settings {
    pub fn new() -> Result<Self, anyhow::Error> {
        Self::from_root(".")
    }

    /// Create settings from CLI arguments (includes config file and CLI overrides)
    pub fn new_with_cli(cli: &Cli) -> Result<Self, anyhow::Error> {
        let config_path = &cli.config;
        let root = config_path
            .parent()
            .map(|p| p.to_str().unwrap_or("."))
            .unwrap_or(".");

        // Build config from file
        let s = Config::builder()
            .add_source(File::from(config_path.clone()).required(false))
            .set_default("server.host", "127.0.0.1")?
            .set_default("server.port", 3000)?
            .build()?;

        let mut settings: Settings = s.try_deserialize()?;

        // Apply CLI overrides (CLI > env vars > config file)
        settings.apply_cli_overrides(cli);

        settings.load_external_configs(root)?;

        // Validate configuration
        validator::ConfigValidator::validate(&settings).map_err(|errors| {
            let error_messages: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
            anyhow::anyhow!(
                "Configuration validation failed:\n{}",
                error_messages.join("\n")
            )
        })?;

        // Validate S3 configuration if present
        if let Some(s3_config) = &settings.s3 {
            s3_config.validate().map_err(|errors| {
                anyhow::anyhow!("S3 configuration validation failed:\n{}", errors.join("\n"))
            })?;
        }

        Ok(settings)
    }

    /// Apply CLI argument overrides to settings
    fn apply_cli_overrides(&mut self, cli: &Cli) {
        // Server overrides
        if let Some(host) = &cli.host {
            self.server.host = host.clone();
        }
        if let Some(port) = cli.port {
            self.server.port = port;
        }

        // S3 overrides - initialize S3Config if any S3 CLI args are provided
        if cli.has_s3_config() {
            let s3_config = self.s3.get_or_insert_with(S3Config::default);
            s3_config.merge_cli(cli);
        }
    }

    pub fn from_root(root: &str) -> Result<Self, anyhow::Error> {
        let config_path = std::path::Path::new(root).join("metis");
        let s = Config::builder()
            .add_source(File::from(config_path).required(false))
            .set_default("server.host", "127.0.0.1")?
            .set_default("server.port", 3000)?
            .build()?;

        let mut settings: Settings = s.try_deserialize()?;

        settings.load_external_configs(root)?;

        // Validate configuration
        validator::ConfigValidator::validate(&settings)
            .map_err(|errors| {
                let error_messages: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
                anyhow::anyhow!("Configuration validation failed:\n{}", error_messages.join("\n"))
            })?;

        Ok(settings)
    }

    fn load_external_configs(&mut self, root: &str) -> Result<(), anyhow::Error> {
        self.load_tools_from_dir(&format!("{}/config/tools", root))?;
        self.load_resources_from_dir(&format!("{}/config/resources", root))?;
        self.load_resource_templates_from_dir(&format!("{}/config/resource_templates", root))?;
        self.load_prompts_from_dir(&format!("{}/config/prompts", root))?;
        self.load_workflows_from_dir(&format!("{}/config/workflows", root))?;
        self.load_agents_from_dir(&format!("{}/config/agents", root))?;
        self.load_orchestrations_from_dir(&format!("{}/config/orchestrations", root))?;
        Ok(())
    }

    /// Merge another Settings into this one.
    /// The `other` settings take precedence (override) over `self`.
    /// Arrays are merged by unique identifier (name/uri), with `other` items overriding `self` items.
    pub fn merge(&mut self, other: Settings) {
        // Server settings: other overrides self
        self.server = other.server;

        // Auth: other overrides self
        self.auth = other.auth;

        // Rate limit: other overrides if present
        if other.rate_limit.is_some() {
            self.rate_limit = other.rate_limit;
        }

        // S3 config: other overrides if present
        if other.s3.is_some() {
            self.s3 = other.s3;
        }

        // Secrets: merge individual fields, other overrides if present
        self.secrets.merge(&other.secrets);

        // Merge arrays by identifier
        Self::merge_vec_by_key(&mut self.resources, other.resources, |r| r.uri.clone());
        Self::merge_vec_by_key(&mut self.resource_templates, other.resource_templates, |r| r.uri_template.clone());
        Self::merge_vec_by_key(&mut self.tools, other.tools, |t| t.name.clone());
        Self::merge_vec_by_key(&mut self.prompts, other.prompts, |p| p.name.clone());
        Self::merge_vec_by_key(&mut self.workflows, other.workflows, |w| w.name.clone());
        Self::merge_vec_by_key(&mut self.agents, other.agents, |a| a.name.clone());
        Self::merge_vec_by_key(&mut self.orchestrations, other.orchestrations, |o| o.name.clone());
        Self::merge_vec_by_key(&mut self.mcp_servers, other.mcp_servers, |m| m.name.clone());
    }

    /// Merge two vectors by a key function.
    /// Items from `other` override items in `base` with the same key.
    /// Items from `other` not in `base` are added.
    fn merge_vec_by_key<T, K, F>(base: &mut Vec<T>, other: Vec<T>, key_fn: F)
    where
        K: Eq + std::hash::Hash,
        F: Fn(&T) -> K,
    {
        use std::collections::HashMap;

        // Build a map of existing items by key
        let mut key_to_index: HashMap<K, usize> = HashMap::new();
        for (i, item) in base.iter().enumerate() {
            key_to_index.insert(key_fn(item), i);
        }

        // Process other items
        for item in other {
            let key = key_fn(&item);
            if let Some(&idx) = key_to_index.get(&key) {
                // Override existing item
                base[idx] = item;
            } else {
                // Add new item
                base.push(item);
            }
        }
    }

    /// Merge S3 configuration files into this Settings.
    /// Each file in the list is parsed and merged with precedence (later files override earlier).
    /// Supports TOML, YAML, and JSON formats based on file extension.
    pub fn merge_s3_configs(&mut self, configs: Vec<(String, String)>) {
        for (key, content) in configs {
            // Determine format from key (file path)
            let settings_result: Result<Settings, String> = if key.ends_with(".toml") {
                toml::from_str(&content).map_err(|e| format!("TOML parse error in {}: {}", key, e))
            } else if key.ends_with(".yaml") || key.ends_with(".yml") {
                serde_yaml::from_str(&content).map_err(|e| format!("YAML parse error in {}: {}", key, e))
            } else if key.ends_with(".json") {
                serde_json::from_str(&content).map_err(|e| format!("JSON parse error in {}: {}", key, e))
            } else {
                tracing::warn!("Unknown config file format for S3 key: {}", key);
                continue;
            };

            match settings_result {
                Ok(s3_settings) => {
                    tracing::info!("Merging S3 config from: {}", key);
                    self.merge(s3_settings);
                }
                Err(e) => {
                    tracing::error!("Failed to parse S3 config {}: {}", key, e);
                }
            }
        }
    }

    fn load_tools_from_dir(&mut self, path: &str) -> Result<(), anyhow::Error> {
        let pattern = format!("{}/*", path);
        for entry in glob::glob(&pattern)? {
            match entry {
                Ok(path) => {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if matches!(ext, "json" | "yaml" | "yml") {
                            let content = std::fs::read_to_string(&path)?;
                            let tool: ToolConfig = if ext == "json" {
                                serde_json::from_str(&content)?
                            } else {
                                serde_yaml::from_str(&content)?
                            };
                            self.tools.push(tool);
                        }
                    }
                }
                Err(e) => tracing::warn!("Failed to read glob entry: {}", e),
            }
        }
        Ok(())
    }

    fn load_resources_from_dir(&mut self, path: &str) -> Result<(), anyhow::Error> {
        let pattern = format!("{}/*", path);
        for entry in glob::glob(&pattern)? {
            match entry {
                Ok(path) => {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if matches!(ext, "json" | "yaml" | "yml") {
                            let content = std::fs::read_to_string(&path)?;
                            let resource: ResourceConfig = if ext == "json" {
                                serde_json::from_str(&content)?
                            } else {
                                serde_yaml::from_str(&content)?
                            };
                            self.resources.push(resource);
                        }
                    }
                }
                Err(e) => tracing::warn!("Failed to read glob entry: {}", e),
            }
        }
        Ok(())
    }

    fn load_resource_templates_from_dir(&mut self, path: &str) -> Result<(), anyhow::Error> {
        let pattern = format!("{}/*", path);
        for entry in glob::glob(&pattern)? {
            match entry {
                Ok(path) => {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if matches!(ext, "json" | "yaml" | "yml") {
                            let content = std::fs::read_to_string(&path)?;
                            let resource_template: ResourceTemplateConfig = if ext == "json" {
                                serde_json::from_str(&content)?
                            } else {
                                serde_yaml::from_str(&content)?
                            };
                            self.resource_templates.push(resource_template);
                        }
                    }
                }
                Err(e) => tracing::warn!("Failed to read glob entry: {}", e),
            }
        }
        Ok(())
    }

    fn load_prompts_from_dir(&mut self, path: &str) -> Result<(), anyhow::Error> {
        let pattern = format!("{}/*", path);
        for entry in glob::glob(&pattern)? {
            match entry {
                Ok(path) => {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if matches!(ext, "json" | "yaml" | "yml") {
                            let content = std::fs::read_to_string(&path)?;
                            let prompt: PromptConfig = if ext == "json" {
                                serde_json::from_str(&content)?
                            } else {
                                serde_yaml::from_str(&content)?
                            };
                            self.prompts.push(prompt);
                        }
                    }
                }
                Err(e) => tracing::warn!("Failed to read glob entry: {}", e),
            }
        }
        Ok(())
    }

    fn load_workflows_from_dir(&mut self, path: &str) -> Result<(), anyhow::Error> {
        let pattern = format!("{}/*", path);
        for entry in glob::glob(&pattern)? {
            match entry {
                Ok(path) => {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if matches!(ext, "json" | "yaml" | "yml" | "toml") {
                            let content = std::fs::read_to_string(&path)?;
                            let workflow: WorkflowConfig = match ext {
                                "json" => serde_json::from_str(&content)?,
                                "toml" => toml::from_str(&content)?,
                                _ => serde_yaml::from_str(&content)?,
                            };
                            self.workflows.push(workflow);
                        }
                    }
                }
                Err(e) => tracing::warn!("Failed to read glob entry: {}", e),
            }
        }
        Ok(())
    }

    fn load_agents_from_dir(&mut self, path: &str) -> Result<(), anyhow::Error> {
        let pattern = format!("{}/*", path);
        for entry in glob::glob(&pattern)? {
            match entry {
                Ok(path) => {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if matches!(ext, "json" | "yaml" | "yml" | "toml") {
                            let content = std::fs::read_to_string(&path)?;
                            let agent: AgentConfig = match ext {
                                "json" => serde_json::from_str(&content)?,
                                "toml" => toml::from_str(&content)?,
                                _ => serde_yaml::from_str(&content)?,
                            };
                            self.agents.push(agent);
                        }
                    }
                }
                Err(e) => tracing::warn!("Failed to read glob entry: {}", e),
            }
        }
        Ok(())
    }

    fn load_orchestrations_from_dir(&mut self, path: &str) -> Result<(), anyhow::Error> {
        let pattern = format!("{}/*", path);
        for entry in glob::glob(&pattern)? {
            match entry {
                Ok(path) => {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        if matches!(ext, "json" | "yaml" | "yml" | "toml") {
                            let content = std::fs::read_to_string(&path)?;
                            let orchestration: OrchestrationConfig = match ext {
                                "json" => serde_json::from_str(&content)?,
                                "toml" => toml::from_str(&content)?,
                                _ => serde_yaml::from_str(&content)?,
                            };
                            self.orchestrations.push(orchestration);
                        }
                    }
                }
                Err(e) => tracing::warn!("Failed to read glob entry: {}", e),
            }
        }
        Ok(())
    }
}
