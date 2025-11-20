use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use metis::adapters::logging_handler::LoggingHandler;
use metis::adapters::mcp_protocol_handler::McpProtocolHandler;
use metis::adapters::mock_strategy::MockStrategyHandler;
use metis::adapters::prompt_handler::InMemoryPromptHandler;
use metis::adapters::resource_handler::InMemoryResourceHandler;
use metis::adapters::state_manager::StateManager;
use metis::adapters::tool_handler::BasicToolHandler;
// use metis::config::{Settings, ServerSettings}; // Not used in this test
use serde_json::{json, Value};
use std::sync::Arc;
use tower::util::ServiceExt; // Correct import for oneshot

#[tokio::test]
async fn test_api_integration() {
    // Setup application
    let state_manager = Arc::new(StateManager::new());
    let mock_strategy = Arc::new(MockStrategyHandler::new(state_manager));
    let resource_handler = Arc::new(InMemoryResourceHandler::new(vec![], mock_strategy.clone()));
    let tool_handler = Arc::new(BasicToolHandler::new(vec![], mock_strategy.clone()));
    let prompt_handler = Arc::new(InMemoryPromptHandler::new(vec![]));
    let logging_handler = Arc::new(LoggingHandler::new());
    let protocol_handler = Arc::new(McpProtocolHandler::new(
        resource_handler,
        tool_handler,
        prompt_handler,
        logging_handler,
    ));

    let app = metis::create_app(protocol_handler);

    // Test Initialize
    let request = Request::builder()
        .uri("/mcp")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(json!({
            "jsonrpc": "2.0",
            "method": "initialize",
            "params": {
                "protocol_version": "2024-11-05",
                "capabilities": {},
                "client_info": { "name": "test", "version": "1.0" }
            },
            "id": 1
        }).to_string()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_json: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(body_json["result"]["protocol_version"].as_str().is_some());

    // Test Ping
    let request = Request::builder()
        .uri("/mcp")
        .method("POST")
        .header("Content-Type", "application/json")
        .body(Body::from(json!({
            "jsonrpc": "2.0",
            "method": "ping",
            "id": 2
        }).to_string()))
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    
    let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body_json: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert!(body_json["result"].as_object().is_some());
}
