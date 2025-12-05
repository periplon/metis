//! REST API handlers for Web UI configuration management
//!
//! Provides CRUD endpoints for resources, tools, prompts, and workflows.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::adapters::mock_strategy::MockStrategyHandler;
use crate::adapters::rmcp_server::SharedNotificationBroadcaster;
use crate::adapters::secrets::{SharedSecretsStore, SharedPassphraseStore};
use crate::adapters::state_manager::StateManager;
use crate::adapters::tool_handler::AGENT_TOOL_PREFIX;
use crate::adapters::workflow_engine::WorkflowEngine;
use crate::agents::config::{
    AgentConfig, AgentReference, LlmProviderConfig, LlmProviderType, MemoryConfig,
    MergeStrategy, OrchestrationConfig, OrchestrationPattern,
};
use crate::agents::domain::{AgentPort, AgentType};
use crate::adapters::encryption;
use crate::adapters::secrets::keys;
use crate::config::{
    MockConfig, PromptArgument, PromptConfig, PromptMessage, RateLimitConfig, ResourceConfig,
    ResourceTemplateConfig, SchemaConfig, SecretsConfig, Settings, ToolConfig, WorkflowConfig, WorkflowStep,
};
use crate::domain::ToolPort;
use crate::persistence::models::{ArchetypeType, Changeset, Commit, Tag};
use crate::persistence::repository::{ArchetypeRepository, CommitRepository};
use crate::persistence::DataStore;

/// Shared application state for API handlers
#[derive(Clone)]
pub struct ApiState {
    pub settings: Arc<RwLock<Settings>>,
    pub state_manager: Arc<StateManager>,
    pub mock_strategy: Arc<MockStrategyHandler>,
    pub agent_handler: Option<Arc<dyn AgentPort>>,
    /// Shared agent handler for testing (persists memory across requests)
    pub test_agent_handler: Arc<RwLock<Option<Arc<dyn AgentPort>>>>,
    /// Secrets store for API keys (in-memory)
    pub secrets: SharedSecretsStore,
    /// Passphrase store for encrypting secrets when saving config
    pub passphrase: SharedPassphraseStore,
    /// MCP notification broadcaster for list change notifications
    pub broadcaster: Option<SharedNotificationBroadcaster>,
    /// Tool handler to reinitialize agents when agents are created/updated/deleted
    pub tool_handler: Option<Arc<crate::adapters::tool_handler::BasicToolHandler>>,
    /// Database store for archetypes (when database persistence is enabled)
    pub data_store: Option<Arc<DataStore>>,
    /// File storage handler for data lake records
    pub file_storage: Option<Arc<crate::adapters::file_storage::FileStorageHandler>>,
    /// DataFusion handler for SQL queries
    pub datafusion: Option<Arc<crate::adapters::datafusion_handler::DataFusionHandler>>,
}

/// Tool handler for workflow testing that uses mock strategies
struct TestToolHandler {
    settings: Arc<RwLock<Settings>>,
    mock_strategy: Arc<MockStrategyHandler>,
    /// Optional agent handler for testing workflows with agent tools
    agent_handler: Option<Arc<dyn AgentPort>>,
}

impl TestToolHandler {
    fn new(
        settings: Arc<RwLock<Settings>>,
        mock_strategy: Arc<MockStrategyHandler>,
        agent_handler: Option<Arc<dyn AgentPort>>,
    ) -> Self {
        Self {
            settings,
            mock_strategy,
            agent_handler,
        }
    }
}

#[async_trait::async_trait]
impl ToolPort for TestToolHandler {
    async fn list_tools(&self) -> anyhow::Result<Vec<crate::domain::Tool>> {
        let settings = self.settings.read().await;
        let tools = settings
            .tools
            .iter()
            .map(|t| crate::domain::Tool {
                name: t.name.clone(),
                description: t.description.clone(),
                input_schema: t.input_schema.clone(),
                output_schema: t.output_schema.clone(),
            })
            .collect();
        Ok(tools)
    }

    async fn execute_tool(&self, name: &str, args: Value) -> anyhow::Result<Value> {
        // Check if this is an agent tool
        if let Some(agent_name) = name.strip_prefix(AGENT_TOOL_PREFIX) {
            if let Some(agent_handler) = &self.agent_handler {
                // Extract session_id if provided
                let session_id = args
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                // Execute the agent
                let response = agent_handler
                    .execute(agent_name, args.clone(), session_id)
                    .await?;

                // Return agent response as tool result
                return Ok(json!({
                    "content": response.output.get("content").cloned().unwrap_or(Value::Null),
                    "tool_calls": response.tool_calls,
                    "iterations": response.iterations,
                    "execution_time_ms": response.execution_time_ms
                }));
            } else {
                return Err(anyhow::anyhow!(
                    "Agent handler not available. To test workflows with agent tools, ensure agents are configured and the server is running."
                ));
            }
        }

        let settings = self.settings.read().await;
        if let Some(config) = settings.tools.iter().find(|t| t.name == name) {
            let config = config.clone();
            drop(settings);

            if let Some(mock_config) = &config.mock {
                self.mock_strategy.generate(mock_config, Some(&args)).await
            } else if let Some(static_response) = &config.static_response {
                Ok(static_response.clone())
            } else {
                Ok(Value::Null)
            }
        } else {
            Err(anyhow::anyhow!("Tool not found: {}", name))
        }
    }
}

// ============================================================================
// Response Types
// ============================================================================

#[derive(Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }
}

impl<T> ApiResponse<T> {
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}

impl ApiResponse<()> {
    pub fn ok() -> Self {
        Self {
            success: true,
            data: None,
            error: None,
        }
    }
}

#[derive(Serialize)]
pub struct ConfigOverview {
    pub server: ServerInfo,
    pub resources_count: usize,
    pub resource_templates_count: usize,
    pub tools_count: usize,
    pub prompts_count: usize,
    pub workflows_count: usize,
    pub agents_count: usize,
    pub auth_enabled: bool,
    pub rate_limit_enabled: bool,
    pub s3_enabled: bool,
    pub config_file_loaded: bool,
    pub mcp_servers_count: usize,
    pub schemas_count: usize,
    pub data_lakes_count: usize,
    /// Version number for optimistic locking (incremented on each save)
    pub config_version: u64,
}

/// Request body for save operations with optimistic locking
#[derive(Deserialize)]
pub struct SaveConfigRequest {
    /// Expected version number for optimistic locking.
    /// If provided, the save will fail if the current version doesn't match.
    /// If not provided (None), the save will proceed without version checking.
    #[serde(default)]
    pub expected_version: Option<u64>,
}

/// Response for successful save operations
#[derive(Serialize)]
pub struct SaveConfigResponse {
    /// New version number after save
    pub new_version: u64,
}

/// Error response for version conflicts
#[derive(Serialize)]
pub struct VersionConflictResponse {
    pub expected_version: u64,
    pub current_version: u64,
    pub message: String,
}

#[derive(Serialize)]
pub struct ServerInfo {
    pub host: String,
    pub port: u16,
    pub version: String,
}

#[derive(Serialize)]
pub struct HealthInfo {
    pub status: String,
    pub uptime_seconds: u64,
    pub version: String,
}

// ============================================================================
// Serializable Config Types (for API responses)
// ============================================================================

/// DTO for static resources (no input variables, only output schema)
#[derive(Serialize, Deserialize, Clone)]
pub struct ResourceDto {
    pub uri: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Tags for categorization and filtering
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// JSON Schema for the expected output structure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mock: Option<MockConfig>,
}

impl From<&ResourceConfig> for ResourceDto {
    fn from(r: &ResourceConfig) -> Self {
        Self {
            uri: r.uri.clone(),
            name: r.name.clone(),
            description: r.description.clone(),
            mime_type: r.mime_type.clone(),
            tags: r.tags.clone(),
            output_schema: r.output_schema.clone(),
            content: r.content.clone(),
            mock: r.mock.clone(),
        }
    }
}

impl From<ResourceDto> for ResourceConfig {
    fn from(dto: ResourceDto) -> Self {
        Self {
            uri: dto.uri,
            name: dto.name,
            description: dto.description,
            mime_type: dto.mime_type,
            tags: dto.tags,
            output_schema: dto.output_schema,
            content: dto.content,
            mock: dto.mock,
        }
    }
}

