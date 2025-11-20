use crate::config::PromptConfig;
use crate::domain::PromptPort;
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct InMemoryPromptHandler {
    prompts: Arc<RwLock<HashMap<String, PromptConfig>>>,
}

impl InMemoryPromptHandler {
    pub fn new(config: Vec<PromptConfig>) -> Self {
        let mut prompts = HashMap::new();
        for prompt in config {
            prompts.insert(prompt.name.clone(), prompt);
        }
        Self {
            prompts: Arc::new(RwLock::new(prompts)),
        }
    }
}

#[async_trait]
impl PromptPort for InMemoryPromptHandler {
    async fn get_prompt(&self, name: &str, _arguments: Option<Value>) -> anyhow::Result<Value> {
        let prompts = self.prompts.read().await;
        if let Some(prompt) = prompts.get(name) {
            if let Some(messages) = &prompt.messages {
                Ok(json!({
                    "messages": messages.iter().map(|m| {
                        json!({
                            "role": m.role,
                            "content": m.content
                        })
                    }).collect::<Vec<_>>()
                }))
            } else {
                // Fallback or error if no messages defined
                Err(anyhow::anyhow!("No messages defined for prompt: {}", name))
            }
        } else {
            Err(anyhow::anyhow!("Prompt not found: {}", name))
        }
    }

    async fn list_prompts(&self) -> anyhow::Result<Vec<Value>> {
        let prompts = self.prompts.read().await;
        Ok(prompts.values().map(|p| {
            json!({
                "name": p.name,
                "description": p.description,
                "arguments": p.arguments.as_ref().map(|args| {
                    args.iter().map(|a| {
                        json!({
                            "name": a.name,
                            "description": a.description,
                            "required": a.required
                        })
                    }).collect::<Vec<_>>()
                })
            })
        }).collect())
    }
}
