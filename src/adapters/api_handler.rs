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
use crate::adapters::secrets::SharedSecretsStore;
use crate::adapters::state_manager::StateManager;
use crate::adapters::tool_handler::AGENT_TOOL_PREFIX;
use crate::adapters::workflow_engine::WorkflowEngine;
use crate::agents::config::{
    AgentConfig, AgentReference, LlmProviderConfig, LlmProviderType, MemoryConfig,
    MergeStrategy, OrchestrationConfig, OrchestrationPattern,
};
use crate::agents::domain::{AgentPort, AgentType};
use crate::config::{
    MockConfig, PromptArgument, PromptConfig, PromptMessage, RateLimitConfig, ResourceConfig,
    ResourceTemplateConfig, Settings, ToolConfig, WorkflowConfig, WorkflowStep,
};
use crate::domain::ToolPort;

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
    /// MCP notification broadcaster for list change notifications
    pub broadcaster: Option<SharedNotificationBroadcaster>,
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

#[derive(Serialize)]
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

    // Check if config file exists (default: metis.toml)
    let config_file_loaded = std::path::Path::new("metis.toml").exists();

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

#[derive(Serialize, Deserialize, Clone)]
pub struct ServerSettingsDto {
    pub auth: AuthConfigDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<RateLimitConfigDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s3: Option<S3ConfigDto>,
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