/// DTO for resource templates (with URI pattern variables and input schema)
#[derive(Serialize, Deserialize, Clone)]
pub struct ResourceTemplateDto {
    /// URI template pattern (e.g., "postgres://db/users/{id}")
    pub uri_template: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Tags for categorization and filtering
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
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

impl From<&ResourceTemplateConfig> for ResourceTemplateDto {
    fn from(r: &ResourceTemplateConfig) -> Self {
        Self {
            uri_template: r.uri_template.clone(),
            name: r.name.clone(),
            description: r.description.clone(),
            mime_type: r.mime_type.clone(),
            tags: r.tags.clone(),
            input_schema: r.input_schema.clone(),
            output_schema: r.output_schema.clone(),
            content: r.content.clone(),
            mock: r.mock.clone(),
        }
    }
}

impl From<ResourceTemplateDto> for ResourceTemplateConfig {
    fn from(dto: ResourceTemplateDto) -> Self {
        Self {
            uri_template: dto.uri_template,
            name: dto.name,
            description: dto.description,
            mime_type: dto.mime_type,
            tags: dto.tags,
            input_schema: dto.input_schema,
            output_schema: dto.output_schema,
            content: dto.content,
            mock: dto.mock,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ToolDto {
    pub name: String,
    pub description: String,
    /// Tags for categorization and filtering
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub input_schema: Value,
    /// Optional JSON Schema defining the expected output structure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub static_response: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mock: Option<MockConfig>,
}

impl From<&ToolConfig> for ToolDto {
    fn from(t: &ToolConfig) -> Self {
        Self {
            name: t.name.clone(),
            description: t.description.clone(),
            tags: t.tags.clone(),
            input_schema: t.input_schema.clone(),
            output_schema: t.output_schema.clone(),
            static_response: t.static_response.clone(),
            mock: t.mock.clone(),
        }
    }
}

impl From<ToolDto> for ToolConfig {
    fn from(dto: ToolDto) -> Self {
        Self {
            name: dto.name,
            description: dto.description,
            tags: dto.tags,
            input_schema: dto.input_schema,
            output_schema: dto.output_schema,
            static_response: dto.static_response,
            mock: dto.mock,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PromptDto {
    pub name: String,
    pub description: String,
    /// Tags for categorization and filtering
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<PromptArgumentDto>>,
    /// JSON Schema for prompt input parameters (more detailed than arguments)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_schema: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages: Option<Vec<PromptMessageDto>>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PromptArgumentDto {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub required: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PromptMessageDto {
    pub role: String,
    pub content: String,
}

impl From<&PromptConfig> for PromptDto {
    fn from(p: &PromptConfig) -> Self {
        Self {
            name: p.name.clone(),
            description: p.description.clone(),
            tags: p.tags.clone(),
            arguments: p.arguments.as_ref().map(|args| {
                args.iter()
                    .map(|a| PromptArgumentDto {
                        name: a.name.clone(),
                        description: a.description.clone(),
                        required: a.required,
                    })
                    .collect()
            }),
            input_schema: p.input_schema.clone(),
            messages: p.messages.as_ref().map(|msgs| {
                msgs.iter()
                    .map(|m| PromptMessageDto {
                        role: m.role.clone(),
                        content: m.content.clone(),
                    })
                    .collect()
            }),
        }
    }
}

impl From<PromptDto> for PromptConfig {
    fn from(dto: PromptDto) -> Self {
        Self {
            name: dto.name,
            description: dto.description,
            tags: dto.tags,
            arguments: dto.arguments.map(|args| {
                args.into_iter()
                    .map(|a| PromptArgument {
                        name: a.name,
                        description: a.description,
                        required: a.required,
                    })
                    .collect()
            }),
            input_schema: dto.input_schema,
            messages: dto.messages.map(|msgs| {
                msgs.into_iter()
                    .map(|m| PromptMessage {
                        role: m.role,
                        content: m.content,
                    })
                    .collect()
            }),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WorkflowDto {
    pub name: String,
    pub description: String,
    /// Tags for categorization and filtering
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub input_schema: Value,
    /// JSON Schema for the expected workflow output structure
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_schema: Option<Value>,
    pub steps: Vec<WorkflowStepDto>,
    #[serde(default)]
    pub on_error: crate::config::ErrorStrategy,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WorkflowStepDto {
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
    pub on_error: crate::config::ErrorStrategy,
}

fn default_loop_var() -> String {
    "item".to_string()
}

fn default_concurrency() -> u32 {
    1
}

impl From<&WorkflowConfig> for WorkflowDto {
    fn from(w: &WorkflowConfig) -> Self {
        Self {
            name: w.name.clone(),
            description: w.description.clone(),
            tags: w.tags.clone(),
            input_schema: w.input_schema.clone(),
            output_schema: w.output_schema.clone(),
            steps: w.steps.iter().map(WorkflowStepDto::from).collect(),
            on_error: w.on_error.clone(),
        }
    }
}

impl From<&WorkflowStep> for WorkflowStepDto {
    fn from(s: &WorkflowStep) -> Self {
        Self {
            id: s.id.clone(),
            tool: s.tool.clone(),
            args: s.args.clone(),
            depends_on: s.depends_on.clone(),
            condition: s.condition.clone(),
            loop_over: s.loop_over.clone(),
            loop_var: s.loop_var.clone(),
            loop_concurrency: s.loop_concurrency,
            on_error: s.on_error.clone(),
        }
    }
}

impl From<WorkflowDto> for WorkflowConfig {
    fn from(dto: WorkflowDto) -> Self {
        Self {
            name: dto.name,
            description: dto.description,
            tags: dto.tags,
            input_schema: dto.input_schema,
            output_schema: dto.output_schema,
            steps: dto.steps.into_iter().map(WorkflowStep::from).collect(),
            on_error: dto.on_error,
        }
    }
}

impl From<WorkflowStepDto> for WorkflowStep {
    fn from(dto: WorkflowStepDto) -> Self {
        Self {
            id: dto.id,
            tool: dto.tool,
            args: dto.args,
            depends_on: dto.depends_on,
            condition: dto.condition,
            loop_over: dto.loop_over,
            loop_var: dto.loop_var,
            loop_concurrency: dto.loop_concurrency,
            on_error: dto.on_error,
        }
    }
}

// ============================================================================
// Config Overview Endpoints
// ============================================================================

/// GET /api/config - Get configuration overview
pub async fn get_config_overview(
    State(state): State<ApiState>,
) -> impl IntoResponse {
    let settings = state.settings.read().await;

    // Check if config file exists using the stored config path
    let config_file_loaded = settings
        .config_path
        .as_ref()
        .map(|p| {
            let exists = p.exists();
            tracing::debug!("Config path: {:?}, exists: {}", p, exists);
            exists
        })
        .unwrap_or_else(|| {
            tracing::debug!("No config_path set in settings");
            false
        });

    let overview = ConfigOverview {
        server: ServerInfo {
            host: settings.server.host.clone(),
            port: settings.server.port,
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        resources_count: settings.resources.len(),
        resource_templates_count: settings.resource_templates.len(),
        tools_count: settings.tools.len(),
        prompts_count: settings.prompts.len(),
        workflows_count: settings.workflows.len(),
        agents_count: settings.agents.len(),
        auth_enabled: settings.auth.enabled,
        rate_limit_enabled: settings.rate_limit.as_ref().is_some_and(|r| r.enabled),
        s3_enabled: settings.s3.as_ref().is_some_and(|s| s.enabled),
        config_file_loaded,
        mcp_servers_count: settings.mcp_servers.len(),
        schemas_count: settings.schemas.len(),
        data_lakes_count: settings.data_lakes.len(),
        config_version: settings.version,
    };

    (StatusCode::OK, Json(ApiResponse::success(overview)))
}

// ============================================================================
// Server Settings DTOs for editing
// ============================================================================

#[derive(Serialize, Deserialize, Clone)]
pub struct AuthConfigDto {
    pub enabled: bool,
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_keys: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jwt_secret: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jwt_algorithm: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jwks_url: Option<String>,
}

impl From<&crate::domain::auth::AuthConfig> for AuthConfigDto {
    fn from(a: &crate::domain::auth::AuthConfig) -> Self {
        Self {
            enabled: a.enabled,
            mode: format!("{:?}", a.mode),
            api_keys: a.api_keys.clone(),
            jwt_secret: a.jwt_secret.clone(),
            jwt_algorithm: a.jwt_algorithm.clone(),
            jwks_url: a.jwks_url.clone(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RateLimitConfigDto {
    pub enabled: bool,
    pub requests_per_second: u32,
    pub burst_size: u32,
}

impl From<&RateLimitConfig> for RateLimitConfigDto {
    fn from(r: &RateLimitConfig) -> Self {
        Self {
            enabled: r.enabled,
            requests_per_second: r.requests_per_second,
            burst_size: r.burst_size,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct S3ConfigDto {
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

impl From<&crate::config::s3::S3Config> for S3ConfigDto {
    fn from(s: &crate::config::s3::S3Config) -> Self {
        Self {
            enabled: s.enabled,
            bucket: s.bucket.clone(),
            prefix: s.prefix.clone(),
            region: s.region.clone(),
            endpoint: s.endpoint.clone(),
            poll_interval_secs: s.poll_interval_secs,
        }
    }
}

/// Database configuration DTO
#[derive(Serialize, Deserialize, Clone)]
pub struct DatabaseConfigDto {
    pub url: String,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    #[serde(default = "default_auto_migrate")]
    pub auto_migrate: bool,
    #[serde(default)]
    pub seed_on_startup: bool,
    #[serde(default = "default_snapshot_interval")]
    pub snapshot_interval: u32,
}

fn default_max_connections() -> u32 { 5 }
fn default_auto_migrate() -> bool { true }
fn default_snapshot_interval() -> u32 { 10 }

impl From<&crate::persistence::PersistenceConfig> for DatabaseConfigDto {
    fn from(c: &crate::persistence::PersistenceConfig) -> Self {
        Self {
            url: c.url.clone(),
            max_connections: c.max_connections,
            auto_migrate: c.auto_migrate,
            seed_on_startup: c.seed_on_startup,
            snapshot_interval: c.snapshot_interval,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct S3DataConfigDto {
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

impl From<&crate::config::S3DataConfig> for S3DataConfigDto {
    fn from(c: &crate::config::S3DataConfig) -> Self {
        Self {
            bucket: c.bucket.clone(),
            prefix: c.prefix.clone(),
            region: c.region.clone(),
            endpoint: c.endpoint.clone(),
            access_key_id: c.access_key_id.clone(),
            secret_access_key: c.secret_access_key.clone(),
            force_path_style: c.force_path_style,
            allow_http: c.allow_http,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct FileStorageConfigDto {
    #[serde(default)]
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub local_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s3: Option<S3DataConfigDto>,
    #[serde(default)]
    pub default_format: crate::config::DataLakeFileFormat,
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
    #[serde(default = "default_max_file_size")]
    pub max_file_size_bytes: usize,
}

fn default_batch_size() -> usize { 1000 }
fn default_max_file_size() -> usize { 128 * 1024 * 1024 } // 128MB

impl From<&crate::config::FileStorageConfig> for FileStorageConfigDto {
    fn from(c: &crate::config::FileStorageConfig) -> Self {
        Self {
            enabled: c.enabled,
            local_path: c.local_path.clone(),
            s3: c.s3.as_ref().map(S3DataConfigDto::from),
            default_format: c.default_format.clone(),
            batch_size: c.batch_size,
            max_file_size_bytes: c.max_file_size_bytes,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ServerSettingsDto {
    pub auth: AuthConfigDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<RateLimitConfigDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s3: Option<S3ConfigDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub database: Option<DatabaseConfigDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_storage: Option<FileStorageConfigDto>,
}

/// GET /api/config/settings - Get editable server settings
pub async fn get_server_settings(
    State(state): State<ApiState>,
) -> impl IntoResponse {
    let settings = state.settings.read().await;

    // Always return S3 config (with defaults if not configured) so UI can edit it
    let s3_dto = settings.s3.as_ref().map(S3ConfigDto::from).unwrap_or_else(|| {
        S3ConfigDto {
            enabled: false,
            bucket: None,
            prefix: None,
            region: None,
            endpoint: None,
            poll_interval_secs: 30,
        }
    });

    let dto = ServerSettingsDto {
        auth: AuthConfigDto::from(&settings.auth),
        rate_limit: settings.rate_limit.as_ref().map(RateLimitConfigDto::from),
        s3: Some(s3_dto),
        database: settings.database.as_ref().map(DatabaseConfigDto::from),
        file_storage: settings.file_storage.as_ref().map(FileStorageConfigDto::from),
    };

    (StatusCode::OK, Json(ApiResponse::success(dto)))
}

/// PUT /api/config/settings - Update server settings
pub async fn update_server_settings(
    State(state): State<ApiState>,
    Json(dto): Json<ServerSettingsDto>,
) -> impl IntoResponse {
    let mut settings = state.settings.write().await;

    // Update auth settings
    settings.auth.enabled = dto.auth.enabled;
    if let Some(keys) = dto.auth.api_keys {
        settings.auth.api_keys = Some(keys);
    }
    if let Some(secret) = dto.auth.jwt_secret {
        settings.auth.jwt_secret = Some(secret);
    }
    if let Some(algo) = dto.auth.jwt_algorithm {
        settings.auth.jwt_algorithm = Some(algo);
    }
    if let Some(url) = dto.auth.jwks_url {
        settings.auth.jwks_url = Some(url);
    }

    // Update rate limit settings
    if let Some(rate_limit_dto) = dto.rate_limit {
        if let Some(ref mut rate_limit) = settings.rate_limit {
            rate_limit.enabled = rate_limit_dto.enabled;
            rate_limit.requests_per_second = rate_limit_dto.requests_per_second;
            rate_limit.burst_size = rate_limit_dto.burst_size;
        } else {
            settings.rate_limit = Some(RateLimitConfig {
                enabled: rate_limit_dto.enabled,
                requests_per_second: rate_limit_dto.requests_per_second,
                burst_size: rate_limit_dto.burst_size,
            });
        }
    }

    // Update S3 settings
    if let Some(s3_dto) = dto.s3 {
        if let Some(ref mut s3) = settings.s3 {
            s3.enabled = s3_dto.enabled;
            s3.bucket = s3_dto.bucket;
            s3.prefix = s3_dto.prefix;
            s3.region = s3_dto.region;
            s3.endpoint = s3_dto.endpoint;
            s3.poll_interval_secs = s3_dto.poll_interval_secs;
        } else {
            settings.s3 = Some(crate::config::s3::S3Config {
                enabled: s3_dto.enabled,
                bucket: s3_dto.bucket,
                prefix: s3_dto.prefix,
                region: s3_dto.region,
                endpoint: s3_dto.endpoint,
                poll_interval_secs: s3_dto.poll_interval_secs,
            });
        }
    }

    // Update database settings
    if let Some(db_dto) = dto.database {
        if !db_dto.url.is_empty() {
            settings.database = Some(crate::persistence::PersistenceConfig {
                url: db_dto.url,
                max_connections: db_dto.max_connections,
                auto_migrate: db_dto.auto_migrate,
                seed_on_startup: db_dto.seed_on_startup,
                snapshot_interval: db_dto.snapshot_interval,
            });
        } else {
            // Empty URL means disable database
            settings.database = None;
        }
    }

    // Update file storage settings
    if let Some(fs_dto) = dto.file_storage {
        if fs_dto.enabled {
            settings.file_storage = Some(crate::config::FileStorageConfig {
                enabled: fs_dto.enabled,
                local_path: fs_dto.local_path,
                s3: fs_dto.s3.map(|s3| crate::config::S3DataConfig {
                    bucket: s3.bucket,
                    prefix: s3.prefix,
                    region: s3.region,
                    endpoint: s3.endpoint,
                    access_key_id: s3.access_key_id,
                    secret_access_key: s3.secret_access_key,
                    force_path_style: s3.force_path_style,
                    allow_http: s3.allow_http,
                }),
                default_format: fs_dto.default_format,
                batch_size: fs_dto.batch_size,
                max_file_size_bytes: fs_dto.max_file_size_bytes,
            });
        } else {
            // Disabled means remove file storage config
            settings.file_storage = None;
        }
    }

    let response_dto = ServerSettingsDto {
        auth: AuthConfigDto::from(&settings.auth),
        rate_limit: settings.rate_limit.as_ref().map(RateLimitConfigDto::from),
        s3: settings.s3.as_ref().map(S3ConfigDto::from),
        database: settings.database.as_ref().map(DatabaseConfigDto::from),
        file_storage: settings.file_storage.as_ref().map(FileStorageConfigDto::from),
    };

    (StatusCode::OK, Json(ApiResponse::success(response_dto)))
}

/// GET /api/metrics/json - Get metrics as JSON (for dashboard)
pub async fn get_metrics_json(
    State(state): State<ApiState>,
) -> impl IntoResponse {
    let settings = state.settings.read().await;

    let metrics = json!({
        "resources_count": settings.resources.len(),
        "tools_count": settings.tools.len(),
        "prompts_count": settings.prompts.len(),
        "workflows_count": settings.workflows.len(),
        "version": env!("CARGO_PKG_VERSION"),
    });

    (StatusCode::OK, Json(ApiResponse::success(metrics)))
}

// ============================================================================
// Resource CRUD Endpoints
// ============================================================================

/// GET /api/resources - List all resources
pub async fn list_resources(
    State(state): State<ApiState>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        match store.archetypes().list(ArchetypeType::Resource.as_str()).await {
            Ok(resources) => {
                let dtos: Vec<ResourceDto> = resources
                    .into_iter()
                    .filter_map(|v| serde_json::from_value(v).ok())
                    .collect();
                return (StatusCode::OK, Json(ApiResponse::success(dtos)));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<Vec<ResourceDto>>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let settings = state.settings.read().await;
    let resources: Vec<ResourceDto> = settings.resources.iter().map(ResourceDto::from).collect();
    (StatusCode::OK, Json(ApiResponse::success(resources)))
}

/// GET /api/resources/:uri - Get a single resource
pub async fn get_resource(
    State(state): State<ApiState>,
    Path(uri): Path<String>,
) -> impl IntoResponse {
    let decoded_uri = urlencoding::decode(&uri).map(|s| s.into_owned()).unwrap_or(uri.clone());

    // Use database if available
    if let Some(store) = &state.data_store {
        match store.archetypes().get(ArchetypeType::Resource.as_str(), &decoded_uri).await {
            Ok(Some(resource)) => {
                match serde_json::from_value::<ResourceDto>(resource) {
                    Ok(dto) => return (StatusCode::OK, Json(ApiResponse::success(dto))),
                    Err(e) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ApiResponse::<ResourceDto>::error(format!("Failed to parse resource: {}", e))),
                        );
                    }
                }
            }
            Ok(None) => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<ResourceDto>::error("Resource not found")));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<ResourceDto>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let settings = state.settings.read().await;
    if let Some(resource) = settings.resources.iter().find(|r| r.uri == decoded_uri) {
        (StatusCode::OK, Json(ApiResponse::success(ResourceDto::from(resource))))
    } else {
        (StatusCode::NOT_FOUND, Json(ApiResponse::<ResourceDto>::error("Resource not found")))
    }
}

/// POST /api/resources - Create a new resource
pub async fn create_resource(
    State(state): State<ApiState>,
    Json(dto): Json<ResourceDto>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        let definition = match serde_json::to_value(&dto) {
            Ok(v) => v,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<ResourceDto>::error(format!("Invalid resource data: {}", e))),
                );
            }
        };

        // Use URI as the name for resources
        match store.archetypes().create(ArchetypeType::Resource.as_str(), &dto.uri, &definition).await {
            Ok(()) => {
                // CRITICAL: Also add to in-memory settings so MCP handlers see the new resource
                {
                    let mut settings = state.settings.write().await;
                    settings.resources.push(crate::config::ResourceConfig {
                        uri: dto.uri.clone(),
                        name: dto.name.clone(),
                        description: dto.description.clone(),
                        mime_type: dto.mime_type.clone(),
                        tags: dto.tags.clone(),
                        output_schema: dto.output_schema.clone(),
                        content: dto.content.clone(),
                        mock: dto.mock.clone(),
                    });
                }
                // Auto-sync to S3 if configured
                let safe_name = sanitize_uri_for_s3(&dto.uri);
                if let Err(e) = sync_item_to_s3_if_active(&state, "resources", &safe_name, &dto).await {
                    tracing::warn!("Failed to sync resource to S3: {}", e);
                }
                if let Some(broadcaster) = &state.broadcaster {
                    broadcaster.notify_resources_changed().await;
                    broadcaster.notify_tools_changed().await;
                }
                return (StatusCode::CREATED, Json(ApiResponse::success(dto)));
            }
            Err(crate::persistence::error::PersistenceError::Duplicate { .. }) => {
                return (
                    StatusCode::CONFLICT,
                    Json(ApiResponse::<ResourceDto>::error("Resource with this URI already exists")),
                );
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<ResourceDto>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let mut settings = state.settings.write().await;

    // Check for duplicate URI
    if settings.resources.iter().any(|r| r.uri == dto.uri) {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::<ResourceDto>::error("Resource with this URI already exists")),
        );
    }

    let resource = ResourceConfig::from(dto.clone());
    settings.resources.push(resource);
    drop(settings);

    // Auto-sync to S3 if configured
    let safe_name = sanitize_uri_for_s3(&dto.uri);
    if let Err(e) = sync_item_to_s3_if_active(&state, "resources", &safe_name, &dto).await {
        tracing::warn!("Failed to sync resource to S3: {}", e);
    }

    // Notify connected MCP clients about the resource list change
    // Resources are also exposed as tools, so notify both
    if let Some(broadcaster) = &state.broadcaster {
        broadcaster.notify_resources_changed().await;
        broadcaster.notify_tools_changed().await;
    }

    (StatusCode::CREATED, Json(ApiResponse::success(dto)))
}

/// PUT /api/resources/:uri - Update a resource
pub async fn update_resource(
    State(state): State<ApiState>,
    Path(uri): Path<String>,
    Json(dto): Json<ResourceDto>,
) -> impl IntoResponse {
    let decoded_uri = urlencoding::decode(&uri).map(|s| s.into_owned()).unwrap_or(uri.clone());

    // Use database if available
    if let Some(store) = &state.data_store {
        let definition = match serde_json::to_value(&dto) {
            Ok(v) => v,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<ResourceDto>::error(format!("Invalid resource data: {}", e))),
                );
            }
        };

        match store.archetypes().update(ArchetypeType::Resource.as_str(), &decoded_uri, &definition, None).await {
            Ok(_) => {
                // CRITICAL: Also update in-memory settings so MCP handlers see the change
                {
                    let mut settings = state.settings.write().await;
                    if let Some(resource) = settings.resources.iter_mut().find(|r| r.uri == decoded_uri) {
                        resource.name = dto.name.clone();
                        resource.description = dto.description.clone();
                        resource.mime_type = dto.mime_type.clone();
                        resource.tags = dto.tags.clone();
                        resource.output_schema = dto.output_schema.clone();
                        resource.content = dto.content.clone();
                        resource.mock = dto.mock.clone();
                    }
                }
                // Auto-sync to S3 if configured
                let safe_name = sanitize_uri_for_s3(&dto.uri);
                if let Err(e) = sync_item_to_s3_if_active(&state, "resources", &safe_name, &dto).await {
                    tracing::warn!("Failed to sync resource to S3: {}", e);
                }
                if let Some(broadcaster) = &state.broadcaster {
                    broadcaster.notify_resources_changed().await;
                    broadcaster.notify_tools_changed().await;
                }
                return (StatusCode::OK, Json(ApiResponse::success(dto)));
            }
            Err(crate::persistence::error::PersistenceError::NotFound { .. }) => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<ResourceDto>::error("Resource not found")));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<ResourceDto>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let mut settings = state.settings.write().await;

    if let Some(resource) = settings.resources.iter_mut().find(|r| r.uri == decoded_uri) {
        resource.name = dto.name.clone();
        resource.description = dto.description.clone();
        resource.mime_type = dto.mime_type.clone();
        resource.tags = dto.tags.clone();
        resource.output_schema = dto.output_schema.clone();
        resource.content = dto.content.clone();
        resource.mock = dto.mock.clone();
        drop(settings);

        // Auto-sync to S3 if configured
        let safe_name = sanitize_uri_for_s3(&dto.uri);
        if let Err(e) = sync_item_to_s3_if_active(&state, "resources", &safe_name, &dto).await {
            tracing::warn!("Failed to sync resource to S3: {}", e);
        }

        // Notify connected MCP clients about the resource list change
        // Resources are also exposed as tools, so notify both
        if let Some(broadcaster) = &state.broadcaster {
            broadcaster.notify_resources_changed().await;
            broadcaster.notify_tools_changed().await;
        }

        (StatusCode::OK, Json(ApiResponse::success(dto)))
    } else {
        (StatusCode::NOT_FOUND, Json(ApiResponse::<ResourceDto>::error("Resource not found")))
    }
}

/// DELETE /api/resources/:uri - Delete a resource
pub async fn delete_resource(
    State(state): State<ApiState>,
    Path(uri): Path<String>,
) -> impl IntoResponse {
    let decoded_uri = urlencoding::decode(&uri).map(|s| s.into_owned()).unwrap_or(uri.clone());

    // Use database if available
    if let Some(store) = &state.data_store {
        match store.archetypes().delete(ArchetypeType::Resource.as_str(), &decoded_uri).await {
            Ok(true) => {
                // CRITICAL: Also remove from in-memory settings so MCP handlers don't see deleted resource
                {
                    let mut settings = state.settings.write().await;
                    settings.resources.retain(|r| r.uri != decoded_uri);
                }
                // Auto-delete from S3 if configured
                let safe_name = sanitize_uri_for_s3(&decoded_uri);
                if let Err(e) = delete_item_from_s3_if_active(&state, "resources", &safe_name).await {
                    tracing::warn!("Failed to delete resource from S3: {}", e);
                }
                if let Some(broadcaster) = &state.broadcaster {
                    broadcaster.notify_resources_changed().await;
                    broadcaster.notify_tools_changed().await;
                }
                return (StatusCode::OK, Json(ApiResponse::<()>::ok()));
            }
            Ok(false) => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<()>::error("Resource not found")));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<()>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let mut settings = state.settings.write().await;

    let initial_len = settings.resources.len();
    settings.resources.retain(|r| r.uri != decoded_uri);

    if settings.resources.len() < initial_len {
        drop(settings);

        // Auto-delete from S3 if configured
        let safe_name = sanitize_uri_for_s3(&decoded_uri);
        if let Err(e) = delete_item_from_s3_if_active(&state, "resources", &safe_name).await {
            tracing::warn!("Failed to delete resource from S3: {}", e);
        }

        // Notify connected MCP clients about the resource list change
        // Resources are also exposed as tools, so notify both
        if let Some(broadcaster) = &state.broadcaster {
            broadcaster.notify_resources_changed().await;
            broadcaster.notify_tools_changed().await;
        }

        (StatusCode::OK, Json(ApiResponse::<()>::ok()))
    } else {
        (StatusCode::NOT_FOUND, Json(ApiResponse::<()>::error("Resource not found")))
    }
}

// ============================================================================
// Tool CRUD Endpoints
// ============================================================================

/// GET /api/tools - List all tools
pub async fn list_tools(
    State(state): State<ApiState>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        match store.archetypes().list(ArchetypeType::Tool.as_str()).await {
            Ok(tools) => {
                let dtos: Vec<ToolDto> = tools
                    .into_iter()
                    .filter_map(|v| serde_json::from_value(v).ok())
                    .collect();
                return (StatusCode::OK, Json(ApiResponse::success(dtos)));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<Vec<ToolDto>>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let settings = state.settings.read().await;
    let tools: Vec<ToolDto> = settings.tools.iter().map(ToolDto::from).collect();
    (StatusCode::OK, Json(ApiResponse::success(tools)))
}

/// GET /api/tools/:name - Get a single tool
pub async fn get_tool(
    State(state): State<ApiState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        match store.archetypes().get(ArchetypeType::Tool.as_str(), &name).await {
            Ok(Some(tool)) => {
                match serde_json::from_value::<ToolDto>(tool) {
                    Ok(dto) => {
                        tracing::info!("get_tool: Returning tool '{}' with output_schema = {:?}", dto.name, dto.output_schema.as_ref().map(|s| serde_json::to_string(s).unwrap_or_default()));
                        return (StatusCode::OK, Json(ApiResponse::success(dto)));
                    }
                    Err(e) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ApiResponse::<ToolDto>::error(format!("Failed to parse tool: {}", e))),
                        );
                    }
                }
            }
            Ok(None) => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<ToolDto>::error("Tool not found")));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<ToolDto>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let settings = state.settings.read().await;
    if let Some(tool) = settings.tools.iter().find(|t| t.name == name) {
        (StatusCode::OK, Json(ApiResponse::success(ToolDto::from(tool))))
    } else {
        (StatusCode::NOT_FOUND, Json(ApiResponse::<ToolDto>::error("Tool not found")))
    }
}

/// POST /api/tools - Create a new tool
pub async fn create_tool(
    State(state): State<ApiState>,
    Json(dto): Json<ToolDto>,
) -> impl IntoResponse {
    tracing::info!("create_tool: Received tool '{}' with output_schema = {:?}", dto.name, dto.output_schema.as_ref().map(|s| serde_json::to_string(s).unwrap_or_default()));
    // Use database if available
    if let Some(store) = &state.data_store {
        let definition = match serde_json::to_value(&dto) {
            Ok(v) => v,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<ToolDto>::error(format!("Invalid tool data: {}", e))),
                );
            }
        };

        match store.archetypes().create(ArchetypeType::Tool.as_str(), &dto.name, &definition).await {
            Ok(()) => {
                // CRITICAL: Also add to in-memory settings so MCP handlers see the new tool
                {
                    let mut settings = state.settings.write().await;
                    settings.tools.push(ToolConfig::from(dto.clone()));
                }
                // Auto-sync to S3 if configured
                if let Err(e) = sync_item_to_s3_if_active(&state, "tools", &dto.name, &dto).await {
                    tracing::warn!("Failed to sync tool to S3: {}", e);
                }
                // Notify connected MCP clients about the tool list change
                if let Some(broadcaster) = &state.broadcaster {
                    broadcaster.notify_tools_changed().await;
                }
                return (StatusCode::CREATED, Json(ApiResponse::success(dto)));
            }
            Err(crate::persistence::error::PersistenceError::Duplicate { .. }) => {
                return (
                    StatusCode::CONFLICT,
                    Json(ApiResponse::<ToolDto>::error("Tool with this name already exists")),
                );
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<ToolDto>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let mut settings = state.settings.write().await;

    // Check for duplicate name
    if settings.tools.iter().any(|t| t.name == dto.name) {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::<ToolDto>::error("Tool with this name already exists")),
        );
    }

    let tool = ToolConfig::from(dto.clone());
    settings.tools.push(tool);
    drop(settings);

    // Auto-sync to S3 if configured
    if let Err(e) = sync_item_to_s3_if_active(&state, "tools", &dto.name, &dto).await {
        tracing::warn!("Failed to sync tool to S3: {}", e);
    }

    // Notify connected MCP clients about the tool list change
    if let Some(broadcaster) = &state.broadcaster {
        broadcaster.notify_tools_changed().await;
    }

    (StatusCode::CREATED, Json(ApiResponse::success(dto)))
}

/// PUT /api/tools/:name - Update a tool
pub async fn update_tool(
    State(state): State<ApiState>,
    Path(name): Path<String>,
    Json(dto): Json<ToolDto>,
) -> impl IntoResponse {
    tracing::info!("update_tool: Received tool '{}' with output_schema = {:?}", dto.name, dto.output_schema.as_ref().map(|s| serde_json::to_string(s).unwrap_or_default()));
    // Use database if available
    if let Some(store) = &state.data_store {
        let definition = match serde_json::to_value(&dto) {
            Ok(v) => v,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<ToolDto>::error(format!("Invalid tool data: {}", e))),
                );
            }
        };

        match store.archetypes().update(ArchetypeType::Tool.as_str(), &name, &definition, None).await {
            Ok(_) => {
                // CRITICAL: Also update in-memory settings so MCP handlers see the change
                {
                    let mut settings = state.settings.write().await;
                    // Replace the tool config entirely with the new one
                    if let Some(idx) = settings.tools.iter().position(|t| t.name == name) {
                        settings.tools[idx] = ToolConfig::from(dto.clone());
                    }
                }
                // Auto-sync to S3 if configured
                if let Err(e) = sync_item_to_s3_if_active(&state, "tools", &dto.name, &dto).await {
                    tracing::warn!("Failed to sync tool to S3: {}", e);
                }
                // Notify connected MCP clients about the tool list change
                if let Some(broadcaster) = &state.broadcaster {
                    broadcaster.notify_tools_changed().await;
                }
                return (StatusCode::OK, Json(ApiResponse::success(dto)));
            }
            Err(crate::persistence::error::PersistenceError::NotFound { .. }) => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<ToolDto>::error("Tool not found")));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<ToolDto>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let mut settings = state.settings.write().await;

    if let Some(tool) = settings.tools.iter_mut().find(|t| t.name == name) {
        tool.description = dto.description.clone();
        tool.tags = dto.tags.clone();
        tool.input_schema = dto.input_schema.clone();
        tool.output_schema = dto.output_schema.clone();
        tool.static_response = dto.static_response.clone();
        tool.mock = dto.mock.clone();
        drop(settings);

        // Auto-sync to S3 if configured
        if let Err(e) = sync_item_to_s3_if_active(&state, "tools", &dto.name, &dto).await {
            tracing::warn!("Failed to sync tool to S3: {}", e);
        }

        // Notify connected MCP clients about the tool list change
        if let Some(broadcaster) = &state.broadcaster {
            broadcaster.notify_tools_changed().await;
        }

        (StatusCode::OK, Json(ApiResponse::success(dto)))
    } else {
        (StatusCode::NOT_FOUND, Json(ApiResponse::<ToolDto>::error("Tool not found")))
    }
}

/// DELETE /api/tools/:name - Delete a tool
pub async fn delete_tool(
    State(state): State<ApiState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        match store.archetypes().delete(ArchetypeType::Tool.as_str(), &name).await {
            Ok(true) => {
                // CRITICAL: Also remove from in-memory settings so MCP handlers don't see deleted tool
                {
                    let mut settings = state.settings.write().await;
                    settings.tools.retain(|t| t.name != name);
                }
                // Auto-delete from S3 if configured
                if let Err(e) = delete_item_from_s3_if_active(&state, "tools", &name).await {
                    tracing::warn!("Failed to delete tool from S3: {}", e);
                }
                // Notify connected MCP clients about the tool list change
                if let Some(broadcaster) = &state.broadcaster {
                    broadcaster.notify_tools_changed().await;
                }
                return (StatusCode::OK, Json(ApiResponse::<()>::ok()));
            }
            Ok(false) => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<()>::error("Tool not found")));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<()>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let mut settings = state.settings.write().await;

    let initial_len = settings.tools.len();
    settings.tools.retain(|t| t.name != name);

    if settings.tools.len() < initial_len {
        drop(settings);

        // Auto-delete from S3 if configured
        if let Err(e) = delete_item_from_s3_if_active(&state, "tools", &name).await {
            tracing::warn!("Failed to delete tool from S3: {}", e);
        }

        // Notify connected MCP clients about the tool list change
        if let Some(broadcaster) = &state.broadcaster {
            broadcaster.notify_tools_changed().await;
        }

        (StatusCode::OK, Json(ApiResponse::<()>::ok()))
    } else {
        (StatusCode::NOT_FOUND, Json(ApiResponse::<()>::error("Tool not found")))
    }
}

// ============================================================================
// Prompt CRUD Endpoints
// ============================================================================

/// GET /api/prompts - List all prompts
pub async fn list_prompts(
    State(state): State<ApiState>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        match store.archetypes().list(ArchetypeType::Prompt.as_str()).await {
            Ok(prompts) => {
                let dtos: Vec<PromptDto> = prompts
                    .into_iter()
                    .filter_map(|v| serde_json::from_value(v).ok())
                    .collect();
                return (StatusCode::OK, Json(ApiResponse::success(dtos)));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<Vec<PromptDto>>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let settings = state.settings.read().await;
    let prompts: Vec<PromptDto> = settings.prompts.iter().map(PromptDto::from).collect();
    (StatusCode::OK, Json(ApiResponse::success(prompts)))
}

/// GET /api/prompts/:name - Get a single prompt
pub async fn get_prompt(
    State(state): State<ApiState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        match store.archetypes().get(ArchetypeType::Prompt.as_str(), &name).await {
            Ok(Some(prompt)) => {
                match serde_json::from_value::<PromptDto>(prompt) {
                    Ok(dto) => return (StatusCode::OK, Json(ApiResponse::success(dto))),
                    Err(e) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ApiResponse::<PromptDto>::error(format!("Failed to parse prompt: {}", e))),
                        );
                    }
                }
            }
            Ok(None) => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<PromptDto>::error("Prompt not found")));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<PromptDto>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let settings = state.settings.read().await;
    if let Some(prompt) = settings.prompts.iter().find(|p| p.name == name) {
        (StatusCode::OK, Json(ApiResponse::success(PromptDto::from(prompt))))
    } else {
        (StatusCode::NOT_FOUND, Json(ApiResponse::<PromptDto>::error("Prompt not found")))
    }
}

/// POST /api/prompts - Create a new prompt
pub async fn create_prompt(
    State(state): State<ApiState>,
    Json(dto): Json<PromptDto>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        let definition = match serde_json::to_value(&dto) {
            Ok(v) => v,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<PromptDto>::error(format!("Invalid prompt data: {}", e))),
                );
            }
        };

        match store.archetypes().create(ArchetypeType::Prompt.as_str(), &dto.name, &definition).await {
            Ok(()) => {
                // CRITICAL: Also add to in-memory settings so MCP handlers see the new prompt
                {
                    let mut settings = state.settings.write().await;
                    settings.prompts.push(PromptConfig::from(dto.clone()));
                }
                // Auto-sync to S3 if configured
                if let Err(e) = sync_item_to_s3_if_active(&state, "prompts", &dto.name, &dto).await {
                    tracing::warn!("Failed to sync prompt to S3: {}", e);
                }
                if let Some(broadcaster) = &state.broadcaster {
                    broadcaster.notify_prompts_changed().await;
                }
                return (StatusCode::CREATED, Json(ApiResponse::success(dto)));
            }
            Err(crate::persistence::error::PersistenceError::Duplicate { .. }) => {
                return (
                    StatusCode::CONFLICT,
                    Json(ApiResponse::<PromptDto>::error("Prompt with this name already exists")),
                );
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<PromptDto>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let mut settings = state.settings.write().await;

    // Check for duplicate name
    if settings.prompts.iter().any(|p| p.name == dto.name) {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::<PromptDto>::error("Prompt with this name already exists")),
        );
    }

    let prompt = PromptConfig::from(dto.clone());
    settings.prompts.push(prompt);
    drop(settings);

    // Auto-sync to S3 if configured
    if let Err(e) = sync_item_to_s3_if_active(&state, "prompts", &dto.name, &dto).await {
        tracing::warn!("Failed to sync prompt to S3: {}", e);
    }

    // Notify connected MCP clients about the prompt list change
    if let Some(broadcaster) = &state.broadcaster {
        broadcaster.notify_prompts_changed().await;
    }

    (StatusCode::CREATED, Json(ApiResponse::success(dto)))
}

/// PUT /api/prompts/:name - Update a prompt
pub async fn update_prompt(
    State(state): State<ApiState>,
    Path(name): Path<String>,
    Json(dto): Json<PromptDto>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        let definition = match serde_json::to_value(&dto) {
            Ok(v) => v,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<PromptDto>::error(format!("Invalid prompt data: {}", e))),
                );
            }
        };

        match store.archetypes().update(ArchetypeType::Prompt.as_str(), &name, &definition, None).await {
            Ok(_) => {
                // CRITICAL: Also update in-memory settings so MCP handlers see the change
                {
                    let mut settings = state.settings.write().await;
                    // Replace the prompt config entirely with the new one
                    if let Some(idx) = settings.prompts.iter().position(|p| p.name == name) {
                        settings.prompts[idx] = PromptConfig::from(dto.clone());
                    }
                }
                // Auto-sync to S3 if configured
                if let Err(e) = sync_item_to_s3_if_active(&state, "prompts", &dto.name, &dto).await {
                    tracing::warn!("Failed to sync prompt to S3: {}", e);
                }
                if let Some(broadcaster) = &state.broadcaster {
                    broadcaster.notify_prompts_changed().await;
                }
                return (StatusCode::OK, Json(ApiResponse::success(dto)));
            }
            Err(crate::persistence::error::PersistenceError::NotFound { .. }) => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<PromptDto>::error("Prompt not found")));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<PromptDto>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let mut settings = state.settings.write().await;

    if let Some(prompt) = settings.prompts.iter_mut().find(|p| p.name == name) {
        prompt.description = dto.description.clone();
        prompt.tags = dto.tags.clone();
        prompt.arguments = dto.arguments.clone().map(|args| {
            args.into_iter()
                .map(|a| PromptArgument {
                    name: a.name,
                    description: a.description,
                    required: a.required,
                })
                .collect()
        });
        prompt.messages = dto.messages.clone().map(|msgs| {
            msgs.into_iter()
                .map(|m| PromptMessage {
                    role: m.role,
                    content: m.content,
                })
                .collect()
        });
        drop(settings);

        // Auto-sync to S3 if configured
        if let Err(e) = sync_item_to_s3_if_active(&state, "prompts", &dto.name, &dto).await {
            tracing::warn!("Failed to sync prompt to S3: {}", e);
        }

        // Notify connected MCP clients about the prompt list change
        if let Some(broadcaster) = &state.broadcaster {
            broadcaster.notify_prompts_changed().await;
        }

        (StatusCode::OK, Json(ApiResponse::success(dto)))
    } else {
        (StatusCode::NOT_FOUND, Json(ApiResponse::<PromptDto>::error("Prompt not found")))
    }
}

/// DELETE /api/prompts/:name - Delete a prompt
pub async fn delete_prompt(
    State(state): State<ApiState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        match store.archetypes().delete(ArchetypeType::Prompt.as_str(), &name).await {
            Ok(true) => {
                // CRITICAL: Also remove from in-memory settings so MCP handlers don't see deleted prompt
                {
                    let mut settings = state.settings.write().await;
                    settings.prompts.retain(|p| p.name != name);
                }
                // Auto-delete from S3 if configured
                if let Err(e) = delete_item_from_s3_if_active(&state, "prompts", &name).await {
                    tracing::warn!("Failed to delete prompt from S3: {}", e);
                }
                if let Some(broadcaster) = &state.broadcaster {
                    broadcaster.notify_prompts_changed().await;
                }
                return (StatusCode::OK, Json(ApiResponse::<()>::ok()));
            }
            Ok(false) => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<()>::error("Prompt not found")));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<()>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let mut settings = state.settings.write().await;

    let initial_len = settings.prompts.len();
    settings.prompts.retain(|p| p.name != name);

    if settings.prompts.len() < initial_len {
        drop(settings);

        // Auto-delete from S3 if configured
        if let Err(e) = delete_item_from_s3_if_active(&state, "prompts", &name).await {
            tracing::warn!("Failed to delete prompt from S3: {}", e);
        }

        // Notify connected MCP clients about the prompt list change
        if let Some(broadcaster) = &state.broadcaster {
            broadcaster.notify_prompts_changed().await;
        }

        (StatusCode::OK, Json(ApiResponse::<()>::ok()))
    } else {
        (StatusCode::NOT_FOUND, Json(ApiResponse::<()>::error("Prompt not found")))
    }
}

// ============================================================================
// Workflow CRUD Endpoints
// ============================================================================

/// GET /api/workflows - List all workflows
pub async fn list_workflows(
    State(state): State<ApiState>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        match store.archetypes().list(ArchetypeType::Workflow.as_str()).await {
            Ok(workflows) => {
                let dtos: Vec<WorkflowDto> = workflows
                    .into_iter()
                    .filter_map(|v| serde_json::from_value(v).ok())
                    .collect();
                return (StatusCode::OK, Json(ApiResponse::success(dtos)));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<Vec<WorkflowDto>>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let settings = state.settings.read().await;
    let workflows: Vec<WorkflowDto> = settings.workflows.iter().map(WorkflowDto::from).collect();
    (StatusCode::OK, Json(ApiResponse::success(workflows)))
}

/// GET /api/workflows/:name - Get a single workflow
pub async fn get_workflow(
    State(state): State<ApiState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        match store.archetypes().get(ArchetypeType::Workflow.as_str(), &name).await {
            Ok(Some(workflow)) => {
                match serde_json::from_value::<WorkflowDto>(workflow) {
                    Ok(dto) => return (StatusCode::OK, Json(ApiResponse::success(dto))),
                    Err(e) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ApiResponse::<WorkflowDto>::error(format!("Failed to parse workflow: {}", e))),
                        );
                    }
                }
            }
            Ok(None) => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<WorkflowDto>::error("Workflow not found")));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<WorkflowDto>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let settings = state.settings.read().await;
    if let Some(workflow) = settings.workflows.iter().find(|w| w.name == name) {
        (StatusCode::OK, Json(ApiResponse::success(WorkflowDto::from(workflow))))
    } else {
        (StatusCode::NOT_FOUND, Json(ApiResponse::<WorkflowDto>::error("Workflow not found")))
    }
}

/// POST /api/workflows - Create a new workflow
pub async fn create_workflow(
    State(state): State<ApiState>,
    Json(dto): Json<WorkflowDto>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        let definition = match serde_json::to_value(&dto) {
            Ok(v) => v,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<WorkflowDto>::error(format!("Invalid workflow data: {}", e))),
                );
            }
        };

