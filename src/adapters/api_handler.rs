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

use crate::adapters::state_manager::StateManager;
use crate::config::{
    MockConfig, PromptArgument, PromptConfig, PromptMessage, RateLimitConfig, ResourceConfig,
    Settings, ToolConfig, WorkflowConfig, WorkflowStep,
};

/// Shared application state for API handlers
#[derive(Clone)]
pub struct ApiState {
    pub settings: Arc<RwLock<Settings>>,
    pub state_manager: Arc<StateManager>,
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
    pub tools_count: usize,
    pub prompts_count: usize,
    pub workflows_count: usize,
    pub auth_enabled: bool,
    pub rate_limit_enabled: bool,
    pub s3_enabled: bool,
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

#[derive(Serialize, Deserialize, Clone)]
pub struct ResourceDto {
    pub uri: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
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

    let overview = ConfigOverview {
        server: ServerInfo {
            host: settings.server.host.clone(),
            port: settings.server.port,
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
        resources_count: settings.resources.len(),
        tools_count: settings.tools.len(),
        prompts_count: settings.prompts.len(),
        workflows_count: settings.workflows.len(),
        auth_enabled: settings.auth.enabled,
        rate_limit_enabled: settings.rate_limit.as_ref().is_some_and(|r| r.enabled),
        s3_enabled: settings.s3.as_ref().is_some_and(|s| s.enabled),
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
pub struct ServerSettingsDto {
    pub auth: AuthConfigDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<RateLimitConfigDto>,
}

/// GET /api/config/settings - Get editable server settings
pub async fn get_server_settings(
    State(state): State<ApiState>,
) -> impl IntoResponse {
    let settings = state.settings.read().await;

    let dto = ServerSettingsDto {
        auth: AuthConfigDto::from(&settings.auth),
        rate_limit: settings.rate_limit.as_ref().map(RateLimitConfigDto::from),
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

    let response_dto = ServerSettingsDto {
        auth: AuthConfigDto::from(&settings.auth),
        rate_limit: settings.rate_limit.as_ref().map(RateLimitConfigDto::from),
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
        resource.content = dto.content.clone();
        resource.mock = dto.mock.clone();
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
        workflow.description = dto.description.clone();
        workflow.input_schema = dto.input_schema.clone();
        workflow.steps = dto.steps.clone().into_iter().map(WorkflowStep::from).collect();
        workflow.on_error = dto.on_error.clone();
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
        (StatusCode::OK, Json(ApiResponse::<()>::ok()))
    } else {
        (StatusCode::NOT_FOUND, Json(ApiResponse::<()>::error("Workflow not found")))
    }
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
