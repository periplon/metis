//! # Metis - MCP Mock Server
//!
//! Metis is a high-performance, configurable Model Context Protocol (MCP) mock server
//! built in Rust. It provides multiple mock strategies for testing and development.
//!
//! ## Features
//!
//! - **9 Mock Strategies**: Static, Template, Random, Stateful, Script, File, Pattern, LLM, Database
//! - **Authentication**: API Key and JWT Bearer Token support
//! - **Metrics**: Prometheus metrics for monitoring
//! - **Health Checks**: Kubernetes-ready health endpoints
//! - **Live Reload**: Automatic configuration reloading
//! - **Validation**: Comprehensive configuration validation
//! - **Standards-compliant**: Uses official rmcp SDK for MCP protocol
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use metis::config::Settings;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Load configuration
//!     let settings = Settings::new()?;
//!
//!     // Server will start on configured host:port
//!     Ok(())
//! }
//! ```
//!
//! ## Architecture
//!
//! Metis follows Hexagonal Architecture:
//! - **Domain**: Core business logic and types
//! - **Application**: Use cases and ports
//! - **Adapters**: External integrations (handlers, strategies)
//! - **Config**: Configuration management

pub mod adapters;
pub mod agents;
pub mod application;
pub mod cli;
pub mod config;
pub mod domain;