        match store.archetypes().create(ArchetypeType::Workflow.as_str(), &dto.name, &definition).await {
            Ok(()) => {
                // Auto-sync to S3 if configured
                if let Err(e) = sync_item_to_s3_if_active(&state, "workflows", &dto.name, &dto).await {
                    tracing::warn!("Failed to sync workflow to S3: {}", e);
                }
                if let Some(broadcaster) = &state.broadcaster {
                    broadcaster.notify_tools_changed().await;
                }
                return (StatusCode::CREATED, Json(ApiResponse::success(dto)));
            }
            Err(crate::persistence::error::PersistenceError::Duplicate { .. }) => {
                return (
                    StatusCode::CONFLICT,
                    Json(ApiResponse::<WorkflowDto>::error("Workflow with this name already exists")),
                );
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<WorkflowDto>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let mut settings = state.settings.write().await;

    // Check for duplicate name
    if settings.workflows.iter().any(|w| w.name == dto.name) {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::<WorkflowDto>::error("Workflow with this name already exists")),
        );
    }

    let workflow = WorkflowConfig::from(dto.clone());
    settings.workflows.push(workflow);
    drop(settings);

    // Auto-sync to S3 if configured
    if let Err(e) = sync_item_to_s3_if_active(&state, "workflows", &dto.name, &dto).await {
        tracing::warn!("Failed to sync workflow to S3: {}", e);
    }

    // Workflows are exposed as tools, so notify about tool list change
    if let Some(broadcaster) = &state.broadcaster {
        broadcaster.notify_tools_changed().await;
    }

    (StatusCode::CREATED, Json(ApiResponse::success(dto)))
}

/// PUT /api/workflows/:name - Update a workflow
pub async fn update_workflow(
    State(state): State<ApiState>,
    Path(name): Path<String>,
    Json(dto): Json<WorkflowDto>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        let definition = match serde_json::to_value(&dto) {
            Ok(v) => v,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<WorkflowDto>::error(format!("Invalid workflow data: {}", e))),
                );
            }
        };

        match store.archetypes().update(ArchetypeType::Workflow.as_str(), &name, &definition, None).await {
            Ok(_) => {
                // Auto-sync to S3 if configured
                if let Err(e) = sync_item_to_s3_if_active(&state, "workflows", &dto.name, &dto).await {
                    tracing::warn!("Failed to sync workflow to S3: {}", e);
                }
                if let Some(broadcaster) = &state.broadcaster {
                    broadcaster.notify_tools_changed().await;
                }
                return (StatusCode::OK, Json(ApiResponse::success(dto)));
            }
            Err(crate::persistence::error::PersistenceError::NotFound { .. }) => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<WorkflowDto>::error("Workflow not found")));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<WorkflowDto>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let mut settings = state.settings.write().await;

    if let Some(workflow) = settings.workflows.iter_mut().find(|w| w.name == name) {
        workflow.name = dto.name.clone();
        workflow.description = dto.description.clone();
        workflow.tags = dto.tags.clone();
        workflow.input_schema = dto.input_schema.clone();
        workflow.steps = dto.steps.clone().into_iter().map(WorkflowStep::from).collect();
        workflow.on_error = dto.on_error.clone();
        drop(settings);

        // Auto-sync to S3 if configured
        if let Err(e) = sync_item_to_s3_if_active(&state, "workflows", &dto.name, &dto).await {
            tracing::warn!("Failed to sync workflow to S3: {}", e);
        }

        // Workflows are exposed as tools, so notify about tool list change
        if let Some(broadcaster) = &state.broadcaster {
            broadcaster.notify_tools_changed().await;
        }

        (StatusCode::OK, Json(ApiResponse::success(dto)))
    } else {
        (StatusCode::NOT_FOUND, Json(ApiResponse::<WorkflowDto>::error("Workflow not found")))
    }
}

/// DELETE /api/workflows/:name - Delete a workflow
pub async fn delete_workflow(
    State(state): State<ApiState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        match store.archetypes().delete(ArchetypeType::Workflow.as_str(), &name).await {
            Ok(true) => {
                // Auto-delete from S3 if configured
                if let Err(e) = delete_item_from_s3_if_active(&state, "workflows", &name).await {
                    tracing::warn!("Failed to delete workflow from S3: {}", e);
                }
                if let Some(broadcaster) = &state.broadcaster {
                    broadcaster.notify_tools_changed().await;
                }
                return (StatusCode::OK, Json(ApiResponse::<()>::ok()));
            }
            Ok(false) => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<()>::error("Workflow not found")));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<()>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let mut settings = state.settings.write().await;

    let initial_len = settings.workflows.len();
    settings.workflows.retain(|w| w.name != name);

    if settings.workflows.len() < initial_len {
        drop(settings);

        // Auto-delete from S3 if configured
        if let Err(e) = delete_item_from_s3_if_active(&state, "workflows", &name).await {
            tracing::warn!("Failed to delete workflow from S3: {}", e);
        }

        // Workflows are exposed as tools, so notify about tool list change
        if let Some(broadcaster) = &state.broadcaster {
            broadcaster.notify_tools_changed().await;
        }

        (StatusCode::OK, Json(ApiResponse::<()>::ok()))
    } else {
        (StatusCode::NOT_FOUND, Json(ApiResponse::<()>::error("Workflow not found")))
    }
}

// ============================================================================
// Resource Template CRUD Endpoints
// ============================================================================

/// GET /api/resource-templates - List all resource templates
pub async fn list_resource_templates(
    State(state): State<ApiState>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        match store.archetypes().list(ArchetypeType::ResourceTemplate.as_str()).await {
            Ok(templates) => {
                let dtos: Vec<ResourceTemplateDto> = templates
                    .into_iter()
                    .filter_map(|v| serde_json::from_value(v).ok())
                    .collect();
                return (StatusCode::OK, Json(ApiResponse::success(dtos)));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<Vec<ResourceTemplateDto>>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let settings = state.settings.read().await;
    let templates: Vec<ResourceTemplateDto> = settings
        .resource_templates
        .iter()
        .map(ResourceTemplateDto::from)
        .collect();
    (StatusCode::OK, Json(ApiResponse::success(templates)))
}

/// GET /api/resource-templates/:uri_template - Get a single resource template
pub async fn get_resource_template(
    State(state): State<ApiState>,
    Path(uri_template): Path<String>,
) -> impl IntoResponse {
    let decoded_uri = urlencoding::decode(&uri_template)
        .map(|s| s.into_owned())
        .unwrap_or(uri_template.clone());

    // Use database if available
    if let Some(store) = &state.data_store {
        match store.archetypes().get(ArchetypeType::ResourceTemplate.as_str(), &decoded_uri).await {
            Ok(Some(template)) => {
                match serde_json::from_value::<ResourceTemplateDto>(template) {
                    Ok(dto) => return (StatusCode::OK, Json(ApiResponse::success(dto))),
                    Err(e) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ApiResponse::<ResourceTemplateDto>::error(format!("Failed to parse resource template: {}", e))),
                        );
                    }
                }
            }
            Ok(None) => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<ResourceTemplateDto>::error("Resource template not found")));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<ResourceTemplateDto>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let settings = state.settings.read().await;
    if let Some(template) = settings
        .resource_templates
        .iter()
        .find(|r| r.uri_template == decoded_uri)
    {
        (
            StatusCode::OK,
            Json(ApiResponse::success(ResourceTemplateDto::from(template))),
        )
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<ResourceTemplateDto>::error("Resource template not found")),
        )
    }
}

