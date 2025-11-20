use crate::adapters::mock_strategy::MockStrategyHandler;
use crate::config::ResourceConfig;
use crate::domain::ResourcePort;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct InMemoryResourceHandler {
    resources: Arc<RwLock<HashMap<String, ResourceConfig>>>,
    mock_strategy: Arc<MockStrategyHandler>,
}

impl InMemoryResourceHandler {
    pub fn new(config: Vec<ResourceConfig>, mock_strategy: Arc<MockStrategyHandler>) -> Self {
        let mut resources = HashMap::new();
        
        for res in config {
            resources.insert(res.uri.clone(), res);
        }

        Self {
            resources: Arc::new(RwLock::new(resources)),
            mock_strategy,
        }
    }
}

#[async_trait]
impl ResourcePort for InMemoryResourceHandler {
    async fn get_resource(&self, uri: &str) -> anyhow::Result<Value> {
        let resources = self.resources.read().await;
        if let Some(config) = resources.get(uri) {
            // Generate content using mock strategy or static content
            let content = if let Some(mock_config) = &config.mock {
                // Use mock strategy to generate content
                self.mock_strategy.generate(mock_config, None).await?
            } else {
                // If content is a string, wrap it in Value::String, otherwise Null
                config.content.clone().map(Value::String).unwrap_or(Value::Null)
            };

            Ok(json!({
                "uri": config.uri,
                "name": config.name,
                "description": config.description,
                "mimeType": config.mime_type,
                "text": content.as_str().unwrap_or(""),
            }))
        } else {
            Err(anyhow::anyhow!("Resource not found: {}", uri))
        }
    }

    async fn list_resources(&self) -> anyhow::Result<Vec<Value>> {
        let resources = self.resources.read().await;
        let list = resources
            .values()
            .map(|config| {
                json!({
                    "uri": config.uri,
                    "name": config.name,
                    "description": config.description,
                    "mimeType": config.mime_type,
                })
            })
            .collect();
        Ok(list)
    }
}
