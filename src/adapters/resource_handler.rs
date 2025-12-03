use crate::adapters::file_storage::FileStorageHandler;
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
    /// Optional file storage handler for data lake resources
    file_storage: Option<Arc<FileStorageHandler>>,
}

impl InMemoryResourceHandler {
    pub fn new(settings: Arc<RwLock<Settings>>, mock_strategy: Arc<MockStrategyHandler>) -> Self {
        Self {
            settings,
            mock_strategy,
            file_storage: None,
        }
    }

    /// Create a new handler with file storage support for data lake resources
    pub fn with_file_storage(
        settings: Arc<RwLock<Settings>>,
        mock_strategy: Arc<MockStrategyHandler>,
        file_storage: Arc<FileStorageHandler>,
    ) -> Self {
        Self {
            settings,
            mock_strategy,
            file_storage: Some(file_storage),
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

    /// Handle data lake resource URIs (datalake://{lake}/{schema})
    async fn get_data_lake_resource(&self, uri: &str) -> Result<crate::domain::ResourceReadResult> {
        let file_storage = self.file_storage.as_ref()
            .ok_or_else(|| anyhow::anyhow!("File storage not configured"))?;

        // Parse datalake://{lake}/{schema}
        let path = uri.strip_prefix("datalake://")
            .ok_or_else(|| anyhow::anyhow!("Invalid data lake URI"))?;

        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid data lake URI format. Expected: datalake://{{lake}}/{{schema}}"));
        }

        let lake_name = parts[0];
        let schema_name = parts[1];

        // Read all records from files
        let records = file_storage.read_all_records(lake_name, schema_name).await
            .map_err(|e| anyhow::anyhow!("Failed to read data lake records: {}", e))?;

        let content = serde_json::to_string_pretty(&records)?;

        Ok(crate::domain::ResourceReadResult {
            uri: uri.to_string(),
            mime_type: Some("application/json".to_string()),
            content,
        })
    }
}

#[async_trait]
impl ResourcePort for InMemoryResourceHandler {
    async fn list_resources(&self) -> Result<Vec<Resource>> {
        let settings = self.settings.read().await;
        let mut resources: Vec<Resource> = settings
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

        // Add data lake resources if file storage is enabled
        if self.file_storage.is_some() {
            for data_lake in &settings.data_lakes {
                if data_lake.uses_files() {
                    for schema_ref in &data_lake.schemas {
                        resources.push(Resource {
                            uri: format!("datalake://{}/{}", data_lake.name, schema_ref.schema_name),
                            name: format!("{}/{} Records", data_lake.name, schema_ref.alias.as_ref().unwrap_or(&schema_ref.schema_name)),
                            description: Some(format!(
                                "Data lake records for {} in {}. Format: {:?}",
                                schema_ref.schema_name, data_lake.name, data_lake.file_format
                            )),
                            mime_type: Some("application/json".to_string()),
                        });
                    }
                }
            }
        }

        Ok(resources)
    }

    async fn list_resource_templates(&self) -> Result<Vec<ResourceTemplate>> {
        let settings = self.settings.read().await;
        let mut templates: Vec<ResourceTemplate> = settings
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

        // Add SQL query resource template for data lakes with SQL enabled
        if self.file_storage.is_some() {
            for data_lake in &settings.data_lakes {
                if data_lake.enable_sql_queries {
                    templates.push(ResourceTemplate {
                        uri_template: format!("datalake://{}/query?sql={{sql}}&schema={{schema}}", data_lake.name),
                        name: format!("{} SQL Query", data_lake.name),
                        description: Some(format!(
                            "Execute SQL query against {} data lake. Use $table as placeholder for the table name.\n\n\
                            **Example:** SELECT * FROM $table WHERE id = '123'\n\n\
                            **Variables:**\n- sql: The SQL query to execute\n- schema: Schema name to query",
                            data_lake.name
                        )),
                        mime_type: Some("application/json".to_string()),
                    });
                }
            }
        }

        Ok(templates)
    }

    async fn get_resource(&self, uri: &str) -> Result<crate::domain::ResourceReadResult> {
        // Handle datalake:// URIs
        if uri.starts_with("datalake://") {
            return self.get_data_lake_resource(uri).await;
        }

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