/// POST /api/resource-templates - Create a new resource template
pub async fn create_resource_template(
    State(state): State<ApiState>,
    Json(dto): Json<ResourceTemplateDto>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        let definition = match serde_json::to_value(&dto) {
            Ok(v) => v,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<ResourceTemplateDto>::error(format!("Invalid resource template data: {}", e))),
                );
            }
        };

        // Use uri_template as the name
        match store.archetypes().create(ArchetypeType::ResourceTemplate.as_str(), &dto.uri_template, &definition).await {
            Ok(()) => {
                // CRITICAL: Also add to in-memory settings so MCP handlers see the new template
                {
                    let mut settings = state.settings.write().await;
                    settings.resource_templates.push(ResourceTemplateConfig::from(dto.clone()));
                }
                // Auto-sync to S3 if configured
                let safe_name = sanitize_uri_template_for_s3(&dto.uri_template);
                if let Err(e) = sync_item_to_s3_if_active(&state, "resource_templates", &safe_name, &dto).await {
                    tracing::warn!("Failed to sync resource template to S3: {}", e);
                }
                if let Some(broadcaster) = &state.broadcaster {
                    broadcaster.notify_resources_changed().await;
                    broadcaster.notify_tools_changed().await;
                }
                return (StatusCode::CREATED, Json(ApiResponse::success(dto)));
            }
            Err(crate::persistence::error::PersistenceError::Duplicate { .. }) => {
                return (
                    StatusCode::CONFLICT,
                    Json(ApiResponse::<ResourceTemplateDto>::error("Resource template with this URI template already exists")),
                );
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<ResourceTemplateDto>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let mut settings = state.settings.write().await;

    // Check for duplicate URI template
    if settings
        .resource_templates
        .iter()
        .any(|r| r.uri_template == dto.uri_template)
    {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::<ResourceTemplateDto>::error(
                "Resource template with this URI template already exists",
            )),
        );
    }

    let template = ResourceTemplateConfig::from(dto.clone());
    settings.resource_templates.push(template);
    drop(settings);

    // Auto-sync to S3 if configured
    let safe_name = sanitize_uri_template_for_s3(&dto.uri_template);
    if let Err(e) = sync_item_to_s3_if_active(&state, "resource_templates", &safe_name, &dto).await {
        tracing::warn!("Failed to sync resource template to S3: {}", e);
    }

    // Resource templates affect resource list and are also exposed as tools
    if let Some(broadcaster) = &state.broadcaster {
        broadcaster.notify_resources_changed().await;
        broadcaster.notify_tools_changed().await;
    }

    (StatusCode::CREATED, Json(ApiResponse::success(dto)))
}

/// PUT /api/resource-templates/:uri_template - Update a resource template
pub async fn update_resource_template(
    State(state): State<ApiState>,
    Path(uri_template): Path<String>,
    Json(dto): Json<ResourceTemplateDto>,
) -> impl IntoResponse {
    let decoded_uri = urlencoding::decode(&uri_template)
        .map(|s| s.into_owned())
        .unwrap_or(uri_template.clone());

    // Use database if available
    if let Some(store) = &state.data_store {
        let definition = match serde_json::to_value(&dto) {
            Ok(v) => v,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<ResourceTemplateDto>::error(format!("Invalid resource template data: {}", e))),
                );
            }
        };

        match store.archetypes().update(ArchetypeType::ResourceTemplate.as_str(), &decoded_uri, &definition, None).await {
            Ok(_) => {
                // CRITICAL: Also update in-memory settings so MCP handlers see the change
                {
                    let mut settings = state.settings.write().await;
                    if let Some(template) = settings.resource_templates.iter_mut().find(|r| r.uri_template == decoded_uri) {
                        template.name = dto.name.clone();
                        template.description = dto.description.clone();
                        template.mime_type = dto.mime_type.clone();
                        template.tags = dto.tags.clone();
                        template.input_schema = dto.input_schema.clone();
                        template.output_schema = dto.output_schema.clone();
                        template.content = dto.content.clone();
                        template.mock = dto.mock.clone();
                    }
                }
                // Auto-sync to S3 if configured
                let safe_name = sanitize_uri_template_for_s3(&dto.uri_template);
                if let Err(e) = sync_item_to_s3_if_active(&state, "resource_templates", &safe_name, &dto).await {
                    tracing::warn!("Failed to sync resource template to S3: {}", e);
                }
                if let Some(broadcaster) = &state.broadcaster {
                    broadcaster.notify_resources_changed().await;
                    broadcaster.notify_tools_changed().await;
                }
                return (StatusCode::OK, Json(ApiResponse::success(dto)));
            }
            Err(crate::persistence::error::PersistenceError::NotFound { .. }) => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<ResourceTemplateDto>::error("Resource template not found")));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<ResourceTemplateDto>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let mut settings = state.settings.write().await;

    if let Some(template) = settings
        .resource_templates
        .iter_mut()
        .find(|r| r.uri_template == decoded_uri)
    {
        template.name = dto.name.clone();
        template.description = dto.description.clone();
        template.mime_type = dto.mime_type.clone();
        template.tags = dto.tags.clone();
        template.input_schema = dto.input_schema.clone();
        template.output_schema = dto.output_schema.clone();
        template.content = dto.content.clone();
        template.mock = dto.mock.clone();
        drop(settings);

        // Auto-sync to S3 if configured
        let safe_name = sanitize_uri_template_for_s3(&dto.uri_template);
        if let Err(e) = sync_item_to_s3_if_active(&state, "resource_templates", &safe_name, &dto).await {
            tracing::warn!("Failed to sync resource template to S3: {}", e);
        }

        // Resource templates affect resource list and are also exposed as tools
        if let Some(broadcaster) = &state.broadcaster {
            broadcaster.notify_resources_changed().await;
            broadcaster.notify_tools_changed().await;
        }

        (StatusCode::OK, Json(ApiResponse::success(dto)))
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<ResourceTemplateDto>::error("Resource template not found")),
        )
    }
}

/// DELETE /api/resource-templates/:uri_template - Delete a resource template
pub async fn delete_resource_template(
    State(state): State<ApiState>,
    Path(uri_template): Path<String>,
) -> impl IntoResponse {
    let decoded_uri = urlencoding::decode(&uri_template)
        .map(|s| s.into_owned())
        .unwrap_or(uri_template.clone());

    // Use database if available
    if let Some(store) = &state.data_store {
        match store.archetypes().delete(ArchetypeType::ResourceTemplate.as_str(), &decoded_uri).await {
            Ok(true) => {
                // CRITICAL: Also remove from in-memory settings so MCP handlers don't see deleted template
                {
                    let mut settings = state.settings.write().await;
                    settings.resource_templates.retain(|r| r.uri_template != decoded_uri);
                }
                // Auto-delete from S3 if configured
                let safe_name = sanitize_uri_template_for_s3(&decoded_uri);
                if let Err(e) = delete_item_from_s3_if_active(&state, "resource_templates", &safe_name).await {
                    tracing::warn!("Failed to delete resource template from S3: {}", e);
                }
                if let Some(broadcaster) = &state.broadcaster {
                    broadcaster.notify_resources_changed().await;
                    broadcaster.notify_tools_changed().await;
                }
                return (StatusCode::OK, Json(ApiResponse::<()>::ok()));
            }
            Ok(false) => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<()>::error("Resource template not found")));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<()>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let mut settings = state.settings.write().await;

    let initial_len = settings.resource_templates.len();
    settings
        .resource_templates
        .retain(|r| r.uri_template != decoded_uri);

    if settings.resource_templates.len() < initial_len {
        drop(settings);

        // Auto-delete from S3 if configured
        let safe_name = sanitize_uri_template_for_s3(&decoded_uri);
        if let Err(e) = delete_item_from_s3_if_active(&state, "resource_templates", &safe_name).await {
            tracing::warn!("Failed to delete resource template from S3: {}", e);
        }

        // Resource templates affect resource list and are also exposed as tools
        if let Some(broadcaster) = &state.broadcaster {
            broadcaster.notify_resources_changed().await;
            broadcaster.notify_tools_changed().await;
        }

        (StatusCode::OK, Json(ApiResponse::<()>::ok()))
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<()>::error("Resource template not found")),
        )
    }
}

/// POST /api/resource-templates/:uri_template/test - Test a resource template with arguments
pub async fn test_resource_template(
    State(state): State<ApiState>,
    Path(uri_template): Path<String>,
    Json(req): Json<TestRequest>,
) -> impl IntoResponse {
    let start = std::time::Instant::now();
    let decoded_uri = urlencoding::decode(&uri_template)
        .map(|s| s.into_owned())
        .unwrap_or(uri_template);
    let settings = state.settings.read().await;

    // Find the resource template
    let template = match settings
        .resource_templates
        .iter()
        .find(|r| r.uri_template == decoded_uri)
    {
        Some(t) => t.clone(),
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<TestResult>::error("Resource template not found")),
            );
        }
    };
    drop(settings);

    // Resolve URI template with arguments
    let resolved_uri = resolve_uri_template(&template.uri_template, &req.args);

    // Generate resource content
    let output = if let Some(mock_config) = &template.mock {
        // Special handling for Static strategy: use content if available
        if matches!(mock_config.strategy, crate::config::MockStrategyType::Static) {
            if let Some(content) = &template.content {
                // Also resolve template variables in static content
                let resolved_content = resolve_uri_template(content, &req.args);
                json!({
                    "uri_template": template.uri_template,
                    "resolved_uri": resolved_uri,
                    "name": template.name,
                    "mime_type": template.mime_type,
                    "content": resolved_content
                })
            } else {
                // Fall through to generate() which returns null for Static
                match state.mock_strategy.generate(mock_config, Some(&req.args)).await {
                    Ok(result) => {
                        json!({
                            "uri_template": template.uri_template,
                            "resolved_uri": resolved_uri,
                            "name": template.name,
                            "mime_type": template.mime_type,
                            "content": result
                        })
                    }
                    Err(e) => {
                        let elapsed = start.elapsed().as_millis() as u64;
                        return (
                            StatusCode::OK,
                            Json(ApiResponse::success(TestResult {
                                output: Value::Null,
                                error: Some(format!("Mock strategy error: {}", e)),
                                execution_time_ms: elapsed,
                            })),
                        );
                    }
                }
            }
        } else {
            match state.mock_strategy.generate(mock_config, Some(&req.args)).await {
                Ok(result) => {
                    json!({
                        "uri_template": template.uri_template,
                        "resolved_uri": resolved_uri,
                        "name": template.name,
                        "mime_type": template.mime_type,
                        "content": result
                    })
                }
                Err(e) => {
                    let elapsed = start.elapsed().as_millis() as u64;
                    return (
                        StatusCode::OK,
                        Json(ApiResponse::success(TestResult {
                            output: Value::Null,
                            error: Some(format!("Mock strategy error: {}", e)),
                            execution_time_ms: elapsed,
                        })),
                    );
                }
            }
        }
    } else if let Some(content) = &template.content {
        // Also resolve template variables in static content
        let resolved_content = resolve_uri_template(content, &req.args);
        json!({
            "uri_template": template.uri_template,
            "resolved_uri": resolved_uri,
            "name": template.name,
            "mime_type": template.mime_type,
            "content": resolved_content
        })
    } else {
        json!({
            "uri_template": template.uri_template,
            "resolved_uri": resolved_uri,
            "name": template.name,
            "mime_type": template.mime_type,
            "content": ""
        })
    };

    let elapsed = start.elapsed().as_millis() as u64;
    (
        StatusCode::OK,
        Json(ApiResponse::success(TestResult {
            output,
            error: None,
            execution_time_ms: elapsed,
        })),
    )
}

/// Resolve a URI template by substituting {variable} placeholders with argument values
fn resolve_uri_template(uri_template: &str, args: &Value) -> String {
    let mut resolved = uri_template.to_string();
    if let Some(obj) = args.as_object() {
        for (key, value) in obj {
            let placeholder = format!("{{{}}}", key);
            let replacement = match value {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                Value::Bool(b) => b.to_string(),
                _ => value.to_string(),
            };
            resolved = resolved.replace(&placeholder, &replacement);
        }
    }
    resolved
}

// ============================================================================
// State Management Endpoints
// ============================================================================

/// GET /api/state - Get all stateful mock state
pub async fn get_state(
    State(state): State<ApiState>,
) -> impl IntoResponse {
    let all_state = state.state_manager.get_all().await;
    (StatusCode::OK, Json(ApiResponse::success(all_state)))
}

/// DELETE /api/state - Reset all stateful mock state
pub async fn reset_state(
    State(state): State<ApiState>,
) -> impl IntoResponse {
    state.state_manager.clear().await;
    (StatusCode::OK, Json(ApiResponse::<()>::ok()))
}

/// DELETE /api/state/:key - Delete a specific state key
pub async fn delete_state_key(
    State(state): State<ApiState>,
    Path(key): Path<String>,
) -> impl IntoResponse {
    state.state_manager.delete(&key).await;
    (StatusCode::OK, Json(ApiResponse::<()>::ok()))
}

// ============================================================================
// Config Persistence Endpoints
// ============================================================================

/// POST /api/config/save-disk - Save current configuration to config file
/// Supports optimistic locking via expected_version in request body
pub async fn save_config_to_disk(
    State(state): State<ApiState>,
    body: Option<Json<SaveConfigRequest>>,
) -> impl IntoResponse {
    let expected_version = body.and_then(|b| b.expected_version);

    // Acquire write lock for atomic version check and increment
    let mut settings_guard = state.settings.write().await;

    // Check version if expected_version is provided (optimistic locking)
    if let Some(expected) = expected_version {
        if let Err(conflict) = settings_guard.check_version(expected) {
            let response = VersionConflictResponse {
                expected_version: conflict.expected,
                current_version: conflict.actual,
                message: format!(
                    "Configuration was modified by another process. Expected version {}, but current version is {}. Please refresh and try again.",
                    conflict.expected, conflict.actual
                ),
            };
            return (
                StatusCode::CONFLICT,
                Json(ApiResponse::success(serde_json::to_value(response).unwrap()))
            );
        }
    }

    // Increment version before saving
    settings_guard.increment_version();
    let new_version = settings_guard.version;

    let config_path = settings_guard
        .config_path
        .clone()
        .unwrap_or_else(|| std::path::PathBuf::from("metis.toml"));

    // Create a copy of settings for serialization (with new version)
    let mut settings: Settings = serde_json::from_value(
        serde_json::to_value(&*settings_guard).unwrap()
    ).unwrap();
    drop(settings_guard);

    // Build SecretsConfig from in-memory secrets store, encrypting if passphrase is available
    let passphrase = state.passphrase.get().await;
    match build_secrets_config(&state.secrets, passphrase.as_deref()).await {
        Ok(secrets_config) => {
            settings.secrets = secrets_config;
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<Value>::error(format!("Failed to build secrets config: {}", e)))
            );
        }
    }

    // Serialize settings to TOML
    let toml_content = match toml::to_string_pretty(&settings) {
        Ok(content) => content,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<Value>::error(format!("Failed to serialize config: {}", e)))
            );
        }
    };

    // Write to config file
    if let Err(e) = std::fs::write(&config_path, toml_content) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<Value>::error(format!("Failed to write {}: {}", config_path.display(), e)))
        );
    }

    (StatusCode::OK, Json(ApiResponse::success(serde_json::to_value(SaveConfigResponse { new_version }).unwrap())))
}

/// Build SecretsConfig from the in-memory secrets store
/// If passphrase is provided, secrets are encrypted; otherwise stored as plain text
async fn build_secrets_config(
    secrets: &SharedSecretsStore,
    passphrase: Option<&str>,
) -> Result<SecretsConfig, String> {
    let mut config = SecretsConfig::default();

    // Helper to get and optionally encrypt a secret
    let encrypt_secret = |value: String, passphrase: Option<&str>| -> Result<String, String> {
        match passphrase {
            Some(pass) => encryption::encrypt(&value, pass)
                .map_err(|e| format!("Encryption failed: {}", e)),
            None => Ok(value),
        }
    };

    // Get each secret from the store and optionally encrypt it
    if let Some(value) = secrets.get(keys::OPENAI_API_KEY).await {
        config.openai_api_key = Some(encrypt_secret(value, passphrase)?);
    }
    if let Some(value) = secrets.get(keys::ANTHROPIC_API_KEY).await {
        config.anthropic_api_key = Some(encrypt_secret(value, passphrase)?);
    }
    if let Some(value) = secrets.get(keys::GEMINI_API_KEY).await {
        config.gemini_api_key = Some(encrypt_secret(value, passphrase)?);
    }
    if let Some(value) = secrets.get(keys::AWS_ACCESS_KEY_ID).await {
        config.aws_access_key_id = Some(encrypt_secret(value, passphrase)?);
    }
    if let Some(value) = secrets.get(keys::AWS_SECRET_ACCESS_KEY).await {
        config.aws_secret_access_key = Some(encrypt_secret(value, passphrase)?);
    }
    if let Some(value) = secrets.get(keys::AWS_REGION).await {
        config.aws_region = Some(encrypt_secret(value, passphrase)?);
    }

    Ok(config)
}

/// Upload a single item to S3 as a YAML file in a subdirectory
async fn upload_item_to_s3<T: Serialize>(
    client: &aws_sdk_s3::Client,
    bucket: &str,
    prefix: &str,
    subdir: &str,
    name: &str,
    item: &T,
) -> Result<(), String> {
    let yaml_content = serde_yaml::to_string(item)
        .map_err(|e| format!("Failed to serialize {} to YAML: {}", name, e))?;

    let key = format!("{}{}/{}.yaml", prefix, subdir, name);

    client
        .put_object()
        .bucket(bucket)
        .key(&key)
        .body(yaml_content.into_bytes().into())
        .content_type("application/yaml")
        .send()
        .await
        .map_err(|e| format!("Failed to upload {} to S3: {}", key, e))?;

    tracing::info!("Uploaded {} to S3: {}", name, key);
    Ok(())
}

/// Sync a single item to S3 if S3 backend is configured and active.
/// This is called automatically when items are created or updated through the UI.
/// Returns Ok(true) if synced, Ok(false) if S3 not active, Err on failure.
pub async fn sync_item_to_s3_if_active<T: Serialize>(
    state: &ApiState,
    subdir: &str,
    name: &str,
    item: &T,
) -> Result<bool, String> {
    let settings = state.settings.read().await;

    // Check if S3 is configured and active
    let s3_config = match &settings.s3 {
        Some(cfg) if cfg.is_active() => cfg.clone(),
        _ => return Ok(false), // S3 not active, nothing to do
    };

    drop(settings); // Release the lock before async operations

    // Build S3 client
    let sdk_config = build_s3_config(&s3_config, &state.secrets)
        .await
        .map_err(|e| format!("Failed to configure S3 client: {}", e))?;
    let client = aws_sdk_s3::Client::new(&sdk_config);

    let bucket = s3_config.bucket.as_ref().ok_or("S3 bucket not configured")?;
    let prefix = s3_config.get_prefix();

    // Upload the item
    upload_item_to_s3(&client, bucket, &prefix, subdir, name, item).await?;

    tracing::info!("Auto-synced {} '{}' to S3", subdir, name);
    Ok(true)
}

/// Delete a single item from S3 if S3 backend is configured and active.
/// This is called automatically when items are deleted through the UI.
/// Returns Ok(true) if deleted, Ok(false) if S3 not active, Err on failure.
pub async fn delete_item_from_s3_if_active(
    state: &ApiState,
    subdir: &str,
    name: &str,
) -> Result<bool, String> {
    let settings = state.settings.read().await;

    // Check if S3 is configured and active
    let s3_config = match &settings.s3 {
        Some(cfg) if cfg.is_active() => cfg.clone(),
        _ => return Ok(false), // S3 not active, nothing to do
    };

    drop(settings); // Release the lock before async operations

    // Build S3 client
    let sdk_config = build_s3_config(&s3_config, &state.secrets)
        .await
        .map_err(|e| format!("Failed to configure S3 client: {}", e))?;
    let client = aws_sdk_s3::Client::new(&sdk_config);

    let bucket = s3_config.bucket.as_ref().ok_or("S3 bucket not configured")?;
    let prefix = s3_config.get_prefix();
    let key = format!("{}{}/{}.yaml", prefix, subdir, name);

    // Delete the item from S3
    client
        .delete_object()
        .bucket(bucket)
        .key(&key)
        .send()
        .await
        .map_err(|e| format!("Failed to delete {} from S3: {}", key, e))?;

    tracing::info!("Auto-deleted {} '{}' from S3: {}", subdir, name, key);
    Ok(true)
}

/// Sanitize a URI for use as an S3 key filename.
/// Replaces characters that are problematic in S3 keys.
fn sanitize_uri_for_s3(uri: &str) -> String {
    uri.replace(['/', ':', '?', '#', ' '], "_")
}

/// Sanitize a URI template for use as an S3 key filename.
/// Replaces characters that are problematic in S3 keys.
fn sanitize_uri_template_for_s3(uri_template: &str) -> String {
    uri_template.replace(['/', ':', '?', '#', ' ', '{', '}'], "_")
}

