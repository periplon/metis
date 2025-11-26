use std::collections::HashMap;
use thiserror::Error;

use crate::config::{ResourceConfig, Settings, ToolConfig};

#[derive(Debug, Error)]
pub enum ValidationError {
    #[error("Missing required field: {0}")]
    MissingField(String),
    
    #[error("Invalid value for {field}: {reason}")]
    InvalidValue { field: String, reason: String },
    
    #[error("Cross-reference error: {0}")]
    CrossReference(String),
    
    #[error("Duplicate entry: {0}")]
    Duplicate(String),
}

pub struct ConfigValidator;

impl ConfigValidator {
    pub fn validate(settings: &Settings) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        // Validate server settings
        if let Err(e) = Self::validate_server(&settings.server) {
            errors.extend(e);
        }

        // Validate resources
        if let Err(e) = Self::validate_resources(&settings.resources) {
            errors.extend(e);
        }

        // Validate tools
        if let Err(e) = Self::validate_tools(&settings.tools) {
            errors.extend(e);
        }

        // Validate prompts
        if let Err(e) = Self::validate_prompts(&settings.prompts) {
            errors.extend(e);
        }

        // Cross-reference validation
        if let Err(e) = Self::validate_cross_references(settings) {
            errors.extend(e);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn validate_server(server: &crate::config::ServerSettings) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        if server.host.is_empty() {
            errors.push(ValidationError::MissingField("server.host".to_string()));
        }

        if server.port == 0 {
            errors.push(ValidationError::InvalidValue {
                field: "server.port".to_string(),
                reason: "Port must be greater than 0".to_string(),
            });
        }

        // Note: u16 max is 65535, so no need to check upper bound

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn validate_resources(resources: &[ResourceConfig]) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();
        let mut seen_uris = HashMap::new();

        for (idx, resource) in resources.iter().enumerate() {
            // Check for duplicate URIs
            if let Some(prev_idx) = seen_uris.insert(&resource.uri, idx) {
                errors.push(ValidationError::Duplicate(
                    format!("Resource URI '{}' appears at indices {} and {}", resource.uri, prev_idx, idx)
                ));
            }

            // Validate required fields
            if resource.uri.is_empty() {
                errors.push(ValidationError::MissingField(
                    format!("resources[{}].uri", idx)
                ));
            }

            if resource.name.is_empty() {
                errors.push(ValidationError::MissingField(
                    format!("resources[{}].name", idx)
                ));
            }

            // Validate that either content or mock is provided
            if resource.content.is_none() && resource.mock.is_none() {
                errors.push(ValidationError::InvalidValue {
                    field: format!("resources[{}]", idx),
                    reason: "Either 'content' or 'mock' must be provided".to_string(),
                });
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn validate_tools(tools: &[ToolConfig]) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();
        let mut seen_names = HashMap::new();

        for (idx, tool) in tools.iter().enumerate() {
            // Check for duplicate names
            if let Some(prev_idx) = seen_names.insert(&tool.name, idx) {
                errors.push(ValidationError::Duplicate(
                    format!("Tool name '{}' appears at indices {} and {}", tool.name, prev_idx, idx)
                ));
            }

            // Validate required fields
            if tool.name.is_empty() {
                errors.push(ValidationError::MissingField(
                    format!("tools[{}].name", idx)
                ));
            }

            if tool.description.is_empty() {
                errors.push(ValidationError::MissingField(
                    format!("tools[{}].description", idx)
                ));
            }

            // Validate that either static_response or mock is provided
            if tool.static_response.is_none() && tool.mock.is_none() {
                errors.push(ValidationError::InvalidValue {
                    field: format!("tools[{}]", idx),
                    reason: "Either 'static_response' or 'mock' must be provided".to_string(),
                });
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn validate_prompts(prompts: &[crate::config::PromptConfig]) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();
        let mut seen_names = HashMap::new();

        for (idx, prompt) in prompts.iter().enumerate() {
            // Check for duplicate names
            if let Some(prev_idx) = seen_names.insert(&prompt.name, idx) {
                errors.push(ValidationError::Duplicate(
                    format!("Prompt name '{}' appears at indices {} and {}", prompt.name, prev_idx, idx)
                ));
            }

            // Validate required fields
            if prompt.name.is_empty() {
                errors.push(ValidationError::MissingField(
                    format!("prompts[{}].name", idx)
                ));
            }

            if prompt.description.is_empty() {
                errors.push(ValidationError::MissingField(
                    format!("prompts[{}].description", idx)
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn validate_cross_references(_settings: &Settings) -> Result<(), Vec<ValidationError>> {
        let errors = Vec::new();

        // TODO: Add cross-reference validation
        // For example: validate that tool references to resources exist

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ServerSettings, Settings};

    #[test]
    fn test_valid_config() {
        let settings = Settings {
            server: ServerSettings {
                host: "127.0.0.1".to_string(),
                port: 3000,
            },
            auth: Default::default(),
            resources: vec![],
            resource_templates: vec![],
            tools: vec![],
            prompts: vec![],
            rate_limit: None,
            s3: None,
            workflows: vec![],
            agents: vec![],
            orchestrations: vec![],
            mcp_servers: vec![],
            secrets: Default::default(),
        };

        let result = ConfigValidator::validate(&settings);
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_port() {
        let settings = Settings {
            server: ServerSettings {
                host: "127.0.0.1".to_string(),
                port: 0,
            },
            auth: Default::default(),
            resources: vec![],
            resource_templates: vec![],
            tools: vec![],
            prompts: vec![],
            rate_limit: None,
            s3: None,
            workflows: vec![],
            agents: vec![],
            orchestrations: vec![],
            mcp_servers: vec![],
            secrets: Default::default(),
        };

        let result = ConfigValidator::validate(&settings);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn test_duplicate_resource_uris() {
        let settings = Settings {
            server: ServerSettings {
                host: "127.0.0.1".to_string(),
                port: 3000,
            },
            auth: Default::default(),
            resources: vec![
                ResourceConfig {
                    uri: "test://same".to_string(),
                    name: "First".to_string(),
                    description: None,
                    mime_type: None,
                    output_schema: None,
                    content: Some("content".to_string()),
                    mock: None,
                },
                ResourceConfig {
                    uri: "test://same".to_string(),
                    name: "Second".to_string(),
                    description: None,
                    mime_type: None,
                    output_schema: None,
                    content: Some("content".to_string()),
                    mock: None,
                },
            ],
            resource_templates: vec![],
            tools: vec![],
            prompts: vec![],
            rate_limit: None,
            s3: None,
            workflows: vec![],
            agents: vec![],
            orchestrations: vec![],
            mcp_servers: vec![],
            secrets: Default::default(),
        };

        let result = ConfigValidator::validate(&settings);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| matches!(e, ValidationError::Duplicate(_))));
    }
}