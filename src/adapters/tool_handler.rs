use crate::adapters::mock_strategy::MockStrategyHandler;
use crate::config::ToolConfig;
use crate::domain::ToolPort;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct BasicToolHandler {
    tools: Arc<RwLock<HashMap<String, ToolConfig>>>,
    mock_strategy: Arc<MockStrategyHandler>,
}

impl BasicToolHandler {
    pub fn new(config: Vec<ToolConfig>, mock_strategy: Arc<MockStrategyHandler>) -> Self {
        let mut tools = HashMap::new();
        for tool in config {
            tools.insert(tool.name.clone(), tool);
        }
        Self {
            tools: Arc::new(RwLock::new(tools)),
            mock_strategy,
        }
    }
}

#[async_trait]
impl ToolPort for BasicToolHandler {
    async fn execute_tool(&self, name: &str, args: Value) -> anyhow::Result<Value> {
        let tools = self.tools.read().await;
        if let Some(tool) = tools.get(name) {
            if let Some(mock_config) = &tool.mock {
                return self.mock_strategy.generate(mock_config, Some(&args)).await;
            }
            
            if let Some(response) = &tool.static_response {
                Ok(response.clone())
            } else {
                // Fallback for echo or other built-ins if needed, or just return null/error
                if name == "echo" {
                     Ok(json!({ "result": args }))
                } else {
                    Ok(json!({ "status": "executed", "tool": name }))
                }
            }
        } else {
            Err(anyhow::anyhow!("Tool not found: {}", name))
        }
    }

    async fn list_tools(&self) -> anyhow::Result<Vec<Value>> {
        let tools = self.tools.read().await;
        Ok(tools.values().map(|t| {
            json!({
                "name": t.name,
                "description": t.description,
                "input_schema": t.input_schema
            })
        }).collect())
    }
}
