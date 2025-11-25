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
            .map(|r| {
                // Build enhanced description with schemas embedded (MCP doesn't natively support schemas for resources)
                let mut description_parts = Vec::new();
                if let Some(desc) = &r.description {
                    description_parts.push(desc.clone());
                }
                if let Some(input_schema) = &r.input_schema {
                    description_parts.push(format!(
                        "\n\n**Input Schema:**\n```json\n{}\n```",
                        serde_json::to_string_pretty(input_schema).unwrap_or_default()
                    ));
                }
                if let Some(output_schema) = &r.output_schema {
                    description_parts.push(format!(
                        "\n\n**Output Schema:**\n```json\n{}\n```",
                        serde_json::to_string_pretty(output_schema).unwrap_or_default()
                    ));
                }
                let description = if description_parts.is_empty() {
                    None
                } else {
                    Some(description_parts.join(""))
                };

                Resource {
                    uri: r.uri.clone(),
                    name: r.name.clone(),
                    description,
                    mime_type: r.mime_type.clone(),
                }
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