use crate::adapters::api_handler::{self, ApiState, SecretsApiState};
use crate::adapters::auth_middleware::{auth_middleware, AuthMiddleware, SharedAuthMiddleware};
use crate::adapters::health_handler::HealthHandler;
use crate::adapters::metrics_handler::MetricsHandler;
use crate::adapters::mock_strategy::MockStrategyHandler;
use crate::adapters::rmcp_server::MetisServer;
use crate::adapters::secrets::SharedSecretsStore;
use crate::adapters::state_manager::StateManager;
use crate::agents::domain::AgentPort;
use crate::agents::handler::AgentHandler;
use axum::{routing::{delete, get, post}, Router};
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Creates the Axum application router with all endpoints configured.
///
/// # Arguments
///
/// * `metis_server` - MCP server implementation using rmcp SDK
/// * `health_handler` - Health check handler
/// * `metrics_handler` - Metrics collection handler
/// * `settings` - Application settings
/// * `state_manager` - State manager for stateful mocks
/// * `secrets_store` - In-memory secrets store for API keys
/// * `tool_handler` - Tool handler for agents (used to reinitialize when API keys change)
///
/// # Returns
///
/// Configured Axum Router
pub async fn create_app(
    metis_server: MetisServer,
    health_handler: Arc<HealthHandler>,
    metrics_handler: Arc<MetricsHandler>,
    settings: Arc<RwLock<crate::config::Settings>>,
    state_manager: Arc<StateManager>,
    secrets_store: SharedSecretsStore,
    tool_handler: Arc<crate::adapters::tool_handler::BasicToolHandler>,
) -> Router {
    // Get the broadcaster before moving metis_server into the closure
    let broadcaster = metis_server.broadcaster().clone();

    // Create rmcp HTTP transport service
    let session_manager = Arc::new(LocalSessionManager::default());
    let config = StreamableHttpServerConfig::default();
    let mcp_service = StreamableHttpService::new(
        move || Ok(metis_server.clone()),
        session_manager,
        config,
    );

    // Public routes (no authentication required)
    let public_router = Router::new()
        // Health check endpoints
        .route("/health", get({
            let handler = health_handler.clone();
            move || {
                let h = handler.clone();
                async move { h.health().await }
            }
        }))
        .route("/health/ready", get({
            let handler = health_handler.clone();
            move || {
                let h = handler.clone();
                async move { h.ready().await }
            }
        }))
        .route("/health/live", get({
            let handler = health_handler.clone();
            move || {
                let h = handler.clone();
                async move { h.live().await }
            }
        }));

    // Protected routes (authentication applied when enabled)
    let protected_router = Router::new()
        // Metrics endpoint
        .route("/metrics", get({
            let handler = metrics_handler.clone();
            move || {
                let h = handler.clone();
                async move { h.metrics().await }
            }
        }))
        // MCP protocol endpoint using rmcp streamable HTTP transport
        .nest_service("/mcp", mcp_service);

    // Create mock strategy handler for test endpoints
    let mock_strategy = Arc::new(MockStrategyHandler::new(state_manager.clone()));

    // Try to create agent handler if agents are configured
    let agent_handler: Option<Arc<dyn AgentPort>> = {
        let settings_read = settings.read().await;
        if !settings_read.agents.is_empty() {
            // Create a tool handler that uses mock strategies for agent tool calls
            let tool_handler = Arc::new(crate::adapters::tool_handler::BasicToolHandler::new(
                settings.clone(),
                mock_strategy.clone(),
            ));

            let handler = AgentHandler::new_with_secrets(settings.clone(), tool_handler, secrets_store.clone());

            // Initialize agents - this loads them into memory
            if let Err(e) = handler.initialize().await {
                tracing::warn!("Failed to initialize agents: {}", e);
            } else {
                tracing::info!("AgentHandler initialized with {} agents", settings_read.agents.len());
            }

            Some(Arc::new(handler) as Arc<dyn AgentPort>)
        } else {
            None
        }
    };

    // Create shared test agent handler (shared between ApiState and SecretsApiState)
    let test_agent_handler: Arc<tokio::sync::RwLock<Option<Arc<dyn AgentPort>>>> =
        Arc::new(tokio::sync::RwLock::new(None));

    // Create API state for REST endpoints
    let api_state = ApiState {
        settings: settings.clone(),
        state_manager,
        mock_strategy,
        agent_handler,
        test_agent_handler: test_agent_handler.clone(),
        secrets: secrets_store.clone(),
        broadcaster: Some(broadcaster.clone()),
    };

    // API routes for Web UI
    let api_router = Router::new()
        // Config overview and settings
        .route("/config", get(api_handler::get_config_overview))
        .route("/config/settings", get(api_handler::get_server_settings).put(api_handler::update_server_settings))
        .route("/config/save-disk", post(api_handler::save_config_to_disk))
        .route("/config/save-s3", post(api_handler::save_config_to_s3))
        .route("/config/export", get(api_handler::export_config))
        .route("/config/import", post(api_handler::import_config))
        .route("/config/merge", post(api_handler::merge_config))
        .route("/metrics/json", get(api_handler::get_metrics_json))
        // Resources CRUD + Test
        .route("/resources", get(api_handler::list_resources).post(api_handler::create_resource))
        .route("/resources/:uri", get(api_handler::get_resource).put(api_handler::update_resource).delete(api_handler::delete_resource))
        .route("/resources/:uri/test", post(api_handler::test_resource))
        // Tools CRUD + Test
        .route("/tools", get(api_handler::list_tools).post(api_handler::create_tool))
        .route("/tools/:name", get(api_handler::get_tool).put(api_handler::update_tool).delete(api_handler::delete_tool))
        .route("/tools/:name/test", post(api_handler::test_tool))
        // Prompts CRUD + Test
        .route("/prompts", get(api_handler::list_prompts).post(api_handler::create_prompt))
        .route("/prompts/:name", get(api_handler::get_prompt).put(api_handler::update_prompt).delete(api_handler::delete_prompt))
        .route("/prompts/:name/test", post(api_handler::test_prompt))
        // Workflows CRUD + Test
        .route("/workflows", get(api_handler::list_workflows).post(api_handler::create_workflow))
        .route("/workflows/:name", get(api_handler::get_workflow).put(api_handler::update_workflow).delete(api_handler::delete_workflow))
        .route("/workflows/:name/test", post(api_handler::test_workflow))
        // Resource Templates CRUD + Test
        .route("/resource-templates", get(api_handler::list_resource_templates).post(api_handler::create_resource_template))
        .route("/resource-templates/:uri_template", get(api_handler::get_resource_template).put(api_handler::update_resource_template).delete(api_handler::delete_resource_template))
        .route("/resource-templates/:uri_template/test", post(api_handler::test_resource_template))
        // State management
        .route("/state", get(api_handler::get_state).delete(api_handler::reset_state))
        .route("/state/:key", delete(api_handler::delete_state_key))
        // Agents CRUD + Test
        .route("/agents", get(api_handler::list_agents).post(api_handler::create_agent))
        .route("/agents/:name", get(api_handler::get_agent).put(api_handler::update_agent).delete(api_handler::delete_agent))
        .route("/agents/:name/test", post(api_handler::test_agent))
        // Orchestrations CRUD + Test
        .route("/orchestrations", get(api_handler::list_orchestrations).post(api_handler::create_orchestration))
        .route("/orchestrations/:name", get(api_handler::get_orchestration).put(api_handler::update_orchestration).delete(api_handler::delete_orchestration))
        .route("/orchestrations/:name/test", post(api_handler::test_orchestration))
        // LLM models discovery
        .route("/llm/models/:provider", get(api_handler::fetch_llm_models))
        .with_state(api_state);

    // Secrets API routes (separate state for secrets store, but shares test_agent_handler and broadcaster)
    let secrets_state = SecretsApiState {
        secrets: secrets_store,
        test_agent_handler,
        broadcaster: Some(broadcaster),
        tool_handler: Some(tool_handler),
    };
    let secrets_router = Router::new()
        .route("/secrets", get(api_handler::list_secrets).delete(api_handler::clear_secrets))
        .route("/secrets/:key", post(api_handler::set_secret).delete(api_handler::delete_secret))
        .with_state(secrets_state);

    // Merge API routers
    let api_router = api_router.merge(secrets_router);

    // Build protected router with API routes
    let mut protected_router = protected_router
        .nest("/api", api_router)
        // UI endpoint (catch-all for SPA)
        .fallback(crate::adapters::ui_handler::UIHandler::serve);

    // Apply Rate Limiting to protected routes if enabled
    let settings_read = settings.read().await;
    if let Some(rate_limit) = &settings_read.rate_limit {
        if rate_limit.enabled {
            let limiter = crate::adapters::rate_limit::create_limiter(
                rate_limit.requests_per_second,
                rate_limit.burst_size,
            );

            protected_router = protected_router.layer(axum::middleware::from_fn_with_state(
                limiter,
                crate::adapters::rate_limit::rate_limit_middleware,
            ));
        }
    }

    // Apply Authentication middleware to protected routes if enabled
    if settings_read.auth.enabled {
        let auth: SharedAuthMiddleware = Arc::new(AuthMiddleware::new(Arc::new(settings_read.auth.clone())));
        protected_router = protected_router.layer(axum::middleware::from_fn_with_state(auth, auth_middleware));
    }

    // Merge public and protected routers
    // Public routes are checked first, then protected routes
    let router = public_router.merge(protected_router);

    router.layer(
        tower_http::cors::CorsLayer::new()
            .allow_origin(tower_http::cors::Any)
            .allow_methods(tower_http::cors::Any)
            .allow_headers(tower_http::cors::Any),
    )
}
