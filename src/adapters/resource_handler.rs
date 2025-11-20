use crate::adapters::mock_strategy::MockStrategyHandler;
use crate::config::{ResourceConfig, Settings};
use crate::domain::{Resource, ResourcePort};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct InMemoryResourceHandler {
    settings: Arc<RwLock<Settings>>,
    mock_strategy: Arc<MockStrategyHandler>,
}

impl InMemoryResourceHandler {
    pub fn new(settings: Arc<RwLock<Settings>>, mock_strategy: Arc<MockStrategyHandler>) -> Self {
        Self {
            settings,
            mock_strategy,
        }
    }

    async fn find_resource_config(&self, uri: &str) -> Option<ResourceConfig> {
        let settings = self.settings.read().await;
        settings.resources.iter().find(|r| r.uri == uri).cloned()
    }
}

#[async_trait]
impl ResourcePort for InMemoryResourceHandler {
    async fn list_resources(&self) -> Result<Vec<Resource>> {
        let settings = self.settings.read().await;
        let resources = settings
            .resources
            .iter()
            .map(|r| Resource {
                uri: r.uri.clone(),
                name: r.name.clone(),
                description: r.description.clone(),
                mime_type: r.mime_type.clone(),
            })
            .collect();
        Ok(resources)
    }

    async fn get_resource(&self, uri: &str) -> Result<crate::domain::ResourceReadResult> {
        if let Some(config) = self.find_resource_config(uri).await {
            let content = if let Some(mock_config) = &config.mock {
                let result = self.mock_strategy.generate(mock_config, None).await?;
                if let Some(s) = result.as_str() {
                    s.to_string()
                } else {
                    result.to_string()
                }
            } else if let Some(c) = &config.content {
                c.clone()
            } else {
                "".to_string()
            };

            Ok(crate::domain::ResourceReadResult {
                uri: config.uri.clone(),
                mime_type: config.mime_type.clone(),
                content,
            })
        } else {
            Err(anyhow::anyhow!("Resource not found: {}", uri))
        }
    }
}