    let response_dto = ServerSettingsDto {
        auth: AuthConfigDto::from(&settings.auth),
        rate_limit: settings.rate_limit.as_ref().map(RateLimitConfigDto::from),
        s3: settings.s3.as_ref().map(S3ConfigDto::from),
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
    let settings = state.settings.read().await;
    let resources: Vec<ResourceDto> = settings.resources.iter().map(ResourceDto::from).collect();
    (StatusCode::OK, Json(ApiResponse::success(resources)))
}

/// GET /api/resources/:uri - Get a single resource
pub async fn get_resource(
    State(state): State<ApiState>,
    Path(uri): Path<String>,
) -> impl IntoResponse {
    let settings = state.settings.read().await;
    let decoded_uri = urlencoding::decode(&uri).map(|s| s.into_owned()).unwrap_or(uri);

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

    // Notify connected MCP clients about the resource list change
    if let Some(broadcaster) = &state.broadcaster {
        broadcaster.notify_resources_changed().await;
    }

    (StatusCode::CREATED, Json(ApiResponse::success(dto)))
}

/// PUT /api/resources/:uri - Update a resource
pub async fn update_resource(
    State(state): State<ApiState>,
    Path(uri): Path<String>,
    Json(dto): Json<ResourceDto>,
) -> impl IntoResponse {
    let mut settings = state.settings.write().await;
    let decoded_uri = urlencoding::decode(&uri).map(|s| s.into_owned()).unwrap_or(uri);

    if let Some(resource) = settings.resources.iter_mut().find(|r| r.uri == decoded_uri) {
        resource.name = dto.name.clone();
        resource.description = dto.description.clone();
        resource.mime_type = dto.mime_type.clone();
        resource.output_schema = dto.output_schema.clone();
        resource.content = dto.content.clone();
        resource.mock = dto.mock.clone();
        drop(settings);

        // Notify connected MCP clients about the resource list change
        if let Some(broadcaster) = &state.broadcaster {
            broadcaster.notify_resources_changed().await;
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
    let mut settings = state.settings.write().await;
    let decoded_uri = urlencoding::decode(&uri).map(|s| s.into_owned()).unwrap_or(uri);

    let initial_len = settings.resources.len();
    settings.resources.retain(|r| r.uri != decoded_uri);

    if settings.resources.len() < initial_len {
        drop(settings);

        // Notify connected MCP clients about the resource list change
        if let Some(broadcaster) = &state.broadcaster {
            broadcaster.notify_resources_changed().await;
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
    let settings = state.settings.read().await;
    let tools: Vec<ToolDto> = settings.tools.iter().map(ToolDto::from).collect();
    (StatusCode::OK, Json(ApiResponse::success(tools)))
}

/// GET /api/tools/:name - Get a single tool
pub async fn get_tool(
    State(state): State<ApiState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
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
    let mut settings = state.settings.write().await;

    if let Some(tool) = settings.tools.iter_mut().find(|t| t.name == name) {
        tool.description = dto.description.clone();
        tool.input_schema = dto.input_schema.clone();
        tool.static_response = dto.static_response.clone();
        tool.mock = dto.mock.clone();
        drop(settings);

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
    let mut settings = state.settings.write().await;

    let initial_len = settings.tools.len();
    settings.tools.retain(|t| t.name != name);

    if settings.tools.len() < initial_len {
        drop(settings);

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
    let settings = state.settings.read().await;
    let prompts: Vec<PromptDto> = settings.prompts.iter().map(PromptDto::from).collect();
    (StatusCode::OK, Json(ApiResponse::success(prompts)))
}

/// GET /api/prompts/:name - Get a single prompt
pub async fn get_prompt(
    State(state): State<ApiState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
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
    let mut settings = state.settings.write().await;

    if let Some(prompt) = settings.prompts.iter_mut().find(|p| p.name == name) {
        prompt.description = dto.description.clone();
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
    let mut settings = state.settings.write().await;

    let initial_len = settings.prompts.len();
    settings.prompts.retain(|p| p.name != name);

    if settings.prompts.len() < initial_len {
        drop(settings);

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
    let settings = state.settings.read().await;
    let workflows: Vec<WorkflowDto> = settings.workflows.iter().map(WorkflowDto::from).collect();
    (StatusCode::OK, Json(ApiResponse::success(workflows)))
}

/// GET /api/workflows/:name - Get a single workflow
pub async fn get_workflow(
    State(state): State<ApiState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
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
    let mut settings = state.settings.write().await;

    if let Some(workflow) = settings.workflows.iter_mut().find(|w| w.name == name) {
        workflow.name = dto.name.clone();
        workflow.description = dto.description.clone();
        workflow.input_schema = dto.input_schema.clone();
        workflow.steps = dto.steps.clone().into_iter().map(WorkflowStep::from).collect();
        workflow.on_error = dto.on_error.clone();
        drop(settings);

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
    let mut settings = state.settings.write().await;

    let initial_len = settings.workflows.len();
    settings.workflows.retain(|w| w.name != name);

    if settings.workflows.len() < initial_len {
        drop(settings);

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
    let settings = state.settings.read().await;
    let decoded_uri = urlencoding::decode(&uri_template)
        .map(|s| s.into_owned())
        .unwrap_or(uri_template);

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

    // Resource templates affect resource list
    if let Some(broadcaster) = &state.broadcaster {
        broadcaster.notify_resources_changed().await;
    }

    (StatusCode::CREATED, Json(ApiResponse::success(dto)))
}

/// PUT /api/resource-templates/:uri_template - Update a resource template
pub async fn update_resource_template(
    State(state): State<ApiState>,
    Path(uri_template): Path<String>,
    Json(dto): Json<ResourceTemplateDto>,
) -> impl IntoResponse {
    let mut settings = state.settings.write().await;
    let decoded_uri = urlencoding::decode(&uri_template)
        .map(|s| s.into_owned())
        .unwrap_or(uri_template);

    if let Some(template) = settings
        .resource_templates
        .iter_mut()
        .find(|r| r.uri_template == decoded_uri)
    {
        template.name = dto.name.clone();
        template.description = dto.description.clone();
        template.mime_type = dto.mime_type.clone();
        template.input_schema = dto.input_schema.clone();
        template.output_schema = dto.output_schema.clone();
        template.content = dto.content.clone();
        template.mock = dto.mock.clone();
        drop(settings);

        // Resource templates affect resource list
        if let Some(broadcaster) = &state.broadcaster {
            broadcaster.notify_resources_changed().await;
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
    let mut settings = state.settings.write().await;
    let decoded_uri = urlencoding::decode(&uri_template)
        .map(|s| s.into_owned())
        .unwrap_or(uri_template);

    let initial_len = settings.resource_templates.len();
    settings
        .resource_templates
        .retain(|r| r.uri_template != decoded_uri);

    if settings.resource_templates.len() < initial_len {
        drop(settings);

        // Resource templates affect resource list
        if let Some(broadcaster) = &state.broadcaster {
            broadcaster.notify_resources_changed().await;
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
        match state
            .mock_strategy
            .generate(mock_config, Some(&req.args))
            .await
        {
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

/// POST /api/config/save-disk - Save current configuration to metis.toml
pub async fn save_config_to_disk(
    State(state): State<ApiState>,
) -> impl IntoResponse {
    let settings = state.settings.read().await;

    // Serialize settings to TOML
    let toml_content = match toml::to_string_pretty(&*settings) {
        Ok(content) => content,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(format!("Failed to serialize config: {}", e)))
            );
        }
    };

    // Write to metis.toml
    if let Err(e) = std::fs::write("metis.toml", toml_content) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(format!("Failed to write metis.toml: {}", e)))
        );
    }

    (StatusCode::OK, Json(ApiResponse::<()>::ok()))
}

/// POST /api/config/save-s3 - Save current configuration to S3
pub async fn save_config_to_s3(
    State(state): State<ApiState>,
) -> impl IntoResponse {
    let settings = state.settings.read().await;

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
                Json(ApiResponse::<()>::error(msg))
            );
        }
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiResponse::<()>::error(
                    "S3 is not configured. Please configure S3 settings (enable S3, set bucket name, region) and save to disk first."
                ))
            );
        }
    };

    // Serialize settings to TOML
    let toml_content = match toml::to_string_pretty(&*settings) {
        Ok(content) => content,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(format!("Failed to serialize config: {}", e)))
            );
        }
    };

    // Drop the read lock before async S3 operations
    drop(settings);

    // Build S3 client
    let sdk_config = match build_s3_config(&s3_config).await {
        Ok(cfg) => cfg,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiResponse::<()>::error(format!("Failed to configure S3 client: {}", e)))
            );
        }
    };
    let client = aws_sdk_s3::Client::new(&sdk_config);

    // Upload to S3
    let bucket = s3_config.bucket.as_ref().unwrap();
    let key = format!("{}metis.toml", s3_config.get_prefix());

    match client
        .put_object()
        .bucket(bucket)
        .key(&key)
        .body(toml_content.into_bytes().into())
        .content_type("application/toml")
        .send()
        .await
    {
        Ok(_) => (StatusCode::OK, Json(ApiResponse::<()>::ok())),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(format!("Failed to upload to S3: {}", e)))
        ),
    }
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
async fn build_s3_config(config: &crate::config::s3::S3Config) -> anyhow::Result<aws_config::SdkConfig> {
    use aws_config::BehaviorVersion;

    let mut loader = aws_config::defaults(BehaviorVersion::latest());

    if let Some(region) = &config.region {
        loader = loader.region(aws_config::Region::new(region.clone()));
    }

    if let Some(endpoint) = &config.endpoint {
        loader = loader.endpoint_url(endpoint);
    }

    Ok(loader.load().await)
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
        match state.mock_strategy.generate(mock_config, None).await {
            Ok(result) => {
                // Return as structured content
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
    let settings = state.settings.read().await;
    let agents: Vec<AgentDto> = settings.agents.iter().map(AgentDto::from).collect();
    (StatusCode::OK, Json(ApiResponse::success(agents)))
}

/// GET /api/agents/:name - Get a single agent
pub async fn get_agent(
    State(state): State<ApiState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
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
    let mut settings = state.settings.write().await;

    // Check for duplicate name
    if settings.agents.iter().any(|a| a.name == dto.name) {
        return (
            StatusCode::CONFLICT,
            Json(ApiResponse::<AgentDto>::error("Agent with this name already exists")),
        );
    }

    let agent: AgentConfig = dto.into();
    settings.agents.push(agent.clone());
    drop(settings);

    // Reset cached agent handler so it re-initializes with new agent
    *state.test_agent_handler.write().await = None;

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
    let mut settings = state.settings.write().await;

    if let Some(agent) = settings.agents.iter_mut().find(|a| a.name == name) {
        *agent = dto.into();
        let result = AgentDto::from(&*agent);
        drop(settings);

        // Reset cached agent handler so it re-initializes with updated agent
        *state.test_agent_handler.write().await = None;

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
    let mut settings = state.settings.write().await;

    let initial_len = settings.agents.len();
    settings.agents.retain(|a| a.name != name);

    if settings.agents.len() < initial_len {
        drop(settings);

        // Reset cached agent handler so it re-initializes without deleted agent
        *state.test_agent_handler.write().await = None;

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
    let settings = state.settings.read().await;
    let orchestrations: Vec<OrchestrationDto> = settings.orchestrations.iter().map(OrchestrationDto::from).collect();
    (StatusCode::OK, Json(ApiResponse::success(orchestrations)))
}

/// GET /api/orchestrations/:name - Get a single orchestration
pub async fn get_orchestration(
    State(state): State<ApiState>,
    Path(name): Path<String>,
) -> impl IntoResponse {
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