/// POST /api/config/save-s3 - Save current configuration to S3
/// Supports optimistic locking via expected_version in request body
/// Also uploads individual schemas, tools, resources, prompts, agents, and workflows
/// to their respective subdirectories for better organization and hot-reload support
pub async fn save_config_to_s3(
    State(state): State<ApiState>,
    body: Option<Json<SaveConfigRequest>>,
) -> impl IntoResponse {
    let expected_version = body.and_then(|b| b.expected_version);

    // Acquire write lock for atomic version check and increment
    let mut settings_guard = state.settings.write().await;

    // Check version if expected_version is provided (optimistic locking)
    if let Some(expected) = expected_version {
        if let Err(conflict) = settings_guard.check_version(expected) {
            let response = VersionConflictResponse {
                expected_version: conflict.expected,
                current_version: conflict.actual,
                message: format!(
                    "Configuration was modified by another process. Expected version {}, but current version is {}. Please refresh and try again.",
                    conflict.expected, conflict.actual
                ),
            };
            return (
                StatusCode::CONFLICT,
                Json(ApiResponse::success(serde_json::to_value(response).unwrap()))
            );
        }
    }

    // Increment version before saving
    settings_guard.increment_version();
    let new_version = settings_guard.version;

    // Create a copy of settings for serialization (with new version)
    let mut settings: Settings = serde_json::from_value(
        serde_json::to_value(&*settings_guard).unwrap()
    ).unwrap();
    drop(settings_guard);

    // Check if S3 is configured
    let s3_config = match &settings.s3 {
        Some(cfg) if cfg.is_active() => cfg.clone(),
        Some(cfg) => {
            // S3 exists but not active - provide specific feedback
            let mut issues = Vec::new();
            if !cfg.enabled {
                issues.push("S3 is not enabled");
            }
            if cfg.bucket.is_none() || cfg.bucket.as_ref().map(|b| b.is_empty()).unwrap_or(true) {
                issues.push("bucket name is required");
            }
            let msg = format!(
                "S3 configuration incomplete: {}. Please update S3 settings and save to disk first.",
                issues.join(", ")
            );
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<Value>::error(msg))
            );
        }
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<Value>::error(
                    "S3 is not configured. Please configure S3 settings (enable S3, set bucket name, region) and save to disk first."
                ))
            );
        }
    };

    // Build SecretsConfig from in-memory secrets store, encrypting if passphrase is available
    let passphrase = state.passphrase.get().await;
    match build_secrets_config(&state.secrets, passphrase.as_deref()).await {
        Ok(secrets_config) => {
            settings.secrets = secrets_config;
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<Value>::error(format!("Failed to build secrets config: {}", e)))
            );
        }
    }

    // Serialize settings to TOML
    let toml_content = match toml::to_string_pretty(&settings) {
        Ok(content) => content,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<Value>::error(format!("Failed to serialize config: {}", e)))
            );
        }
    };

    // Build S3 client (uses credentials from UI secrets store with env var fallback)
    let sdk_config = match build_s3_config(&s3_config, &state.secrets).await {
        Ok(cfg) => cfg,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<Value>::error(format!("Failed to configure S3 client: {}", e)))
            );
        }
    };
    let client = aws_sdk_s3::Client::new(&sdk_config);

    // Upload to S3
    let bucket = s3_config.bucket.as_ref().unwrap();
    let key = format!("{}metis.toml", s3_config.get_prefix());

    // First, upload the main config file
    if let Err(e) = client
        .put_object()
        .bucket(bucket)
        .key(&key)
        .body(toml_content.into_bytes().into())
        .content_type("application/toml")
        .send()
        .await
    {
        // Extract full error chain for better diagnostics
        let error_msg = format!("{}", e);
        let debug_msg = format!("{:?}", e);

        // Try to get the underlying service error for more details
        let service_error_details = if let Some(service_err) = e.as_service_error() {
            format!(" Service error: {:?}", service_err)
        } else {
            String::new()
        };

        // Log the full error for debugging
        tracing::error!("S3 upload error: {} | Debug: {} | Service: {}", error_msg, debug_msg, service_error_details);

        // Provide more helpful error messages for common S3 issues
        let full_error = format!("{}{}", error_msg, service_error_details);
        let helpful_msg = if full_error.contains("credentials") || full_error.contains("Credentials") || full_error.contains("NoCredentialsError") {
            format!(
                "S3 credentials error: {}. Set AWS credentials in the UI Secrets section (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY) or as environment variables.",
                error_msg
            )
        } else if full_error.contains("NoSuchBucket") {
            format!(
                "S3 bucket '{}' does not exist or you don't have access to it.",
                bucket
            )
        } else if full_error.contains("AccessDenied") || full_error.contains("Forbidden") {
            format!(
                "S3 access denied to bucket '{}'. Check your credentials and bucket permissions.",
                bucket
            )
        } else if full_error.contains("InvalidAccessKeyId") {
            "Invalid AWS access key. Check your AWS_ACCESS_KEY_ID in UI Secrets or environment.".to_string()
        } else if full_error.contains("SignatureDoesNotMatch") {
            "AWS signature mismatch. Check your AWS_SECRET_ACCESS_KEY in UI Secrets or environment.".to_string()
        } else if s3_config.region.is_none() && s3_config.endpoint.is_some() {
            format!(
                "S3 error: {}. Note: You're using a custom endpoint but no region is set. For S3-compatible services like Wasabi or MinIO, you may need to specify a region (e.g., 'us-east-1').",
                full_error
            )
        } else {
            // Include debug info for unknown errors
            format!(
                "S3 upload failed: {}.{} Config: bucket='{}', region={:?}, endpoint={:?}. Check server logs for more details.",
                error_msg,
                service_error_details,
                bucket,
                s3_config.region,
                s3_config.endpoint
            )
        };
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<Value>::error(helpful_msg))
        );
    }

    tracing::info!("Uploaded main config to S3: {}", key);

    // Upload individual items to S3 subdirectories for hot-reload support
    let prefix = s3_config.get_prefix();
    let mut upload_errors: Vec<String> = Vec::new();

    // Upload schemas
    for schema in &settings.schemas {
        if let Err(e) = upload_item_to_s3(&client, bucket, &prefix, "schemas", &schema.name, schema).await {
            upload_errors.push(e);
        }
    }

    // Upload tools
    for tool in &settings.tools {
        if let Err(e) = upload_item_to_s3(&client, bucket, &prefix, "tools", &tool.name, tool).await {
            upload_errors.push(e);
        }
    }

    // Upload resources
    for resource in &settings.resources {
        // Use a sanitized version of the URI as the filename
        let safe_name = resource.uri.replace(['/', ':', '?', '#', ' '], "_");
        if let Err(e) = upload_item_to_s3(&client, bucket, &prefix, "resources", &safe_name, resource).await {
            upload_errors.push(e);
        }
    }

    // Upload resource templates
    for template in &settings.resource_templates {
        // Use a sanitized version of the URI template as the filename
        let safe_name = template.uri_template.replace(['/', ':', '?', '#', ' ', '{', '}'], "_");
        if let Err(e) = upload_item_to_s3(&client, bucket, &prefix, "resource_templates", &safe_name, template).await {
            upload_errors.push(e);
        }
    }

    // Upload prompts
    for prompt in &settings.prompts {
        if let Err(e) = upload_item_to_s3(&client, bucket, &prefix, "prompts", &prompt.name, prompt).await {
            upload_errors.push(e);
        }
    }

    // Upload agents
    for agent in &settings.agents {
        if let Err(e) = upload_item_to_s3(&client, bucket, &prefix, "agents", &agent.name, agent).await {
            upload_errors.push(e);
        }
    }

    // Upload workflows
    for workflow in &settings.workflows {
        if let Err(e) = upload_item_to_s3(&client, bucket, &prefix, "workflows", &workflow.name, workflow).await {
            upload_errors.push(e);
        }
    }

    // Upload data lakes
    for data_lake in &settings.data_lakes {
        if let Err(e) = upload_item_to_s3(&client, bucket, &prefix, "data_lakes", &data_lake.name, data_lake).await {
            upload_errors.push(e);
        }
    }

    // Log any upload errors but don't fail the whole operation
    if !upload_errors.is_empty() {
        tracing::warn!("Some individual item uploads failed: {:?}", upload_errors);
    }

    let total_items = settings.schemas.len() + settings.tools.len() + settings.resources.len()
        + settings.resource_templates.len() + settings.prompts.len()
        + settings.agents.len() + settings.workflows.len() + settings.data_lakes.len();
    let successful_items = total_items - upload_errors.len();
    tracing::info!("Uploaded {}/{} individual items to S3", successful_items, total_items);

    (StatusCode::OK, Json(ApiResponse::success(serde_json::to_value(SaveConfigResponse { new_version }).unwrap())))
}

/// GET /api/config/export - Export current configuration as JSON for browser download
pub async fn export_config(
    State(state): State<ApiState>,
) -> impl IntoResponse {
    let settings = state.settings.read().await;

    // Serialize settings to JSON
    match serde_json::to_value(&*settings) {
        Ok(json_value) => (StatusCode::OK, Json(ApiResponse::success(json_value))),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<Value>::error(format!("Failed to serialize config: {}", e)))
        ),
    }
}

/// POST /api/config/import - Import configuration from JSON
pub async fn import_config(
    State(state): State<ApiState>,
    Json(new_settings): Json<Settings>,
) -> impl IntoResponse {
    let mut settings = state.settings.write().await;

    // Replace the current settings with the imported ones
    *settings = new_settings;

    (StatusCode::OK, Json(ApiResponse::<()>::ok()))
}

/// Response for merge operation showing what was added
#[derive(Serialize)]
pub struct MergeResult {
    pub resources_added: usize,
    pub resource_templates_added: usize,
    pub tools_added: usize,
    pub prompts_added: usize,
    pub workflows_added: usize,
    pub agents_added: usize,
    pub orchestrations_added: usize,
    pub mcp_servers_added: usize,
}

/// POST /api/config/merge - Merge configuration from JSON, only adding new elements
pub async fn merge_config(
    State(state): State<ApiState>,
    Json(new_settings): Json<Settings>,
) -> impl IntoResponse {
    let mut settings = state.settings.write().await;
    let mut result = MergeResult {
        resources_added: 0,
        resource_templates_added: 0,
        tools_added: 0,
        prompts_added: 0,
        workflows_added: 0,
        agents_added: 0,
        orchestrations_added: 0,
        mcp_servers_added: 0,
    };

    // Merge resources (keyed by uri)
    for resource in new_settings.resources {
        if !settings.resources.iter().any(|r| r.uri == resource.uri) {
            settings.resources.push(resource);
            result.resources_added += 1;
        }
    }

    // Merge resource templates (keyed by uri_template)
    for template in new_settings.resource_templates {
        if !settings.resource_templates.iter().any(|t| t.uri_template == template.uri_template) {
            settings.resource_templates.push(template);
            result.resource_templates_added += 1;
        }
    }

    // Merge tools (keyed by name)
    for tool in new_settings.tools {
        if !settings.tools.iter().any(|t| t.name == tool.name) {
            settings.tools.push(tool);
            result.tools_added += 1;
        }
    }

    // Merge prompts (keyed by name)
    for prompt in new_settings.prompts {
        if !settings.prompts.iter().any(|p| p.name == prompt.name) {
            settings.prompts.push(prompt);
            result.prompts_added += 1;
        }
    }

    // Merge workflows (keyed by name)
    for workflow in new_settings.workflows {
        if !settings.workflows.iter().any(|w| w.name == workflow.name) {
            settings.workflows.push(workflow);
            result.workflows_added += 1;
        }
    }

    // Merge agents (keyed by name)
    for agent in new_settings.agents {
        if !settings.agents.iter().any(|a| a.name == agent.name) {
            settings.agents.push(agent);
            result.agents_added += 1;
        }
    }

    // Merge orchestrations (keyed by name)
    for orchestration in new_settings.orchestrations {
        if !settings.orchestrations.iter().any(|o| o.name == orchestration.name) {
            settings.orchestrations.push(orchestration);
            result.orchestrations_added += 1;
        }
    }

    // Merge MCP servers (keyed by name)
    for mcp_server in new_settings.mcp_servers {
        if !settings.mcp_servers.iter().any(|m| m.name == mcp_server.name) {
            settings.mcp_servers.push(mcp_server);
            result.mcp_servers_added += 1;
        }
    }

    (StatusCode::OK, Json(ApiResponse::success(result)))
}

/// Build AWS SDK configuration for S3 operations
/// Uses credentials from secrets store (UI) with fallback to environment variables
async fn build_s3_config(
    config: &crate::config::s3::S3Config,
    secrets: &SharedSecretsStore,
) -> anyhow::Result<aws_config::SdkConfig> {
    use aws_config::BehaviorVersion;
    use crate::adapters::secrets::keys;

    // Check for credentials in secrets store (UI) first, then environment
    let access_key = secrets.get_or_env(keys::AWS_ACCESS_KEY_ID).await;
    let secret_key = secrets.get_or_env(keys::AWS_SECRET_ACCESS_KEY).await;

    // If we have explicit credentials, use no_credentials() to skip the default
    // credential chain (which includes EC2 IMDS that causes timeouts outside AWS)
    if let (Some(access_key), Some(secret_key)) = (access_key, secret_key) {
        let credentials = aws_sdk_s3::config::Credentials::new(
            access_key,
            secret_key,
            None, // session token
            None, // expiry
            "metis-secrets-store",
        );

        let mut loader = aws_config::defaults(BehaviorVersion::latest())
            .no_credentials()  // Skip default credential chain (including IMDS)
            .credentials_provider(credentials);

        if let Some(region) = &config.region {
            loader = loader.region(aws_config::Region::new(region.clone()));
        }

        if let Some(endpoint) = &config.endpoint {
            loader = loader.endpoint_url(endpoint);
        }

        tracing::debug!("Using explicit AWS credentials (IMDS disabled)");
        Ok(loader.load().await)
    } else {
        // No explicit credentials - use default credential chain
        // This will try environment vars, shared config, IMDS, etc.
        let mut loader = aws_config::defaults(BehaviorVersion::latest());

        if let Some(region) = &config.region {
            loader = loader.region(aws_config::Region::new(region.clone()));
        }

        if let Some(endpoint) = &config.endpoint {
            loader = loader.endpoint_url(endpoint);
        }

        tracing::debug!("No explicit AWS credentials found, using default credential chain");
        Ok(loader.load().await)
    }
}

// ============================================================================
// Test Endpoints - Execute tools, resources, prompts, workflows
// ============================================================================

/// Request body for test endpoints
#[derive(Deserialize)]
pub struct TestRequest {
    #[serde(default)]
    pub args: Value,
    /// Optional session ID for multi-turn conversations
    #[serde(default)]
    pub session_id: Option<String>,
}

/// Response for test endpoints
#[derive(Serialize)]
pub struct TestResult {
    pub output: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

/// POST /api/tools/:name/test - Execute a tool with test inputs
pub async fn test_tool(
    State(state): State<ApiState>,
    Path(name): Path<String>,
    Json(req): Json<TestRequest>,
) -> impl IntoResponse {
    let start = std::time::Instant::now();
    let settings = state.settings.read().await;

    // Find the tool
    let tool = match settings.tools.iter().find(|t| t.name == name) {
        Some(t) => t.clone(),
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<TestResult>::error("Tool not found")),
            );
        }
    };
    drop(settings);

    // Execute the mock strategy
    let output = if let Some(mock_config) = &tool.mock {
        // Special handling for Static strategy: use static_response if available
        if matches!(mock_config.strategy, crate::config::MockStrategyType::Static) {
            if let Some(static_response) = &tool.static_response {
                static_response.clone()
            } else {
                // Fall through to generate() which returns null for Static
                match state.mock_strategy.generate(mock_config, Some(&req.args)).await {
                    Ok(result) => result,
                    Err(e) => {
                        let elapsed = start.elapsed().as_millis() as u64;
                        return (
                            StatusCode::OK,
                            Json(ApiResponse::success(TestResult {
                                output: Value::Null,
                                error: Some(format!("Mock strategy error: {}", e)),
                                execution_time_ms: elapsed,
                            })),
                        );
                    }
                }
            }
        } else {
            match state.mock_strategy.generate(mock_config, Some(&req.args)).await {
                Ok(result) => result,
                Err(e) => {
                    let elapsed = start.elapsed().as_millis() as u64;
                    return (
                        StatusCode::OK,
                        Json(ApiResponse::success(TestResult {
                            output: Value::Null,
                            error: Some(format!("Mock strategy error: {}", e)),
                            execution_time_ms: elapsed,
                        })),
                    );
                }
            }
        }
    } else if let Some(static_response) = &tool.static_response {
        static_response.clone()
    } else {
        Value::Null
    };

    let elapsed = start.elapsed().as_millis() as u64;
    (
        StatusCode::OK,
        Json(ApiResponse::success(TestResult {
            output,
            error: None,
            execution_time_ms: elapsed,
        })),
    )
}

/// POST /api/resources/:uri/test - Read a resource and get its content
pub async fn test_resource(
    State(state): State<ApiState>,
    Path(uri): Path<String>,
) -> impl IntoResponse {
    let start = std::time::Instant::now();
    let decoded_uri = urlencoding::decode(&uri).map(|s| s.into_owned()).unwrap_or(uri);
    let settings = state.settings.read().await;

    // Find the resource
    let resource = match settings.resources.iter().find(|r| r.uri == decoded_uri) {
        Some(r) => r.clone(),
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<TestResult>::error("Resource not found")),
            );
        }
    };
    drop(settings);

    // Generate resource content
    let output = if let Some(mock_config) = &resource.mock {
        // Special handling for Static strategy: use content if available
        if matches!(mock_config.strategy, crate::config::MockStrategyType::Static) {
            if let Some(content) = &resource.content {
                json!({
                    "uri": resource.uri,
                    "name": resource.name,
                    "mime_type": resource.mime_type,
                    "content": content
                })
            } else {
                // Fall through to generate() which returns null for Static
                match state.mock_strategy.generate(mock_config, None).await {
                    Ok(result) => {
                        json!({
                            "uri": resource.uri,
                            "name": resource.name,
                            "mime_type": resource.mime_type,
                            "content": result
                        })
                    }
                    Err(e) => {
                        let elapsed = start.elapsed().as_millis() as u64;
                        return (
                            StatusCode::OK,
                            Json(ApiResponse::success(TestResult {
                                output: Value::Null,
                                error: Some(format!("Mock strategy error: {}", e)),
                                execution_time_ms: elapsed,
                            })),
                        );
                    }
                }
            }
        } else {
            match state.mock_strategy.generate(mock_config, None).await {
                Ok(result) => {
                    json!({
                        "uri": resource.uri,
                        "name": resource.name,
                        "mime_type": resource.mime_type,
                        "content": result
                    })
                }
                Err(e) => {
                    let elapsed = start.elapsed().as_millis() as u64;
                    return (
                        StatusCode::OK,
                        Json(ApiResponse::success(TestResult {
                            output: Value::Null,
                            error: Some(format!("Mock strategy error: {}", e)),
                            execution_time_ms: elapsed,
                        })),
                    );
                }
            }
        }
    } else if let Some(content) = &resource.content {
        json!({
            "uri": resource.uri,
            "name": resource.name,
            "mime_type": resource.mime_type,
            "content": content
        })
    } else {
        json!({
            "uri": resource.uri,
            "name": resource.name,
            "mime_type": resource.mime_type,
            "content": ""
        })
    };

    let elapsed = start.elapsed().as_millis() as u64;
    (
        StatusCode::OK,
        Json(ApiResponse::success(TestResult {
            output,
            error: None,
            execution_time_ms: elapsed,
        })),
    )
}

/// POST /api/prompts/:name/test - Get prompt messages with arguments
pub async fn test_prompt(
    State(state): State<ApiState>,
    Path(name): Path<String>,
    Json(req): Json<TestRequest>,
) -> impl IntoResponse {
    let start = std::time::Instant::now();
    let settings = state.settings.read().await;

    // Find the prompt
    let prompt = match settings.prompts.iter().find(|p| p.name == name) {
        Some(p) => p.clone(),
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<TestResult>::error("Prompt not found")),
            );
        }
    };
    drop(settings);

    // Build prompt output with arguments substituted in messages
    let messages: Vec<Value> = prompt.messages.as_ref().map(|msgs| {
        msgs.iter().map(|m| {
            let mut content = m.content.clone();
            // Simple variable substitution from args
            if let Some(args_obj) = req.args.as_object() {
                for (key, value) in args_obj {
                    let placeholder = format!("{{{{{}}}}}", key);
                    if let Some(val_str) = value.as_str() {
                        content = content.replace(&placeholder, val_str);
                    } else {
                        content = content.replace(&placeholder, &value.to_string());
                    }
                }
            }
            json!({
                "role": m.role,
                "content": content
            })
        }).collect()
    }).unwrap_or_default();

    let output = json!({
        "name": prompt.name,
        "description": prompt.description,
        "messages": messages
    });

    let elapsed = start.elapsed().as_millis() as u64;
    (
        StatusCode::OK,
        Json(ApiResponse::success(TestResult {
            output,
            error: None,
            execution_time_ms: elapsed,
        })),
    )
}

