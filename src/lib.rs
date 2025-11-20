pub mod adapters;
pub mod application;
pub mod config;
pub mod domain;

use crate::adapters::mcp_protocol_handler::{McpProtocolHandler, JsonRpcRequest, JsonRpcResponse};
use axum::{
    routing::{get, post},
    Router,
    extract::State,
    Json,
};
use std::sync::Arc;

pub fn create_app(protocol_handler: Arc<McpProtocolHandler>) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/mcp", post(handle_mcp))
        .with_state(protocol_handler)
}

async fn health_check() -> &'static str {
    "OK"
}

async fn handle_mcp(
    State(handler): State<Arc<McpProtocolHandler>>,
    Json(request): Json<JsonRpcRequest>,
) -> Json<JsonRpcResponse> {
    let response = handler.handle_request(request).await;
    Json(response)
}
