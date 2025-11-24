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

use crate::adapters::health_handler::HealthHandler;
use crate::adapters::metrics_handler::MetricsHandler;
use crate::adapters::rmcp_server::MetisServer;
use axum::{routing::get, Router};
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
///
/// # Returns
///
/// Configured Axum Router
pub async fn create_app(
    metis_server: MetisServer,
    health_handler: Arc<HealthHandler>,
    metrics_handler: Arc<MetricsHandler>,
    settings: Arc<RwLock<crate::config::Settings>>,
) -> Router {
    // Create rmcp HTTP transport service
    let session_manager = Arc::new(LocalSessionManager::default());
    let config = StreamableHttpServerConfig::default();
    let mcp_service = StreamableHttpService::new(
        move || Ok(metis_server.clone()),
        session_manager,
        config,
    );

    let mut router = Router::new()
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
        }))
        // Metrics endpoint
        .route("/metrics", get({
            let handler = metrics_handler.clone();
            move || {
                let h = handler.clone();
                async move { h.metrics().await }
            }
        }))
        // MCP protocol endpoint using rmcp streamable HTTP transport
        .nest_service("/mcp", mcp_service)
        // UI endpoint (catch-all for SPA)
        .fallback(crate::adapters::ui_handler::UIHandler::serve);

    // Apply Rate Limiting if enabled (before state layer)
    let settings_read = settings.read().await;
    if let Some(rate_limit) = &settings_read.rate_limit {
        if rate_limit.enabled {
            let limiter = crate::adapters::rate_limit::create_limiter(
                rate_limit.requests_per_second,
                rate_limit.burst_size,
            );

            router = router.layer(axum::middleware::from_fn_with_state(
                limiter,
                crate::adapters::rate_limit::rate_limit_middleware,
            ));
        }
    }

    router.layer(
        tower_http::cors::CorsLayer::new()
            .allow_origin(tower_http::cors::Any)
            .allow_methods(tower_http::cors::Any)
            .allow_headers(tower_http::cors::Any),
    )
}
