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

    /// Try to match a resolved URI against resource templates and extract arguments
    /// Returns the matched template config and extracted arguments as JSON
    async fn match_uri_to_template(&self, uri: &str) -> Option<(ResourceTemplateConfig, Value)> {
        let settings = self.settings.read().await;

        for template in &settings.resource_templates {
            if let Some(args) = Self::extract_template_args(&template.uri_template, uri) {
                return Some((template.clone(), args));
            }
        }

        None
    }

    /// Extract arguments from a URI by matching against a template pattern
    /// Template: "file://countries/{country_code}/info"
    /// URI: "file://countries/us/info"
    /// Returns: {"country_code": "us"}
    pub(crate) fn extract_template_args(template: &str, uri: &str) -> Option<Value> {
        // Parse template into segments (literal parts and placeholders)
        let mut template_parts: Vec<TemplatePart> = Vec::new();
        let mut current_pos = 0;

        while current_pos < template.len() {
            if let Some(start) = template[current_pos..].find('{') {
                let abs_start = current_pos + start;
                // Add literal part before placeholder
                if abs_start > current_pos {
                    template_parts.push(TemplatePart::Literal(
                        template[current_pos..abs_start].to_string(),
                    ));
                }
                // Find end of placeholder
                if let Some(end) = template[abs_start..].find('}') {
                    let abs_end = abs_start + end;
                    let placeholder_name = &template[abs_start + 1..abs_end];
                    template_parts.push(TemplatePart::Placeholder(placeholder_name.to_string()));
                    current_pos = abs_end + 1;
                } else {
                    // Malformed template - unclosed brace
                    return None;
                }
            } else {
                // Rest is literal
                template_parts.push(TemplatePart::Literal(template[current_pos..].to_string()));
                break;
            }
        }

        // Now try to match URI against template parts
        let mut uri_pos = 0;
        let mut args = serde_json::Map::new();

        for (i, part) in template_parts.iter().enumerate() {
            match part {
                TemplatePart::Literal(lit) => {
                    // URI must have this literal at current position
                    if !uri[uri_pos..].starts_with(lit) {
                        return None;
                    }
                    uri_pos += lit.len();
                }
                TemplatePart::Placeholder(name) => {
                    // Find next literal to know where placeholder value ends
                    let end_pos = if i + 1 < template_parts.len() {
                        if let TemplatePart::Literal(next_lit) = &template_parts[i + 1] {
                            // Find next literal in URI
                            uri[uri_pos..].find(next_lit.as_str()).map(|p| uri_pos + p)
                        } else {
                            // Next is another placeholder - match until /
                            uri[uri_pos..].find('/').map(|p| uri_pos + p)
                        }
                    } else {
                        // Last part - take rest of URI
                        Some(uri.len())
                    };

                    let end = end_pos?;
                    if end <= uri_pos {
                        return None; // Empty placeholder value
                    }

                    let value = &uri[uri_pos..end];
                    args.insert(name.clone(), Value::String(value.to_string()));
                    uri_pos = end;
                }
            }
        }

        // URI must be fully consumed
        if uri_pos != uri.len() {
            return None;
        }

        Some(Value::Object(args))
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

/// Helper enum for template parsing
enum TemplatePart {
    Literal(String),
    Placeholder(String),
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
        tracing::debug!("get_resource called with uri: {}", uri);

        // Handle datalake:// URIs
        if uri.starts_with("datalake://") {
            return self.get_data_lake_resource(uri).await;
        }

        if let Some(config) = self.find_resource_config(uri).await {
            let content = if let Some(mock_config) = &config.mock {
                let result = self.mock_strategy.generate(mock_config, None).await?;

                // Log result type and size for debugging
                let (content_str, array_len) = if let Some(arr) = result.as_array() {
                    let s = result.to_string();
                    tracing::debug!(
                        "Resource {} mock result: JSON array with {} elements, {} bytes",
                        uri,
                        arr.len(),
                        s.len()
                    );
                    (s, Some(arr.len()))
                } else if let Some(s) = result.as_str() {
                    tracing::debug!(
                        "Resource {} mock result: string, {} bytes",
                        uri,
                        s.len()
                    );
                    (s.to_string(), None)
                } else {
                    let s = result.to_string();
                    let type_str = match &result {
                        serde_json::Value::Null => "null",
                        serde_json::Value::Bool(_) => "boolean",
                        serde_json::Value::Number(_) => "number",
                        serde_json::Value::Object(_) => "object",
                        _ => "other",
                    };
                    tracing::debug!(
                        "Resource {} mock result: {}, {} bytes",
                        uri,
                        type_str,
                        s.len()
                    );
                    (s, None)
                };

                // Verify array wasn't truncated during serialization
                if let Some(len) = array_len {
                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content_str) {
                        if let Some(parsed_arr) = parsed.as_array() {
                            if parsed_arr.len() != len {
                                tracing::warn!(
                                    "Resource {} array length mismatch: original {} vs serialized {}",
                                    uri, len, parsed_arr.len()
                                );
                            }
                        }
                    }
                }

                content_str
            } else if let Some(c) = &config.content {
                c.clone()
            } else {
                "".to_string()
            };

            tracing::debug!("Resource {} returning {} bytes of content", uri, content.len());

            Ok(crate::domain::ResourceReadResult {
                uri: config.uri.clone(),
                mime_type: config.mime_type.clone(),
                content,
            })
        } else if let Some((template_config, args)) = self.match_uri_to_template(uri).await {
            // URI matches a resource template - execute it with extracted arguments
            tracing::debug!(
                "URI {} matched template {}, args: {}",
                uri,
                template_config.uri_template,
                args
            );

            let content = if let Some(mock_config) = &template_config.mock {
                let result = self.mock_strategy.generate(mock_config, Some(&args)).await?;
                if let Some(s) = result.as_str() {
                    s.to_string()
                } else {
                    result.to_string()
                }
            } else if let Some(c) = &template_config.content {
                // Also resolve template variables in static content
                Self::resolve_uri_template(c, Some(&args))
            } else {
                "".to_string()
            };

            tracing::debug!("Resource template {} returning {} bytes of content", uri, content.len());

            Ok(crate::domain::ResourceReadResult {
                uri: uri.to_string(),
                mime_type: template_config.mime_type.clone(),
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