/// POST /api/workflows/:name/test - Execute a workflow with test inputs
pub async fn test_workflow(
    State(state): State<ApiState>,
    Path(name): Path<String>,
    Json(req): Json<TestRequest>,
) -> impl IntoResponse {
    let start = std::time::Instant::now();
    let settings = state.settings.read().await;

    // Find the workflow
    let workflow = match settings.workflows.iter().find(|w| w.name == name) {
        Some(w) => w.clone(),
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<TestResult>::error("Workflow not found")),
            );
        }
    };
    drop(settings);

    // Get agent handler (prefer test_agent_handler if available, fallback to regular agent_handler)
    let agent_handler = {
        let test_handler = state.test_agent_handler.read().await;
        test_handler.clone().or_else(|| state.agent_handler.clone())
    };

    // Create a tool handler for testing (uses mock strategies and optionally agents)
    let tool_handler = Arc::new(TestToolHandler::new(
        state.settings.clone(),
        state.mock_strategy.clone(),
        agent_handler,
    ));

    // Create workflow engine
    let workflow_engine = WorkflowEngine::new(tool_handler);

    // Execute the workflow
    match workflow_engine.execute(&workflow, req.args).await {
        Ok(output) => {
            let elapsed = start.elapsed().as_millis() as u64;
            (
                StatusCode::OK,
                Json(ApiResponse::success(TestResult {
                    output,
                    error: None,
                    execution_time_ms: elapsed,
                })),
            )
        }
        Err(e) => {
            let elapsed = start.elapsed().as_millis() as u64;
            (
                StatusCode::OK,
                Json(ApiResponse::success(TestResult {
                    output: Value::Null,
                    error: Some(format!("Workflow execution error: {}", e)),
                    execution_time_ms: elapsed,
                })),
            )
        }
    }
}

// ============================================================================
// Agent DTOs
// ============================================================================

/// Data Transfer Object for Agent configuration
#[derive(Serialize, Deserialize, Clone)]
pub struct AgentDto {
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
    pub llm: LlmProviderConfigDto,
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
    pub memory: MemoryConfigDto,
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

#[derive(Serialize, Deserialize, Clone)]
pub struct LlmProviderConfigDto {
    pub provider: LlmProviderType,
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

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct MemoryConfigDto {
    #[serde(default)]
    pub backend: crate::agents::config::MemoryBackend,
    #[serde(default)]
    pub strategy: crate::agents::config::MemoryStrategy,
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

impl From<&AgentConfig> for AgentDto {
    fn from(a: &AgentConfig) -> Self {
        Self {
            name: a.name.clone(),
            description: a.description.clone(),
            tags: a.tags.clone(),
            agent_type: a.agent_type,
            input_schema: a.input_schema.clone(),
            output_schema: a.output_schema.clone(),
            llm: LlmProviderConfigDto {
                provider: a.llm.provider,
                model: a.llm.model.clone(),
                api_key_env: a.llm.api_key_env.clone(),
                base_url: a.llm.base_url.clone(),
                temperature: a.llm.temperature,
                max_tokens: a.llm.max_tokens,
                stream: a.llm.stream,
            },
            system_prompt: a.system_prompt.clone(),
            prompt_template: a.prompt_template.clone(),
            available_tools: a.available_tools.clone(),
            mcp_tools: a.mcp_tools.clone(),
            agent_tools: a.agent_tools.clone(),
            available_resources: a.available_resources.clone(),
            available_resource_templates: a.available_resource_templates.clone(),
            memory: MemoryConfigDto {
                backend: a.memory.backend,
                strategy: a.memory.strategy.clone(),
                max_messages: a.memory.max_messages,
                file_path: a.memory.file_path.clone(),
                database_url: a.memory.database_url.clone(),
            },
            max_iterations: a.max_iterations,
            timeout_seconds: a.timeout_seconds,
            temperature: a.temperature,
            max_tokens: a.max_tokens,
        }
    }
}

impl From<AgentDto> for AgentConfig {
    fn from(dto: AgentDto) -> Self {
        Self {
            name: dto.name,
            description: dto.description,
            tags: dto.tags,
            agent_type: dto.agent_type,
            input_schema: dto.input_schema,
            output_schema: dto.output_schema,
            llm: LlmProviderConfig {
                provider: dto.llm.provider,
                model: dto.llm.model,
                api_key_env: dto.llm.api_key_env,
                base_url: dto.llm.base_url,
                temperature: dto.llm.temperature,
                max_tokens: dto.llm.max_tokens,
                stream: dto.llm.stream,
            },
            system_prompt: dto.system_prompt,
            prompt_template: dto.prompt_template,
            available_tools: dto.available_tools,
            mcp_tools: dto.mcp_tools,
            agent_tools: dto.agent_tools,
            available_resources: dto.available_resources,
            available_resource_templates: dto.available_resource_templates,
            memory: MemoryConfig {
                backend: dto.memory.backend,
                strategy: dto.memory.strategy,
                max_messages: dto.memory.max_messages,
                file_path: dto.memory.file_path,
                database_url: dto.memory.database_url,
            },
            max_iterations: dto.max_iterations,
            timeout_seconds: dto.timeout_seconds,
            temperature: dto.temperature,
            max_tokens: dto.max_tokens,
        }
    }
}

/// Data Transfer Object for Orchestration configuration
#[derive(Serialize, Deserialize, Clone)]
pub struct OrchestrationDto {
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
    pub agents: Vec<AgentReferenceDto>,
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

#[derive(Serialize, Deserialize, Clone)]
pub struct AgentReferenceDto {
    pub agent: String,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_transform: Option<String>,
}

impl From<&OrchestrationConfig> for OrchestrationDto {
    fn from(o: &OrchestrationConfig) -> Self {
        Self {
            name: o.name.clone(),
            description: o.description.clone(),
            tags: o.tags.clone(),
            pattern: o.pattern,
            input_schema: o.input_schema.clone(),
            output_schema: o.output_schema.clone(),
            agents: o.agents.iter().map(|ar| AgentReferenceDto {
                agent: ar.agent.clone(),
                depends_on: ar.depends_on.clone(),
                condition: ar.condition.clone(),
                input_transform: ar.input_transform.clone(),
            }).collect(),
            manager_agent: o.manager_agent.clone(),
            merge_strategy: o.merge_strategy.clone(),
            timeout_seconds: o.timeout_seconds,
        }
    }
}

impl From<OrchestrationDto> for OrchestrationConfig {
    fn from(dto: OrchestrationDto) -> Self {
        Self {
            name: dto.name,
            description: dto.description,
            tags: dto.tags,
            pattern: dto.pattern,
            input_schema: dto.input_schema,
            output_schema: dto.output_schema,
            agents: dto.agents.into_iter().map(|ar| AgentReference {
                agent: ar.agent,
                depends_on: ar.depends_on,
                condition: ar.condition,
                input_transform: ar.input_transform,
            }).collect(),
            manager_agent: dto.manager_agent,
            merge_strategy: dto.merge_strategy,
            timeout_seconds: dto.timeout_seconds,
        }
    }
}

// ============================================================================
// Agent CRUD Handlers
// ============================================================================

/// GET /api/agents - List all agents
pub async fn list_agents(
    State(state): State<ApiState>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        match store.archetypes().list(ArchetypeType::Agent.as_str()).await {
            Ok(agents) => {
                let dtos: Vec<AgentDto> = agents
                    .into_iter()
                    .filter_map(|v| serde_json::from_value(v).ok())
                    .collect();
                return (StatusCode::OK, Json(ApiResponse::success(dtos)));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<Vec<AgentDto>>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let settings = state.settings.read().await;
    let agents: Vec<AgentDto> = settings.agents.iter().map(AgentDto::from).collect();
    (StatusCode::OK, Json(ApiResponse::success(agents)))
}

/// GET /api/agents/:name - Get a single agent
pub async fn get_agent(
    State(state): State<ApiState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        match store.archetypes().get(ArchetypeType::Agent.as_str(), &name).await {
            Ok(Some(agent)) => {
                match serde_json::from_value::<AgentDto>(agent) {
                    Ok(dto) => return (StatusCode::OK, Json(ApiResponse::success(dto))),
                    Err(e) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ApiResponse::<AgentDto>::error(format!("Failed to parse agent: {}", e))),
                        );
                    }
                }
            }
            Ok(None) => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<AgentDto>::error("Agent not found")));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<AgentDto>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let settings = state.settings.read().await;
    if let Some(agent) = settings.agents.iter().find(|a| a.name == name) {
        (StatusCode::OK, Json(ApiResponse::success(AgentDto::from(agent))))
    } else {
        (StatusCode::NOT_FOUND, Json(ApiResponse::<AgentDto>::error("Agent not found")))
    }
}

/// POST /api/agents - Create a new agent
pub async fn create_agent(
    State(state): State<ApiState>,
    Json(dto): Json<AgentDto>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        let definition = match serde_json::to_value(&dto) {
            Ok(v) => v,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<AgentDto>::error(format!("Invalid agent data: {}", e))),
                );
            }
        };

        match store.archetypes().create(ArchetypeType::Agent.as_str(), &dto.name, &definition).await {
            Ok(()) => {
                // Auto-sync to S3 if configured
                if let Err(e) = sync_item_to_s3_if_active(&state, "agents", &dto.name, &dto).await {
                    tracing::warn!("Failed to sync agent to S3: {}", e);
                }
                *state.test_agent_handler.write().await = None;
                if let Some(tool_handler) = &state.tool_handler {
                    if let Err(e) = tool_handler.reinitialize_agents().await {
                        tracing::warn!("Failed to reinitialize agents after creating agent: {}", e);
                    }
                }
                if let Some(broadcaster) = &state.broadcaster {
                    broadcaster.notify_tools_changed().await;
                }
                return (StatusCode::CREATED, Json(ApiResponse::success(dto)));
            }
            Err(crate::persistence::error::PersistenceError::Duplicate { .. }) => {
                return (
                    StatusCode::CONFLICT,
                    Json(ApiResponse::<AgentDto>::error("Agent with this name already exists")),
                );
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<AgentDto>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let mut settings = state.settings.write().await;

    // Check for duplicate name
    if settings.agents.iter().any(|a| a.name == dto.name) {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::<AgentDto>::error("Agent with this name already exists")),
        );
    }

    let agent: AgentConfig = dto.clone().into();
    settings.agents.push(agent.clone());
    drop(settings);

    // Auto-sync to S3 if configured
    if let Err(e) = sync_item_to_s3_if_active(&state, "agents", &dto.name, &dto).await {
        tracing::warn!("Failed to sync agent to S3: {}", e);
    }

    // Reset cached agent handler so it re-initializes with new agent
    *state.test_agent_handler.write().await = None;

    // Reinitialize agents in the main tool handler so the new agent is available
    if let Some(tool_handler) = &state.tool_handler {
        if let Err(e) = tool_handler.reinitialize_agents().await {
            tracing::warn!("Failed to reinitialize agents after creating agent: {}", e);
        }
    }

    // Agents are exposed as tools, so notify about tool list change
    if let Some(broadcaster) = &state.broadcaster {
        broadcaster.notify_tools_changed().await;
    }

    (StatusCode::CREATED, Json(ApiResponse::success(AgentDto::from(&agent))))
}

/// PUT /api/agents/:name - Update an existing agent
pub async fn update_agent(
    State(state): State<ApiState>,
    Path(name): Path<String>,
    Json(dto): Json<AgentDto>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        let definition = match serde_json::to_value(&dto) {
            Ok(v) => v,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<AgentDto>::error(format!("Invalid agent data: {}", e))),
                );
            }
        };

        match store.archetypes().update(ArchetypeType::Agent.as_str(), &name, &definition, None).await {
            Ok(_) => {
                // Auto-sync to S3 if configured
                if let Err(e) = sync_item_to_s3_if_active(&state, "agents", &dto.name, &dto).await {
                    tracing::warn!("Failed to sync agent to S3: {}", e);
                }
                *state.test_agent_handler.write().await = None;
                if let Some(tool_handler) = &state.tool_handler {
                    if let Err(e) = tool_handler.reinitialize_agents().await {
                        tracing::warn!("Failed to reinitialize agents after updating agent: {}", e);
                    }
                }
                if let Some(broadcaster) = &state.broadcaster {
                    broadcaster.notify_tools_changed().await;
                }
                return (StatusCode::OK, Json(ApiResponse::success(dto)));
            }
            Err(crate::persistence::error::PersistenceError::NotFound { .. }) => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<AgentDto>::error("Agent not found")));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<AgentDto>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let mut settings = state.settings.write().await;

    if let Some(agent) = settings.agents.iter_mut().find(|a| a.name == name) {
        *agent = dto.clone().into();
        let result = AgentDto::from(&*agent);
        drop(settings);

        // Auto-sync to S3 if configured
        if let Err(e) = sync_item_to_s3_if_active(&state, "agents", &dto.name, &dto).await {
            tracing::warn!("Failed to sync agent to S3: {}", e);
        }

        // Reset cached agent handler so it re-initializes with updated agent
        *state.test_agent_handler.write().await = None;

        // Reinitialize agents in the main tool handler so the updated agent is available
        if let Some(tool_handler) = &state.tool_handler {
            if let Err(e) = tool_handler.reinitialize_agents().await {
                tracing::warn!("Failed to reinitialize agents after updating agent: {}", e);
            }
        }

        // Agents are exposed as tools, so notify about tool list change
        if let Some(broadcaster) = &state.broadcaster {
            broadcaster.notify_tools_changed().await;
        }

        (StatusCode::OK, Json(ApiResponse::success(result)))
    } else {
        (StatusCode::NOT_FOUND, Json(ApiResponse::<AgentDto>::error("Agent not found")))
    }
}

/// DELETE /api/agents/:name - Delete an agent
pub async fn delete_agent(
    State(state): State<ApiState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        match store.archetypes().delete(ArchetypeType::Agent.as_str(), &name).await {
            Ok(true) => {
                // Auto-delete from S3 if configured
                if let Err(e) = delete_item_from_s3_if_active(&state, "agents", &name).await {
                    tracing::warn!("Failed to delete agent from S3: {}", e);
                }
                *state.test_agent_handler.write().await = None;
                if let Some(tool_handler) = &state.tool_handler {
                    if let Err(e) = tool_handler.reinitialize_agents().await {
                        tracing::warn!("Failed to reinitialize agents after deleting agent: {}", e);
                    }
                }
                if let Some(broadcaster) = &state.broadcaster {
                    broadcaster.notify_tools_changed().await;
                }
                return (StatusCode::OK, Json(ApiResponse::ok()));
            }
            Ok(false) => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<()>::error("Agent not found")));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<()>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let mut settings = state.settings.write().await;

    let initial_len = settings.agents.len();
    settings.agents.retain(|a| a.name != name);

    if settings.agents.len() < initial_len {
        drop(settings);

        // Auto-delete from S3 if configured
        if let Err(e) = delete_item_from_s3_if_active(&state, "agents", &name).await {
            tracing::warn!("Failed to delete agent from S3: {}", e);
        }

        // Reset cached agent handler so it re-initializes without deleted agent
        *state.test_agent_handler.write().await = None;

        // Reinitialize agents in the main tool handler so the deleted agent is removed
        if let Some(tool_handler) = &state.tool_handler {
            if let Err(e) = tool_handler.reinitialize_agents().await {
                tracing::warn!("Failed to reinitialize agents after deleting agent: {}", e);
            }
        }

        // Agents are exposed as tools, so notify about tool list change
        if let Some(broadcaster) = &state.broadcaster {
            broadcaster.notify_tools_changed().await;
        }

        (StatusCode::OK, Json(ApiResponse::ok()))
    } else {
        (StatusCode::NOT_FOUND, Json(ApiResponse::<()>::error("Agent not found")))
    }
}

/// POST /api/agents/:name/test - Test an agent
pub async fn test_agent(
    State(state): State<ApiState>,
    Path(name): Path<String>,
    Json(req): Json<TestRequest>,
) -> impl IntoResponse {
    use crate::adapters::tool_handler::BasicToolHandler;
    use crate::agents::handler::AgentHandler;

    let start = std::time::Instant::now();

    // Verify agent exists in config
    let settings = state.settings.read().await;
    if !settings.agents.iter().any(|a| a.name == name) {
        return (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<TestResult>::error("Agent not found")),
        );
    }
    drop(settings);

    // Initialize agent handler only if not already present (preserves memory store for multi-turn)
    {
        let handler_guard = state.test_agent_handler.read().await;
        if handler_guard.is_none() {
            drop(handler_guard);
            let mut handler_guard = state.test_agent_handler.write().await;
            // Double-check after acquiring write lock
            if handler_guard.is_none() {
                let tool_handler = Arc::new(BasicToolHandler::new(
                    state.settings.clone(),
                    state.mock_strategy.clone(),
                ));
                // Use new_with_secrets to enable API key lookup from secrets store
                let agent_handler = AgentHandler::new_with_secrets(
                    state.settings.clone(),
                    tool_handler.clone(),
                    state.secrets.clone(),
                );

                // Initialize the agent handler to populate agent cache
                if let Err(e) = agent_handler.initialize().await {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<TestResult>::error(&format!("Failed to initialize agent handler: {}", e))),
                    );
                }

                // Wire up agent handler to tool handler so agents can call other agents
                let agent_handler = Arc::new(agent_handler);
                tool_handler.set_agent_handler(agent_handler.clone()).await;

                *handler_guard = Some(agent_handler);
            }
        }
    }

    // Use the shared handler
    let handler_guard = state.test_agent_handler.read().await;
    let agent_handler = handler_guard.as_ref().unwrap();

    // Execute the agent with optional session_id for multi-turn conversations
    match agent_handler.execute(&name, req.args.clone(), req.session_id.clone()).await {
        Ok(response) => {
            let elapsed = start.elapsed().as_millis() as u64;
            let output = json!({
                "output": response.output,
                "session_id": response.session_id,
                "iterations": response.iterations,
                "tool_calls": response.tool_calls.len(),
                "reasoning_steps": response.reasoning_steps.len(),
            });

            (
                StatusCode::OK,
                Json(ApiResponse::success(TestResult {
                    output,
                    error: None,
                    execution_time_ms: elapsed,
                })),
            )
        }
        Err(e) => {
            let elapsed = start.elapsed().as_millis() as u64;
            (
                StatusCode::OK,
                Json(ApiResponse::success(TestResult {
                    output: json!({}),
                    error: Some(e.to_string()),
                    execution_time_ms: elapsed,
                })),
            )
        }
    }
}

// ============================================================================
// Orchestration CRUD Handlers
// ============================================================================

/// GET /api/orchestrations - List all orchestrations
pub async fn list_orchestrations(
    State(state): State<ApiState>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        match store.archetypes().list(ArchetypeType::Orchestration.as_str()).await {
            Ok(orchestrations) => {
                let dtos: Vec<OrchestrationDto> = orchestrations
                    .into_iter()
                    .filter_map(|v| serde_json::from_value(v).ok())
                    .collect();
                return (StatusCode::OK, Json(ApiResponse::success(dtos)));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<Vec<OrchestrationDto>>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let settings = state.settings.read().await;
    let orchestrations: Vec<OrchestrationDto> = settings.orchestrations.iter().map(OrchestrationDto::from).collect();
    (StatusCode::OK, Json(ApiResponse::success(orchestrations)))
}

/// GET /api/orchestrations/:name - Get a single orchestration
pub async fn get_orchestration(
    State(state): State<ApiState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        match store.archetypes().get(ArchetypeType::Orchestration.as_str(), &name).await {
            Ok(Some(orchestration)) => {
                match serde_json::from_value::<OrchestrationDto>(orchestration) {
                    Ok(dto) => return (StatusCode::OK, Json(ApiResponse::success(dto))),
                    Err(e) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ApiResponse::<OrchestrationDto>::error(format!("Failed to parse orchestration: {}", e))),
                        );
                    }
                }
            }
            Ok(None) => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<OrchestrationDto>::error("Orchestration not found")));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<OrchestrationDto>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let settings = state.settings.read().await;
    if let Some(orchestration) = settings.orchestrations.iter().find(|o| o.name == name) {
        (StatusCode::OK, Json(ApiResponse::success(OrchestrationDto::from(orchestration))))
    } else {
        (StatusCode::NOT_FOUND, Json(ApiResponse::<OrchestrationDto>::error("Orchestration not found")))
    }
}

