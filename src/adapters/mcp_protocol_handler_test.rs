use super::mcp_protocol_handler::{JsonRpcRequest, McpProtocolHandler};
use crate::adapters::logging_handler::LoggingHandler;
use crate::domain::{PromptPort, ResourcePort, ToolPort, Resource, Tool, Prompt, ResourceReadResult, GetPromptResult};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

struct MockResourceHandler;
#[async_trait]
impl ResourcePort for MockResourceHandler {
    async fn get_resource(&self, uri: &str) -> anyhow::Result<ResourceReadResult> {
        Ok(ResourceReadResult {
            uri: uri.to_string(),
            mime_type: Some("text/plain".to_string()),
            content: "mock resource".to_string(),
        })
    }
    async fn list_resources(&self) -> anyhow::Result<Vec<Resource>> {
        Ok(vec![Resource {
            uri: "mock".to_string(),
            name: "mock".to_string(),
            description: None,
            mime_type: None,
        }])
    }
}

struct MockToolHandler;
#[async_trait]
impl ToolPort for MockToolHandler {
    async fn execute_tool(&self, _name: &str, _args: Value) -> anyhow::Result<Value> {
        Ok(json!({ "result": "mock tool" }))
    }
    async fn list_tools(&self) -> anyhow::Result<Vec<Tool>> {
        Ok(vec![Tool {
            name: "mock".to_string(),
            description: "mock".to_string(),
            input_schema: json!({}),
        }])
    }
}

struct MockPromptHandler;
#[async_trait]
impl PromptPort for MockPromptHandler {
    async fn get_prompt(&self, _name: &str, _arguments: Option<Value>) -> anyhow::Result<GetPromptResult> {
        Ok(GetPromptResult {
            description: Some("mock prompt".to_string()),
            messages: vec![],
        })
    }
    async fn list_prompts(&self) -> anyhow::Result<Vec<Prompt>> {
        Ok(vec![Prompt {
            name: "mock".to_string(),
            description: "mock".to_string(),
            arguments: None,
        }])
    }
}

#[tokio::test]
async fn test_initialize() {
    let handler = McpProtocolHandler::new(
        Arc::new(MockResourceHandler),
        Arc::new(MockToolHandler),
        Arc::new(MockPromptHandler),
        Arc::new(LoggingHandler::new()),
    );

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "initialize".to_string(),
        params: Some(json!({
            "protocol_version": "2024-11-05",
            "capabilities": {},
            "client_info": { "name": "test", "version": "1.0" }
        })),
        id: Some(json!(1)),
    };

    let response = handler.handle_request(request).await;
    assert!(response.error.is_none());
    assert!(response.result.is_some());
    let result = response.result.unwrap();
    assert_eq!(result["protocol_version"], "2024-11-05");
}

#[tokio::test]
async fn test_ping() {
    let handler = McpProtocolHandler::new(
        Arc::new(MockResourceHandler),
        Arc::new(MockToolHandler),
        Arc::new(MockPromptHandler),
        Arc::new(LoggingHandler::new()),
    );

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "ping".to_string(),
        params: None,
        id: Some(json!(1)),
    };

    let response = handler.handle_request(request).await;
    assert!(response.error.is_none());
    assert!(response.result.is_some());
}

#[tokio::test]
async fn test_resources_list() {
    let handler = McpProtocolHandler::new(
        Arc::new(MockResourceHandler),
        Arc::new(MockToolHandler),
        Arc::new(MockPromptHandler),
        Arc::new(LoggingHandler::new()),
    );

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "resources/list".to_string(),
        params: None,
        id: Some(json!(1)),
    };

    let response = handler.handle_request(request).await;
    assert!(response.error.is_none());
    let result = response.result.unwrap();
    assert!(result["resources"].is_array());
}

#[tokio::test]
async fn test_method_not_found() {
    let handler = McpProtocolHandler::new(
        Arc::new(MockResourceHandler),
        Arc::new(MockToolHandler),
        Arc::new(MockPromptHandler),
        Arc::new(LoggingHandler::new()),
    );

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "unknown".to_string(),
        params: None,
        id: Some(json!(1)),
    };

    let response = handler.handle_request(request).await;
    assert!(response.error.is_some());
    assert_eq!(response.error.unwrap().code, -32601);
}
