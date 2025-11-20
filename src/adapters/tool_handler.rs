use crate::adapters::mock_strategy::MockStrategyHandler;
use crate::config::{Settings, ToolConfig};
use crate::domain::{Tool, ToolPort};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct BasicToolHandler {
    settings: Arc<RwLock<Settings>>,
    mock_strategy: Arc<MockStrategyHandler>,
}

impl BasicToolHandler {
    pub fn new(settings: Arc<RwLock<Settings>>, mock_strategy: Arc<MockStrategyHandler>) -> Self {
        Self {
            settings,
            mock_strategy,
        }
    }

    async fn find_tool_config(&self, name: &str) -> Option<ToolConfig> {
        let settings = self.settings.read().await;
        settings.tools.iter().find(|t| t.name == name).cloned()
    }
}

#[async_trait]
impl ToolPort for BasicToolHandler {
    async fn list_tools(&self) -> Result<Vec<Tool>> {
        let settings = self.settings.read().await;
        let tools = settings
            .tools
            .iter()
            .map(|t| Tool {
                name: t.name.clone(),
                description: t.description.clone(),
                input_schema: t.input_schema.clone(),
            })
            .collect();
        Ok(tools)
    }

    async fn execute_tool(&self, name: &str, args: Value) -> Result<Value> {
        if let Some(config) = self.find_tool_config(name).await {
            if let Some(mock_config) = &config.mock {
                self.mock_strategy.generate(mock_config, Some(&args)).await
            } else if let Some(static_response) = &config.static_response {
                Ok(static_response.clone())
            } else {
                Ok(Value::Null)
            }
        } else {
            Err(anyhow::anyhow!("Tool not found: {}", name))
        }
    }

}