/// POST /api/orchestrations - Create a new orchestration
pub async fn create_orchestration(
    State(state): State<ApiState>,
    Json(dto): Json<OrchestrationDto>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        let definition = match serde_json::to_value(&dto) {
            Ok(v) => v,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<OrchestrationDto>::error(format!("Invalid orchestration data: {}", e))),
                );
            }
        };

        match store.archetypes().create(ArchetypeType::Orchestration.as_str(), &dto.name, &definition).await {
            Ok(()) => {
                return (StatusCode::CREATED, Json(ApiResponse::success(dto)));
            }
            Err(crate::persistence::error::PersistenceError::Duplicate { .. }) => {
                return (
                    StatusCode::CONFLICT,
                    Json(ApiResponse::<OrchestrationDto>::error("Orchestration with this name already exists")),
                );
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<OrchestrationDto>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let mut settings = state.settings.write().await;

    // Check for duplicate name
    if settings.orchestrations.iter().any(|o| o.name == dto.name) {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::<OrchestrationDto>::error("Orchestration with this name already exists")),
        );
    }

    let orchestration: OrchestrationConfig = dto.into();
    settings.orchestrations.push(orchestration.clone());
    (StatusCode::CREATED, Json(ApiResponse::success(OrchestrationDto::from(&orchestration))))
}

/// PUT /api/orchestrations/:name - Update an existing orchestration
pub async fn update_orchestration(
    State(state): State<ApiState>,
    Path(name): Path<String>,
    Json(dto): Json<OrchestrationDto>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        let definition = match serde_json::to_value(&dto) {
            Ok(v) => v,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<OrchestrationDto>::error(format!("Invalid orchestration data: {}", e))),
                );
            }
        };

        match store.archetypes().update(ArchetypeType::Orchestration.as_str(), &name, &definition, None).await {
            Ok(_) => {
                return (StatusCode::OK, Json(ApiResponse::success(dto)));
            }
            Err(crate::persistence::error::PersistenceError::NotFound { .. }) => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<OrchestrationDto>::error("Orchestration not found")));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<OrchestrationDto>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let mut settings = state.settings.write().await;

    if let Some(orchestration) = settings.orchestrations.iter_mut().find(|o| o.name == name) {
        *orchestration = dto.into();
        (StatusCode::OK, Json(ApiResponse::success(OrchestrationDto::from(&*orchestration))))
    } else {
        (StatusCode::NOT_FOUND, Json(ApiResponse::<OrchestrationDto>::error("Orchestration not found")))
    }
}

/// DELETE /api/orchestrations/:name - Delete an orchestration
pub async fn delete_orchestration(
    State(state): State<ApiState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        match store.archetypes().delete(ArchetypeType::Orchestration.as_str(), &name).await {
            Ok(true) => {
                return (StatusCode::OK, Json(ApiResponse::ok()));
            }
            Ok(false) => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<()>::error("Orchestration not found")));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<()>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let mut settings = state.settings.write().await;

    let initial_len = settings.orchestrations.len();
    settings.orchestrations.retain(|o| o.name != name);

    if settings.orchestrations.len() < initial_len {
        (StatusCode::OK, Json(ApiResponse::ok()))
    } else {
        (StatusCode::NOT_FOUND, Json(ApiResponse::<()>::error("Orchestration not found")))
    }
}

/// POST /api/orchestrations/:name/test - Test an orchestration
pub async fn test_orchestration(
    State(state): State<ApiState>,
    Path(name): Path<String>,
    Json(req): Json<TestRequest>,
) -> impl IntoResponse {
    use crate::adapters::tool_handler::BasicToolHandler;
    use crate::agents::handler::AgentHandler;

    let start = std::time::Instant::now();
    let settings = state.settings.read().await;

    // Find the orchestration
    let orchestration = match settings.orchestrations.iter().find(|o| o.name == name) {
        Some(o) => o.clone(),
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<TestResult>::error("Orchestration not found")),
            );
        }
    };
    drop(settings);

    // Create agent handler on-demand for testing
    let tool_handler = Arc::new(BasicToolHandler::new(
        state.settings.clone(),
        state.mock_strategy.clone(),
    ));
    let agent_handler = AgentHandler::new_with_secrets(state.settings.clone(), tool_handler, state.secrets.clone());

    // Initialize the agent handler to populate agent cache and orchestration engine
    if let Err(e) = agent_handler.initialize().await {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<TestResult>::error(&format!("Failed to initialize agent handler: {}", e))),
        );
    }

    // Execute the orchestration
    match agent_handler.execute_orchestration(&orchestration, req.args.clone()) {
        Ok(stream) => {
            match stream.collect().await {
                Ok(response) => {
                    let elapsed = start.elapsed().as_millis() as u64;
                    let output = json!({
                        "output": response.output,
                        "session_id": response.session_id,
                        "iterations": response.iterations,
                        "tool_calls": response.tool_calls.len(),
                        "reasoning_steps": response.reasoning_steps.len(),
                    });

                    (
                        StatusCode::OK,
                        Json(ApiResponse::success(TestResult {
                            output,
                            error: None,
                            execution_time_ms: elapsed,
                        })),
                    )
                }
                Err(e) => {
                    let elapsed = start.elapsed().as_millis() as u64;
                    (
                        StatusCode::OK,
                        Json(ApiResponse::success(TestResult {
                            output: json!({}),
                            error: Some(e.to_string()),
                            execution_time_ms: elapsed,
                        })),
                    )
                }
            }
        }
        Err(e) => {
            let elapsed = start.elapsed().as_millis() as u64;
            (
                StatusCode::OK,
                Json(ApiResponse::success(TestResult {
                    output: json!({}),
                    error: Some(e.to_string()),
                    execution_time_ms: elapsed,
                })),
            )
        }
    }
}

/// Model info returned from LLM providers
#[derive(Debug, Serialize, Deserialize)]
pub struct LlmModelInfo {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Request for fetching models from a provider
#[derive(Debug, Deserialize)]
pub struct FetchModelsRequest {
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_key_env: Option<String>,
}

/// GET /api/llm/models/:provider - Fetch available models from an LLM provider
pub async fn fetch_llm_models(
    State(state): State<ApiState>,
    Path(provider): Path<String>,
    axum::extract::Query(params): axum::extract::Query<FetchModelsRequest>,
) -> impl IntoResponse {
    let models = match provider.to_lowercase().as_str() {
        "openai" => fetch_openai_models(params.api_key_env, &state.secrets).await,
        "anthropic" => fetch_anthropic_models().await,
        "gemini" => fetch_gemini_models(params.api_key_env, &state.secrets).await,
        "ollama" => fetch_ollama_models(params.base_url).await,
        "azureopenai" => fetch_azure_openai_models().await,
        _ => Err(format!("Unknown provider: {}", provider)),
    };

    match models {
        Ok(models) => (StatusCode::OK, Json(ApiResponse::success(models))),
        Err(e) => (
            StatusCode::OK,
            Json(ApiResponse::success(vec![LlmModelInfo {
                id: "error".to_string(),
                name: format!("Failed to fetch models: {}", e),
                description: None,
            }])),
        ),
    }
}

async fn fetch_openai_models(api_key_env: Option<String>, secrets: &SharedSecretsStore) -> Result<Vec<LlmModelInfo>, String> {
    let env_var = api_key_env.as_deref().unwrap_or("OPENAI_API_KEY");
    let api_key = secrets.get_or_env(env_var).await
        .ok_or_else(|| format!("API key not found: {} (set via UI Secrets or environment variable)", env_var))?;

    let client = reqwest::Client::new();
    let response = client
        .get("https://api.openai.com/v1/models")
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("API error: {}", response.status()));
    }

    let data: Value = response.json().await.map_err(|e| format!("Parse error: {}", e))?;

    let mut models: Vec<LlmModelInfo> = data["data"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|m| {
            let id = m["id"].as_str()?;
            // Filter to only include chat models
            if id.starts_with("gpt-") || id.starts_with("o1") || id.starts_with("chatgpt") {
                Some(LlmModelInfo {
                    id: id.to_string(),
                    name: format_model_name(id),
                    description: None,
                })
            } else {
                None
            }
        })
        .collect();

    // Sort by name
    models.sort_by(|a, b| a.name.cmp(&b.name));

    // If no models found, return defaults
    if models.is_empty() {
        models = get_default_openai_models();
    }

    Ok(models)
}

async fn fetch_anthropic_models() -> Result<Vec<LlmModelInfo>, String> {
    // Anthropic doesn't have a public models API, return known models
    Ok(vec![
        LlmModelInfo { id: "claude-sonnet-4-20250514".to_string(), name: "Claude Sonnet 4 (Latest)".to_string(), description: Some("Most intelligent model".to_string()) },
        LlmModelInfo { id: "claude-opus-4-20250514".to_string(), name: "Claude Opus 4".to_string(), description: Some("Highest capability".to_string()) },
        LlmModelInfo { id: "claude-3-7-sonnet-20250219".to_string(), name: "Claude 3.7 Sonnet".to_string(), description: None },
        LlmModelInfo { id: "claude-3-5-sonnet-20241022".to_string(), name: "Claude 3.5 Sonnet".to_string(), description: None },
        LlmModelInfo { id: "claude-3-5-haiku-20241022".to_string(), name: "Claude 3.5 Haiku".to_string(), description: Some("Fast and efficient".to_string()) },
        LlmModelInfo { id: "claude-3-opus-20240229".to_string(), name: "Claude 3 Opus".to_string(), description: None },
        LlmModelInfo { id: "claude-3-sonnet-20240229".to_string(), name: "Claude 3 Sonnet".to_string(), description: None },
        LlmModelInfo { id: "claude-3-haiku-20240307".to_string(), name: "Claude 3 Haiku".to_string(), description: None },
    ])
}

async fn fetch_gemini_models(api_key_env: Option<String>, secrets: &SharedSecretsStore) -> Result<Vec<LlmModelInfo>, String> {
    let env_var = api_key_env.as_deref().unwrap_or("GEMINI_API_KEY");
    let api_key = secrets.get_or_env(env_var).await
        .ok_or_else(|| format!("API key not found: {} (set via UI Secrets or environment variable)", env_var))?;

    let client = reqwest::Client::new();
    // Use v1beta endpoint to get all models including Gemini 2.0+
    let url = format!("https://generativelanguage.googleapis.com/v1beta/models?key={}", api_key);

    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("API error: {}", response.status()));
    }

    let data: Value = response.json().await.map_err(|e| format!("Parse error: {}", e))?;

    let mut models: Vec<LlmModelInfo> = data["models"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|m| {
            let name = m["name"].as_str()?;
            let display_name = m["displayName"].as_str().unwrap_or(name);
            let description = m["description"].as_str().map(|s| s.to_string());

            // Extract model ID from "models/gemini-1.5-pro" format
            let id = name.strip_prefix("models/").unwrap_or(name);

            // Only include generative models
            if id.contains("gemini") && m["supportedGenerationMethods"]
                .as_array()
                .map(|arr| arr.iter().any(|v| v.as_str() == Some("generateContent")))
                .unwrap_or(false)
            {
                Some(LlmModelInfo {
                    id: id.to_string(),
                    name: display_name.to_string(),
                    description,
                })
            } else {
                None
            }
        })
        .collect();

    // Sort by name
    models.sort_by(|a, b| a.name.cmp(&b.name));

    // If no models found, return defaults
    if models.is_empty() {
        models = get_default_gemini_models();
    }

    Ok(models)
}

async fn fetch_ollama_models(base_url: Option<String>) -> Result<Vec<LlmModelInfo>, String> {
    let base = base_url.unwrap_or_else(|| "http://localhost:11434".to_string());
    let url = format!("{}/api/tags", base.trim_end_matches('/'));

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| format!("Cannot connect to Ollama at {}: {}", base, e))?;

    if !response.status().is_success() {
        return Err(format!("Ollama API error: {}", response.status()));
    }

    let data: Value = response.json().await.map_err(|e| format!("Parse error: {}", e))?;

    let mut models: Vec<LlmModelInfo> = data["models"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|m| {
            let name = m["name"].as_str()?;
            let size = m["size"].as_u64().map(format_size);

            Some(LlmModelInfo {
                id: name.to_string(),
                name: name.to_string(),
                description: size.map(|s| format!("Size: {}", s)),
            })
        })
        .collect();

    // Sort by name
    models.sort_by(|a, b| a.name.cmp(&b.name));

    if models.is_empty() {
        return Err("No models found. Pull models with: ollama pull <model>".to_string());
    }

    Ok(models)
}

async fn fetch_azure_openai_models() -> Result<Vec<LlmModelInfo>, String> {
    // Azure OpenAI models are deployment-specific, return common options
    Ok(vec![
        LlmModelInfo { id: "gpt-4o".to_string(), name: "GPT-4o".to_string(), description: Some("Use your deployment name".to_string()) },
        LlmModelInfo { id: "gpt-4o-mini".to_string(), name: "GPT-4o Mini".to_string(), description: None },
        LlmModelInfo { id: "gpt-4-turbo".to_string(), name: "GPT-4 Turbo".to_string(), description: None },
        LlmModelInfo { id: "gpt-4".to_string(), name: "GPT-4".to_string(), description: None },
        LlmModelInfo { id: "gpt-35-turbo".to_string(), name: "GPT-3.5 Turbo".to_string(), description: None },
    ])
}

fn format_model_name(id: &str) -> String {
    // Convert model IDs to friendly names
    match id {
        "gpt-4o" => "GPT-4o (Latest)".to_string(),
        "gpt-4o-2024-11-20" => "GPT-4o (Nov 2024)".to_string(),
        "gpt-4o-2024-08-06" => "GPT-4o (Aug 2024)".to_string(),
        "gpt-4o-2024-05-13" => "GPT-4o (May 2024)".to_string(),
        "gpt-4o-mini" => "GPT-4o Mini".to_string(),
        "gpt-4o-mini-2024-07-18" => "GPT-4o Mini (Jul 2024)".to_string(),
        "gpt-4-turbo" => "GPT-4 Turbo".to_string(),
        "gpt-4-turbo-preview" => "GPT-4 Turbo Preview".to_string(),
        "gpt-4" => "GPT-4".to_string(),
        "gpt-4-0613" => "GPT-4 (Jun 2023)".to_string(),
        "gpt-3.5-turbo" => "GPT-3.5 Turbo".to_string(),
        "gpt-3.5-turbo-0125" => "GPT-3.5 Turbo (Jan 2024)".to_string(),
        "o1" => "o1 (Reasoning)".to_string(),
        "o1-preview" => "o1 Preview".to_string(),
        "o1-mini" => "o1 Mini".to_string(),
        "chatgpt-4o-latest" => "ChatGPT-4o Latest".to_string(),
        _ => id.to_string(),
    }
}

fn format_size(bytes: u64) -> String {
    const GB: u64 = 1024 * 1024 * 1024;
    const MB: u64 = 1024 * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    }
}

fn get_default_openai_models() -> Vec<LlmModelInfo> {
    vec![
        LlmModelInfo { id: "gpt-4o".to_string(), name: "GPT-4o (Latest)".to_string(), description: None },
        LlmModelInfo { id: "gpt-4o-mini".to_string(), name: "GPT-4o Mini".to_string(), description: None },
        LlmModelInfo { id: "gpt-4-turbo".to_string(), name: "GPT-4 Turbo".to_string(), description: None },
        LlmModelInfo { id: "gpt-4".to_string(), name: "GPT-4".to_string(), description: None },
        LlmModelInfo { id: "gpt-3.5-turbo".to_string(), name: "GPT-3.5 Turbo".to_string(), description: None },
        LlmModelInfo { id: "o1".to_string(), name: "o1 (Reasoning)".to_string(), description: None },
        LlmModelInfo { id: "o1-mini".to_string(), name: "o1 Mini".to_string(), description: None },
    ]
}

fn get_default_gemini_models() -> Vec<LlmModelInfo> {
    vec![
        LlmModelInfo { id: "gemini-2.5-pro-preview-06-05".to_string(), name: "Gemini 2.5 Pro Preview".to_string(), description: Some("Latest preview".to_string()) },
        LlmModelInfo { id: "gemini-2.5-flash-preview-05-20".to_string(), name: "Gemini 2.5 Flash Preview".to_string(), description: None },
        LlmModelInfo { id: "gemini-2.0-flash".to_string(), name: "Gemini 2.0 Flash".to_string(), description: None },
        LlmModelInfo { id: "gemini-2.0-flash-lite".to_string(), name: "Gemini 2.0 Flash Lite".to_string(), description: Some("Cost efficient".to_string()) },
        LlmModelInfo { id: "gemini-2.0-flash-thinking-exp".to_string(), name: "Gemini 2.0 Flash Thinking".to_string(), description: Some("Reasoning model".to_string()) },
        LlmModelInfo { id: "gemini-1.5-pro".to_string(), name: "Gemini 1.5 Pro".to_string(), description: None },
        LlmModelInfo { id: "gemini-1.5-flash".to_string(), name: "Gemini 1.5 Flash".to_string(), description: None },
    ]
}

// ============================================================================
// Secrets Management - In-Memory API Key Storage
// ============================================================================

/// Shared state for secrets API (separate from ApiState to allow different routes)
#[derive(Clone)]
pub struct SecretsApiState {
    pub secrets: SharedSecretsStore,
    /// Reference to cached agent handler to reset when secrets change
    pub test_agent_handler: Arc<RwLock<Option<Arc<dyn AgentPort>>>>,
    /// MCP notification broadcaster for list change notifications
    pub broadcaster: Option<SharedNotificationBroadcaster>,
    /// Tool handler to reinitialize agents when API keys change
    pub tool_handler: Option<Arc<crate::adapters::tool_handler::BasicToolHandler>>,
}

/// Request body for setting a secret
#[derive(Deserialize)]
pub struct SetSecretRequest {
    pub value: String,
}

/// Response showing which secrets are configured (not values)
#[derive(Serialize)]
pub struct SecretsStatusResponse {
    /// List of secret keys that have been set
    pub configured: Vec<String>,
    /// All known secret keys with their set status
    pub keys: Vec<SecretKeyStatus>,
}

/// Status of a single secret key
#[derive(Serialize)]
pub struct SecretKeyStatus {
    pub key: String,
    pub label: String,
    pub description: String,
    pub is_set: bool,
    pub category: String,
}

/// GET /api/secrets - List all secrets and their status (not values)
pub async fn list_secrets(
    State(state): State<SecretsApiState>,
) -> impl IntoResponse {
    let configured = state.secrets.list_keys().await;

    // Define all known secret keys with metadata
    let all_keys = vec![
        ("OPENAI_API_KEY", "OpenAI API Key", "API key for OpenAI models (GPT-4, etc.)", "AI Providers"),
        ("ANTHROPIC_API_KEY", "Anthropic API Key", "API key for Anthropic models", "AI Providers"),
        ("GEMINI_API_KEY", "Gemini API Key", "API key for Google Gemini models", "AI Providers"),
        ("AWS_ACCESS_KEY_ID", "AWS Access Key ID", "AWS access key for S3 and other services", "AWS/S3"),
        ("AWS_SECRET_ACCESS_KEY", "AWS Secret Access Key", "AWS secret key for S3 and other services", "AWS/S3"),
        ("AWS_REGION", "AWS Region", "Default AWS region (e.g., us-east-1)", "AWS/S3"),
    ];

    let keys: Vec<SecretKeyStatus> = all_keys
        .into_iter()
        .map(|(key, label, desc, category)| SecretKeyStatus {
            key: key.to_string(),
            label: label.to_string(),
            description: desc.to_string(),
            is_set: configured.contains(&key.to_string()),
            category: category.to_string(),
        })
        .collect();

    let response = SecretsStatusResponse { configured, keys };
    (StatusCode::OK, Json(ApiResponse::success(response)))
}

/// POST /api/secrets/:key - Set a secret value
pub async fn set_secret(
    State(state): State<SecretsApiState>,
    Path(key): Path<String>,
    Json(req): Json<SetSecretRequest>,
) -> impl IntoResponse {
    // Validate the key is a known secret key
    let valid_keys = [
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
        "GEMINI_API_KEY",
        "AWS_ACCESS_KEY_ID",
        "AWS_SECRET_ACCESS_KEY",
        "AWS_REGION",
    ];

    if !valid_keys.contains(&key.as_str()) {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error(format!("Unknown secret key: {}", key))),
        );
    }

    if req.value.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(ApiResponse::<()>::error("Secret value cannot be empty".to_string())),
        );
    }

    state.secrets.set(&key, &req.value).await;
    tracing::info!("Secret '{}' has been set", key);

    // Reset cached agent handler so it re-initializes with new API key
    *state.test_agent_handler.write().await = None;

    // Reinitialize agents in the main tool handler (makes agents available that now have API keys)
    if let Some(tool_handler) = &state.tool_handler {
        if let Err(e) = tool_handler.reinitialize_agents().await {
            tracing::warn!("Failed to reinitialize agents after setting secret: {}", e);
        }
    }

    // API keys affect which agents are available (agents are exposed as tools)
    if let Some(broadcaster) = &state.broadcaster {
        broadcaster.notify_tools_changed().await;
    }

    (StatusCode::OK, Json(ApiResponse::<()>::ok()))
}

