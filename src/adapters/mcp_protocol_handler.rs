use crate::adapters::logging_handler::{LogMessage, LoggingHandler};
use crate::domain::mcp_types::{
    Implementation, InitializeRequest, InitializeResult, ServerCapabilities,
};
use crate::domain::{PromptPort, ResourcePort, ToolPort};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{error, info, warn};

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
    pub id: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub result: Option<Value>,
    pub error: Option<JsonRpcError>,
    pub id: Option<Value>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

pub struct McpProtocolHandler {
    resource_handler: Arc<dyn ResourcePort>,
    tool_handler: Arc<dyn ToolPort>,
    prompt_handler: Arc<dyn PromptPort>,
    logging_handler: Arc<LoggingHandler>,
}

impl McpProtocolHandler {
    pub fn new(
        resource_handler: Arc<dyn ResourcePort>,
        tool_handler: Arc<dyn ToolPort>,
        prompt_handler: Arc<dyn PromptPort>,
        logging_handler: Arc<LoggingHandler>,
    ) -> Self {
        Self {
            resource_handler,
            tool_handler,
            prompt_handler,
            logging_handler,
        }
    }

    pub async fn handle_request(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        // Handle notifications (no id)
        if request.id.is_none() {
            self.handle_notification(&request).await;
            return JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: None,
                id: None,
            };
        }

        let id = request.id.clone();
        let result = match request.method.as_str() {
            "initialize" => self.handle_initialize(request.params).await,
            "ping" => Ok(json!({})),
            "resources/list" => self
                .resource_handler
                .list_resources()
                .await
                .map(|r| json!({ "resources": r })),
            "resources/read" => {
                if let Some(params) = request.params {
                    if let Some(uri) = params.get("uri").and_then(|v| v.as_str()) {
                        self.resource_handler
                            .get_resource(uri)
                            .await
                            .map(|r| json!({ "contents": [r] }))
                    } else {
                        Err(anyhow::anyhow!("Missing 'uri' parameter"))
                    }
                } else {
                    Err(anyhow::anyhow!("Missing parameters"))
                }
            }
            "tools/list" => self
                .tool_handler
                .list_tools()
                .await
                .map(|t| json!({ "tools": t })),
            "tools/call" => {
                if let Some(params) = request.params {
                    if let Some(name) = params.get("name").and_then(|v| v.as_str()) {
                        let args = params.get("arguments").cloned().unwrap_or(json!({}));
                        self.tool_handler
                            .execute_tool(name, args)
                            .await
                            .map(|r| json!({ "content": [{"type": "text", "text": r.to_string()}] }))
                    } else {
                        Err(anyhow::anyhow!("Missing 'name' parameter"))
                    }
                } else {
                    Err(anyhow::anyhow!("Missing parameters"))
                }
            }
            "prompts/list" => self
                .prompt_handler
                .list_prompts()
                .await
                .map(|p| json!({ "prompts": p })),
            "prompts/get" => {
                if let Some(params) = request.params {
                    if let Some(name) = params.get("name").and_then(|v| v.as_str()) {
                        let args = params.get("arguments").cloned();
                        self.prompt_handler.get_prompt(name, args).await
                    } else {
                        Err(anyhow::anyhow!("Missing 'name' parameter"))
                    }
                } else {
                    Err(anyhow::anyhow!("Missing parameters"))
                }
            }
            _ => Err(anyhow::anyhow!("Method not found: {}", request.method)),
        };

        match result {
            Ok(res) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(res),
                error: None,
                id,
            },
            Err(e) => {
                let (code, message) = if e.to_string().starts_with("Method not found") {
                    (-32601, e.to_string())
                } else {
                    (-32603, e.to_string())
                };
                
                JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    result: None,
                    error: Some(JsonRpcError {
                        code,
                        message,
                        data: None,
                    }),
                    id,
                }
            }
        }
    }

    async fn handle_notification(&self, request: &JsonRpcRequest) {
        match request.method.as_str() {
            "notifications/initialized" => {
                info!("Client initialized");
            }
            "notifications/message" => {
                if let Some(params) = &request.params {
                    if let Ok(log_msg) = serde_json::from_value::<LogMessage>(params.clone()) {
                        self.logging_handler.handle_log(log_msg);
                    } else {
                        warn!("Invalid log message format");
                    }
                }
            }
            _ => {
                warn!("Unknown notification: {}", request.method);
            }
        }
    }

    async fn handle_initialize(&self, params: Option<Value>) -> anyhow::Result<Value> {
        let params = params.ok_or_else(|| anyhow::anyhow!("Missing parameters"))?;
        let request: InitializeRequest = serde_json::from_value(params)
            .map_err(|e| anyhow::anyhow!("Invalid initialize params: {}", e))?;

        info!(
            "Initializing with client: {} ({})",
            request.client_info.name, request.client_info.version
        );

        let result = InitializeResult {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ServerCapabilities {
                resources: Some(json!({})),
                tools: Some(json!({})),
                prompts: Some(json!({})),
                logging: Some(json!({})),
                experimental: None,
            },
            server_info: Implementation {
                name: "metis-mock-server".to_string(),
                version: "0.1.0".to_string(),
            },
        };

        Ok(serde_json::to_value(result)?)
    }
}
