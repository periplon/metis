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
pub mod application;
pub mod cli;
pub mod config;
pub mod domain;

use crate::adapters::api_handler::{self, ApiState};
use crate::adapters::auth_middleware::{auth_middleware, AuthMiddleware, SharedAuthMiddleware};
use crate::adapters::health_handler::HealthHandler;
use crate::adapters::metrics_handler::MetricsHandler;
use crate::adapters::mock_strategy::MockStrategyHandler;
use crate::adapters::rmcp_server::MetisServer;
use crate::adapters::state_manager::StateManager;
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
) -> Router {
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

    // Create API state for REST endpoints
    let api_state = ApiState {
        settings: settings.clone(),
        state_manager,
        mock_strategy,
    };

    // API routes for Web UI
    let api_router = Router::new()
        // Config overview and settings
        .route("/config", get(api_handler::get_config_overview))
        .route("/config/settings", get(api_handler::get_server_settings).put(api_handler::update_server_settings))
        .route("/config/save-disk", post(api_handler::save_config_to_disk))
        .route("/config/save-s3", post(api_handler::save_config_to_s3))
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
        // State management
        .route("/state", get(api_handler::get_state).delete(api_handler::reset_state))
        .route("/state/:key", delete(api_handler::delete_state_key))
        .with_state(api_state);

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
