use crate::config::{PromptConfig, Settings};
use crate::domain::{GetPromptResult, Prompt, PromptArgument, PromptContent, PromptMessage, PromptPort};
use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct InMemoryPromptHandler {
    settings: Arc<RwLock<Settings>>,
}

impl InMemoryPromptHandler {
    pub fn new(settings: Arc<RwLock<Settings>>) -> Self {
        Self { settings }
    }

    async fn find_prompt_config(&self, name: &str) -> Option<PromptConfig> {
        let settings = self.settings.read().await;
        settings.prompts.iter().find(|p| p.name == name).cloned()
    }
}

#[async_trait]
impl PromptPort for InMemoryPromptHandler {
    async fn list_prompts(&self) -> Result<Vec<Prompt>> {
        let settings = self.settings.read().await;
        let prompts = settings
            .prompts
            .iter()
            .map(|p| {
                // Build enhanced description with input_schema embedded if present
                // MCP prompts have arguments but not full JSON schema support
                let description = if let Some(input_schema) = &p.input_schema {
                    format!(
                        "{}\n\n**Input Schema:**\n```json\n{}\n```",
                        p.description,
                        serde_json::to_string_pretty(input_schema).unwrap_or_default()
                    )
                } else {
                    p.description.clone()
                };

                Prompt {
                    name: p.name.clone(),
                    description,
                    arguments: p.arguments.as_ref().map(|args| {
                        args.iter()
                            .map(|a| PromptArgument {
                                name: a.name.clone(),
                                description: a.description.clone(),
                                required: a.required,
                            })
                            .collect()
                    }),
                }
            })
            .collect();
        Ok(prompts)
    }

    async fn get_prompt(&self, name: &str, _args: Option<Value>) -> Result<GetPromptResult> {
        if let Some(config) = self.find_prompt_config(name).await {
            let messages = config.messages.as_ref().map(|msgs| {
                msgs.iter()
                    .map(|m| PromptMessage {
                        role: m.role.clone(),
                        content: PromptContent {
                            type_: "text".to_string(),
                            text: m.content.clone(),
                        },
                    })
                    .collect()
            }).unwrap_or_default();

            Ok(GetPromptResult {
                description: Some(config.description.clone()),
                messages,
            })
        } else {
            Err(anyhow::anyhow!("Prompt not found: {}", name))
        }
    }
}