/// DELETE /api/secrets/:key - Delete a secret
pub async fn delete_secret(
    State(state): State<SecretsApiState>,
    Path(key): Path<String>,
) -> impl IntoResponse {
    if state.secrets.delete(&key).await {
        tracing::info!("Secret '{}' has been deleted", key);

        // Reset cached agent handler so it re-initializes without deleted API key
        *state.test_agent_handler.write().await = None;

        // Reinitialize agents in the main tool handler (removes agents that no longer have API keys)
        if let Some(tool_handler) = &state.tool_handler {
            if let Err(e) = tool_handler.reinitialize_agents().await {
                tracing::warn!("Failed to reinitialize agents after deleting secret: {}", e);
            }
        }

        // API keys affect which agents are available (agents are exposed as tools)
        if let Some(broadcaster) = &state.broadcaster {
            broadcaster.notify_tools_changed().await;
        }

        (StatusCode::OK, Json(ApiResponse::<()>::ok()))
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<()>::error(format!("Secret '{}' not found", key))),
        )
    }
}

/// DELETE /api/secrets - Clear all secrets
pub async fn clear_secrets(
    State(state): State<SecretsApiState>,
) -> impl IntoResponse {
    state.secrets.clear().await;
    tracing::info!("All secrets have been cleared");

    // Reset cached agent handler so it re-initializes without API keys
    *state.test_agent_handler.write().await = None;

    // Reinitialize agents in the main tool handler (removes all agents that needed API keys)
    if let Some(tool_handler) = &state.tool_handler {
        if let Err(e) = tool_handler.reinitialize_agents().await {
            tracing::warn!("Failed to reinitialize agents after clearing secrets: {}", e);
        }
    }

    // API keys affect which agents are available (agents are exposed as tools)
    if let Some(broadcaster) = &state.broadcaster {
        broadcaster.notify_tools_changed().await;
    }

    (StatusCode::OK, Json(ApiResponse::<()>::ok()))
}

// ============================================================================
// Schema CRUD Endpoints
// ============================================================================

/// DTO for reusable JSON schema definitions
#[derive(Serialize, Deserialize, Clone)]
pub struct SchemaDto {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Tags for categorization and filtering
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub schema: Value,
}

impl From<&SchemaConfig> for SchemaDto {
    fn from(s: &SchemaConfig) -> Self {
        Self {
            name: s.name.clone(),
            description: s.description.clone(),
            tags: s.tags.clone(),
            schema: s.schema.clone(),
        }
    }
}

impl From<SchemaDto> for SchemaConfig {
    fn from(dto: SchemaDto) -> Self {
        Self {
            name: dto.name,
            description: dto.description,
            tags: dto.tags,
            schema: dto.schema,
        }
    }
}

/// GET /api/schemas - List all schemas
pub async fn list_schemas(
    State(state): State<ApiState>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        match store.archetypes().list(ArchetypeType::Schema.as_str()).await {
            Ok(schemas) => {
                let dtos: Vec<SchemaDto> = schemas
                    .into_iter()
                    .filter_map(|v| serde_json::from_value(v).ok())
                    .collect();
                return (StatusCode::OK, Json(ApiResponse::success(dtos)));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<Vec<SchemaDto>>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let settings = state.settings.read().await;
    let schemas: Vec<SchemaDto> = settings.schemas.iter().map(SchemaDto::from).collect();
    (StatusCode::OK, Json(ApiResponse::success(schemas)))
}

/// GET /api/schemas/:name - Get a single schema
pub async fn get_schema(
    State(state): State<ApiState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        match store.archetypes().get(ArchetypeType::Schema.as_str(), &name).await {
            Ok(Some(schema)) => {
                match serde_json::from_value::<SchemaDto>(schema) {
                    Ok(dto) => return (StatusCode::OK, Json(ApiResponse::success(dto))),
                    Err(e) => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(ApiResponse::<SchemaDto>::error(format!("Failed to parse schema: {}", e))),
                        );
                    }
                }
            }
            Ok(None) => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<SchemaDto>::error("Schema not found")));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<SchemaDto>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let settings = state.settings.read().await;
    if let Some(schema) = settings.schemas.iter().find(|s| s.name == name) {
        (StatusCode::OK, Json(ApiResponse::success(SchemaDto::from(schema))))
    } else {
        (StatusCode::NOT_FOUND, Json(ApiResponse::<SchemaDto>::error("Schema not found")))
    }
}

/// POST /api/schemas - Create a new schema
pub async fn create_schema(
    State(state): State<ApiState>,
    Json(dto): Json<SchemaDto>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        let definition = match serde_json::to_value(&dto) {
            Ok(v) => v,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<SchemaDto>::error(format!("Invalid schema data: {}", e))),
                );
            }
        };

        match store.archetypes().create(ArchetypeType::Schema.as_str(), &dto.name, &definition).await {
            Ok(()) => {
                // Auto-sync to S3 if configured
                if let Err(e) = sync_item_to_s3_if_active(&state, "schemas", &dto.name, &dto).await {
                    tracing::warn!("Failed to sync schema to S3: {}", e);
                }
                return (StatusCode::CREATED, Json(ApiResponse::success(dto)));
            }
            Err(crate::persistence::error::PersistenceError::Duplicate { .. }) => {
                return (
                    StatusCode::CONFLICT,
                    Json(ApiResponse::<SchemaDto>::error("Schema with this name already exists")),
                );
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<SchemaDto>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let mut settings = state.settings.write().await;

    // Check for duplicate name
    if settings.schemas.iter().any(|s| s.name == dto.name) {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::<SchemaDto>::error("Schema with this name already exists")),
        );
    }

    let schema = SchemaConfig::from(dto.clone());
    settings.schemas.push(schema);
    drop(settings);

    // Auto-sync to S3 if configured
    if let Err(e) = sync_item_to_s3_if_active(&state, "schemas", &dto.name, &dto).await {
        tracing::warn!("Failed to sync schema to S3: {}", e);
    }

    (StatusCode::CREATED, Json(ApiResponse::success(dto)))
}

/// PUT /api/schemas/:name - Update a schema
pub async fn update_schema(
    State(state): State<ApiState>,
    Path(name): Path<String>,
    Json(dto): Json<SchemaDto>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        let definition = match serde_json::to_value(&dto) {
            Ok(v) => v,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(ApiResponse::<SchemaDto>::error(format!("Invalid schema data: {}", e))),
                );
            }
        };

        match store.archetypes().update(ArchetypeType::Schema.as_str(), &name, &definition, None).await {
            Ok(_) => {
                // Auto-sync to S3 if configured
                // If name changed, delete old key first
                if dto.name != name {
                    if let Err(e) = delete_item_from_s3_if_active(&state, "schemas", &name).await {
                        tracing::warn!("Failed to delete old schema from S3: {}", e);
                    }
                }
                if let Err(e) = sync_item_to_s3_if_active(&state, "schemas", &dto.name, &dto).await {
                    tracing::warn!("Failed to sync schema to S3: {}", e);
                }
                return (StatusCode::OK, Json(ApiResponse::success(dto)));
            }
            Err(crate::persistence::error::PersistenceError::NotFound { .. }) => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<SchemaDto>::error("Schema not found")));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<SchemaDto>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let mut settings = state.settings.write().await;

    // Check if name is being changed and would conflict
    if dto.name != name {
        let new_name_exists = settings.schemas.iter().any(|s| s.name == dto.name);
        if new_name_exists {
            return (
                StatusCode::CONFLICT,
                Json(ApiResponse::<SchemaDto>::error("Schema with this name already exists")),
            );
        }
    }

    if let Some(schema) = settings.schemas.iter_mut().find(|s| s.name == name) {
        schema.name = dto.name.clone();
        schema.description = dto.description.clone();
        schema.schema = dto.schema.clone();
        drop(settings);

        // Auto-sync to S3 if configured
        // If name changed, delete old key first
        if dto.name != name {
            if let Err(e) = delete_item_from_s3_if_active(&state, "schemas", &name).await {
                tracing::warn!("Failed to delete old schema from S3: {}", e);
            }
        }
        if let Err(e) = sync_item_to_s3_if_active(&state, "schemas", &dto.name, &dto).await {
            tracing::warn!("Failed to sync schema to S3: {}", e);
        }

        (StatusCode::OK, Json(ApiResponse::success(dto)))
    } else {
        (StatusCode::NOT_FOUND, Json(ApiResponse::<SchemaDto>::error("Schema not found")))
    }
}

/// DELETE /api/schemas/:name - Delete a schema
pub async fn delete_schema(
    State(state): State<ApiState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    // Use database if available
    if let Some(store) = &state.data_store {
        match store.archetypes().delete(ArchetypeType::Schema.as_str(), &name).await {
            Ok(true) => {
                // Auto-delete from S3 if configured
                if let Err(e) = delete_item_from_s3_if_active(&state, "schemas", &name).await {
                    tracing::warn!("Failed to delete schema from S3: {}", e);
                }
                return (StatusCode::OK, Json(ApiResponse::<()>::ok()));
            }
            Ok(false) => {
                return (StatusCode::NOT_FOUND, Json(ApiResponse::<()>::error("Schema not found")));
            }
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiResponse::<()>::error(e.to_string())),
                );
            }
        }
    }

    // Fallback to in-memory settings
    let mut settings = state.settings.write().await;

    let initial_len = settings.schemas.len();
    settings.schemas.retain(|s| s.name != name);

    if settings.schemas.len() < initial_len {
        drop(settings);
        // Auto-delete from S3 if configured
        if let Err(e) = delete_item_from_s3_if_active(&state, "schemas", &name).await {
            tracing::warn!("Failed to delete schema from S3: {}", e);
        }
        (StatusCode::OK, Json(ApiResponse::<()>::ok()))
    } else {
        (StatusCode::NOT_FOUND, Json(ApiResponse::<()>::error("Schema not found")))
    }
}

// ==================== Version History Endpoints ====================

/// Request for listing commits with pagination
#[derive(Debug, Clone, Deserialize)]
pub struct ListCommitsRequest {
    /// Maximum number of commits to return (default: 50)
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Offset for pagination (default: 0)
    #[serde(default)]
    pub offset: usize,
}

fn default_limit() -> usize {
    50
}

/// Request for creating a tag
#[derive(Debug, Clone, Deserialize)]
pub struct CreateTagRequest {
    /// Tag name (e.g., "v1.0", "production")
    pub name: String,
    /// Optional message describing the tag
    pub message: Option<String>,
}

/// Request for rollback
#[derive(Debug, Clone, Deserialize)]
pub struct RollbackRequest {
    /// Commit hash to rollback to
    pub commit_hash: String,
}

/// Database status response
#[derive(Debug, Clone, Serialize)]
pub struct DatabaseStatus {
    /// Whether database is enabled
    pub enabled: bool,
    /// Database backend type (sqlite, postgres, mysql)
    pub backend: Option<String>,
    /// Whether the database connection is healthy
    pub healthy: bool,
    /// Current HEAD commit (if any)
    pub head: Option<Commit>,
    /// Total number of commits
    pub total_commits: usize,
    /// Total number of tags
    pub total_tags: usize,
}

/// Get database status
pub async fn get_database_status(
    State(state): State<ApiState>,
) -> impl IntoResponse {
    if let Some(store) = &state.data_store {
        // Check health
        let healthy = store.pool().health_check().await.is_ok();
        let backend = store.pool().backend().name().to_string();

        // Get HEAD commit
        let head = match store.commits().get_head().await {
            Ok(h) => h,
            Err(_) => None,
        };

        // Get counts
        let total_commits = match store.commits().list_commits(10000, 0).await {
            Ok(commits) => commits.len(),
            Err(_) => 0,
        };

        let total_tags = match store.commits().list_tags().await {
            Ok(tags) => tags.len(),
            Err(_) => 0,
        };

        (
            StatusCode::OK,
            Json(ApiResponse::success(DatabaseStatus {
                enabled: true,
                backend: Some(backend),
                healthy,
                head,
                total_commits,
                total_tags,
            })),
        )
    } else {
        (
            StatusCode::OK,
            Json(ApiResponse::success(DatabaseStatus {
                enabled: false,
                backend: None,
                healthy: false,
                head: None,
                total_commits: 0,
                total_tags: 0,
            })),
        )
    }
}

/// List commits with pagination
pub async fn list_commits(
    State(state): State<ApiState>,
    axum::extract::Query(params): axum::extract::Query<ListCommitsRequest>,
) -> impl IntoResponse {
    if let Some(store) = &state.data_store {
        match store.commits().list_commits(params.limit, params.offset).await {
            Ok(commits) => (StatusCode::OK, Json(ApiResponse::success(commits))),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<Vec<Commit>>::error(e.to_string())),
            ),
        }
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiResponse::<Vec<Commit>>::error(
                "Database not configured. Version history requires database persistence.",
            )),
        )
    }
}

/// Get a specific commit by hash
pub async fn get_commit(
    State(state): State<ApiState>,
    Path(commit_hash): Path<String>,
) -> impl IntoResponse {
    if let Some(store) = &state.data_store {
        match store.commits().get_commit(&commit_hash).await {
            Ok(Some(commit)) => (StatusCode::OK, Json(ApiResponse::success(commit))),
            Ok(None) => (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<Commit>::error(format!(
                    "Commit not found: {}",
                    commit_hash
                ))),
            ),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<Commit>::error(e.to_string())),
            ),
        }
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiResponse::<Commit>::error(
                "Database not configured. Version history requires database persistence.",
            )),
        )
    }
}

/// Get changesets for a commit
pub async fn get_commit_changesets(
    State(state): State<ApiState>,
    Path(commit_hash): Path<String>,
) -> impl IntoResponse {
    if let Some(store) = &state.data_store {
        // First get the commit to get its ID
        match store.commits().get_commit(&commit_hash).await {
            Ok(Some(commit)) => {
                match store.commits().get_changesets(&commit.id).await {
                    Ok(changesets) => (StatusCode::OK, Json(ApiResponse::success(changesets))),
                    Err(e) => (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(ApiResponse::<Vec<Changeset>>::error(e.to_string())),
                    ),
                }
            }
            Ok(None) => (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<Vec<Changeset>>::error(format!(
                    "Commit not found: {}",
                    commit_hash
                ))),
            ),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<Vec<Changeset>>::error(e.to_string())),
            ),
        }
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiResponse::<Vec<Changeset>>::error(
                "Database not configured. Version history requires database persistence.",
            )),
        )
    }
}

/// Rollback to a specific commit
pub async fn rollback_to_commit(
    State(state): State<ApiState>,
    Json(request): Json<RollbackRequest>,
) -> impl IntoResponse {
    if let Some(store) = &state.data_store {
        match store.commits().rollback_to(&request.commit_hash).await {
            Ok(rollback_commit) => {
                // Notify MCP clients that lists have changed
                if let Some(broadcaster) = &state.broadcaster {
                    // Notify about all archetype types that may have changed
                    broadcaster.notify_tools_changed().await;
                    broadcaster.notify_resources_changed().await;
                    broadcaster.notify_prompts_changed().await;
                }
                (StatusCode::OK, Json(ApiResponse::success(rollback_commit)))
            }
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<Commit>::error(e.to_string())),
            ),
        }
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiResponse::<Commit>::error(
                "Database not configured. Version history requires database persistence.",
            )),
        )
    }
}

/// List all tags
pub async fn list_tags(
    State(state): State<ApiState>,
) -> impl IntoResponse {
    if let Some(store) = &state.data_store {
        match store.commits().list_tags().await {
            Ok(tags) => (StatusCode::OK, Json(ApiResponse::success(tags))),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<Vec<Tag>>::error(e.to_string())),
            ),
        }
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiResponse::<Vec<Tag>>::error(
                "Database not configured. Version history requires database persistence.",
            )),
        )
    }
}

/// Get a tag by name
pub async fn get_tag(
    State(state): State<ApiState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    if let Some(store) = &state.data_store {
        match store.commits().get_tag(&name).await {
            Ok(Some(tag)) => (StatusCode::OK, Json(ApiResponse::success(tag))),
            Ok(None) => (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<Tag>::error(format!("Tag not found: {}", name))),
            ),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<Tag>::error(e.to_string())),
            ),
        }
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiResponse::<Tag>::error(
                "Database not configured. Version history requires database persistence.",
            )),
        )
    }
}

/// Create a tag for a commit
pub async fn create_tag(
    State(state): State<ApiState>,
    Path(commit_hash): Path<String>,
    Json(request): Json<CreateTagRequest>,
) -> impl IntoResponse {
    if let Some(store) = &state.data_store {
        match store
            .commits()
            .create_tag(&request.name, &commit_hash, request.message.as_deref())
            .await
        {
            Ok(tag) => (StatusCode::CREATED, Json(ApiResponse::success(tag))),
            Err(e) => {
                // Check for specific error types
                let status = if e.to_string().contains("already exists") {
                    StatusCode::CONFLICT
                } else if e.to_string().contains("not found") {
                    StatusCode::NOT_FOUND
                } else {
                    StatusCode::INTERNAL_SERVER_ERROR
                };
                (status, Json(ApiResponse::<Tag>::error(e.to_string())))
            }
        }
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiResponse::<Tag>::error(
                "Database not configured. Version history requires database persistence.",
            )),
        )
    }
}

/// Delete a tag
pub async fn delete_tag(
    State(state): State<ApiState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    if let Some(store) = &state.data_store {
        match store.commits().delete_tag(&name).await {
            Ok(true) => (StatusCode::OK, Json(ApiResponse::<()>::ok())),
            Ok(false) => (
                StatusCode::NOT_FOUND,
                Json(ApiResponse::<()>::error(format!("Tag not found: {}", name))),
            ),
            Err(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(e.to_string())),
            ),
        }
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(ApiResponse::<()>::error(
                "Database not configured. Version history requires database persistence.",
            )),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_tool_dto_output_schema_serialization() {
        // Create a ToolDto with output_schema
        let tool = ToolDto {
            name: "test-tool".to_string(),
            description: "A test tool".to_string(),
            tags: vec![],
            input_schema: json!({"type": "object", "properties": {}}),
            output_schema: Some(json!({
                "type": "object",
                "properties": {
                    "result": {"type": "string"}
                }
            })),
            static_response: None,
            mock: None,
        };

        // Serialize to JSON Value (what happens before storing in DB)
        let value = serde_json::to_value(&tool).unwrap();

        // Verify output_schema is present
        assert!(value.get("output_schema").is_some(), "output_schema should be present in serialized JSON");

        // Deserialize back (what happens when retrieving from DB)
        let deserialized: ToolDto = serde_json::from_value(value).unwrap();

        // Verify output_schema is preserved
        assert!(deserialized.output_schema.is_some(), "output_schema should be preserved after deserialization");

        let out_schema = deserialized.output_schema.unwrap();
        assert!(out_schema.get("properties").is_some());
        assert!(out_schema["properties"].get("result").is_some());
    }

    #[test]
    fn test_tool_dto_output_schema_none() {
        // Create a ToolDto without output_schema
        let tool = ToolDto {
            name: "test-tool".to_string(),
            description: "A test tool".to_string(),
            tags: vec![],
            input_schema: json!({"type": "object", "properties": {}}),
            output_schema: None,
            static_response: None,
            mock: None,
        };

        // Serialize to JSON Value
        let value = serde_json::to_value(&tool).unwrap();

        // Verify output_schema is NOT present (skip_serializing_if works)
        assert!(value.get("output_schema").is_none(), "output_schema should be omitted when None");

        // Deserialize back
        let deserialized: ToolDto = serde_json::from_value(value).unwrap();

        // Verify output_schema is still None
        assert!(deserialized.output_schema.is_none(), "output_schema should remain None");
    }

    #[test]
    fn test_tool_dto_from_ui_json() {
        // Simulate JSON that would come from the UI
        let ui_json = r#"{
            "name": "test-tool",
            "description": "A test tool",
            "input_schema": {"type": "object", "properties": {}},
            "output_schema": {"type": "object", "properties": {"result": {"type": "string"}}}
        }"#;

        let tool: ToolDto = serde_json::from_str(ui_json).unwrap();

        assert!(tool.output_schema.is_some(), "output_schema should be parsed from UI JSON");

        let out_schema = tool.output_schema.unwrap();
        assert!(out_schema["properties"]["result"]["type"] == "string");
    }
}
