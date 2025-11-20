//! # Metis - MCP Mock Server
//!
//! Metis is a high-performance, configurable Model Context Protocol (MCP) mock server
//! built in Rust. It provides multiple mock strategies for testing and development.
//!
//! ## Features
//!
//! - **7 Mock Strategies**: Static, Template, Random, Stateful, Script, File, Pattern
//! - **Authentication**: API Key and JWT Bearer Token support
//! - **Metrics**: Prometheus metrics for monitoring
//! - **Health Checks**: Kubernetes-ready health endpoints
//! - **Live Reload**: Automatic configuration reloading
//! - **Validation**: Comprehensive configuration validation
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
pub mod config;
pub mod domain;

use crate::adapters::mcp_protocol_handler::{McpProtocolHandler, JsonRpcRequest, JsonRpcResponse};
use crate::adapters::health_handler::HealthHandler;
use crate::adapters::metrics_handler::MetricsHandler;
use axum::{
    routing::{get, post},
    Router,
    extract::State,
    Json,
};
use std::sync::Arc;

/// Creates the Axum application router with all endpoints configured.
///
/// # Arguments
///
/// * `protocol_handler` - MCP protocol handler
/// * `health_handler` - Health check handler
/// * `metrics_handler` - Metrics collection handler
///
/// # Returns
///
/// Configured Axum Router
pub fn create_app(
    protocol_handler: Arc<McpProtocolHandler>,
    health_handler: Arc<HealthHandler>,
    metrics_handler: Arc<MetricsHandler>,
) -> Router {
    Router::new()
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
        // MCP protocol endpoint
        .route("/mcp", post(handle_mcp))
        // UI endpoint (catch-all for SPA)
        .fallback(crate::adapters::ui_handler::UIHandler::serve)
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_origin(tower_http::cors::Any)
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any),
        )
        .with_state(protocol_handler)
}

async fn handle_mcp(
    State(handler): State<Arc<McpProtocolHandler>>,
    Json(request): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    let response = handler.handle_request(request).await;
    Json(response)
}
