use crate::adapters::mock_strategy::MockStrategyHandler;
use crate::config::{ResourceConfig, ResourceTemplateConfig, Settings};
use crate::domain::{Resource, ResourcePort, ResourceTemplate};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
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

    async fn find_resource_template_config(&self, uri_template: &str) -> Option<ResourceTemplateConfig> {
        let settings = self.settings.read().await;
        settings
            .resource_templates
            .iter()
            .find(|r| r.uri_template == uri_template)
            .cloned()
    }

    /// Resolve a URI template by substituting {variable} placeholders with argument values
    fn resolve_uri_template(uri_template: &str, args: Option<&Value>) -> String {
        let mut resolved = uri_template.to_string();
        if let Some(args_val) = args {
            if let Some(obj) = args_val.as_object() {
                for (key, value) in obj {
                    let placeholder = format!("{{{}}}", key);
                    let replacement = match value {
                        Value::String(s) => s.clone(),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        _ => value.to_string(),
                    };
                    resolved = resolved.replace(&placeholder, &replacement);
                }
            }
        }
        resolved
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
                // Build enhanced description with output schema embedded (MCP doesn't natively support schemas for resources)
                let mut description_parts = Vec::new();
                if let Some(desc) = &r.description {
                    description_parts.push(desc.clone());
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

    async fn list_resource_templates(&self) -> Result<Vec<ResourceTemplate>> {
        let settings = self.settings.read().await;
        let templates = settings
            .resource_templates
            .iter()
            .map(|r| {
                // Build enhanced description with schemas embedded
                let mut description_parts = Vec::new();
                if let Some(desc) = &r.description {
                    description_parts.push(desc.clone());
                }
                if let Some(input_schema) = &r.input_schema {
                    description_parts.push(format!(
                        "\n\n**Input Schema (URI Variables):**\n```json\n{}\n```",
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

                ResourceTemplate {
                    uri_template: r.uri_template.clone(),
                    name: r.name.clone(),
                    description,
                    mime_type: r.mime_type.clone(),
                }
            })
            .collect();
        Ok(templates)
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

    async fn read_resource_template(
        &self,
        uri_template: &str,
        args: Option<&Value>,
    ) -> Result<crate::domain::ResourceReadResult> {
        if let Some(config) = self.find_resource_template_config(uri_template).await {
            // Resolve the URI template with the provided arguments
            let resolved_uri = Self::resolve_uri_template(&config.uri_template, args);

            let content = if let Some(mock_config) = &config.mock {
                let result = self.mock_strategy.generate(mock_config, args).await?;
                if let Some(s) = result.as_str() {
                    s.to_string()
                } else {
                    result.to_string()
                }
            } else if let Some(c) = &config.content {
                // Also resolve template variables in static content
                Self::resolve_uri_template(c, args)
            } else {
                "".to_string()
            };

            Ok(crate::domain::ResourceReadResult {
                uri: resolved_uri,
                mime_type: config.mime_type.clone(),
                content,
            })
        } else {
            Err(anyhow::anyhow!("Resource template not found: {}", uri_template))
        }
    }
}
